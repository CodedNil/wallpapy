use crate::common::{
    CommentData, GetWallpapersResponse, ImageFile, LikedState, TokenPacket, TokenUuidLikedPacket,
    TokenUuidPacket, UpscaleState, WallpaperData, WallpaperImageType,
};
use crate::server::{auth::verify_token, prompt, COMMENTS_TREE, DATABASE_PATH, IMAGES_TREE};
use anyhow::{anyhow, Result};
use axum::http::{HeaderMap, HeaderValue};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use image::{imageops, DynamicImage, ExtendedColorType, ImageReader};
use reqwest::Client;
use serde_json::json;
use std::{env, path::Path, time::Duration};
use thumbhash::rgba_to_thumb_hash;
use time::{
    format_description::{self, well_known::Rfc3339},
    OffsetDateTime,
};
use tokio::fs;
use uuid::Uuid;

const TIMEOUT: u64 = 40;

pub async fn generate(packet: Bytes) -> impl IntoResponse {
    let packet: TokenPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize generate_wallpaper packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !matches!(verify_token(&packet.token), Ok(true)) {
        log::error!("Unauthorized generate_wallpaper request");
        return StatusCode::UNAUTHORIZED;
    }

    match generate_wallpaper_impl().await {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Failed to generate wallpaper: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn get() -> impl IntoResponse {
    match sled::open(DATABASE_PATH)
        .and_then(|db| Ok((db.clone(), db.open_tree(IMAGES_TREE)?)))
        .and_then(|(db, images_tree)| Ok((images_tree, db.open_tree(COMMENTS_TREE)?)))
    {
        Ok((images_tree, comments_tree)) => {
            let images: Vec<WallpaperData> = images_tree
                .iter()
                .values()
                .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
                .collect();
            let comments: Vec<CommentData> = comments_tree
                .iter()
                .values()
                .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
                .collect();

            match bincode::serialize(&GetWallpapersResponse { images, comments }) {
                Ok(data) => return (StatusCode::OK, data).into_response(),
                Err(e) => log::error!("{:?}", e),
            }
        }
        Err(e) => log::error!("{:?}", e),
    };

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

pub async fn latest() -> impl IntoResponse {
    match sled::open(DATABASE_PATH).and_then(|db| db.open_tree(IMAGES_TREE)) {
        Ok(images_tree) => {
            if let Some(file_name) = images_tree
                .iter()
                .values()
                .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
                .max_by_key(|wallpaper: &WallpaperData| wallpaper.datetime)
                .map(|image| {
                    image.upscaled_file.as_ref().map_or_else(
                        || image.original_file.file_name.clone(),
                        |upscaled_file| upscaled_file.file_name.clone(),
                    )
                })
            {
                let image_path = Path::new("wallpapers").join(&file_name);
                if let Ok(data) = std::fs::read(&image_path) {
                    let mime_type = mime_guess::from_path(&image_path).first_or_octet_stream();
                    let mut headers = HeaderMap::new();
                    headers.insert(
                        "Content-Type",
                        HeaderValue::from_str(mime_type.as_ref()).unwrap(),
                    );

                    return (StatusCode::OK, headers, data).into_response();
                }
            }
        }
        Err(e) => log::error!("{:?}", e),
    };

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

pub async fn remove(packet: Bytes) -> impl IntoResponse {
    let packet: TokenUuidPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize remove_comment packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !matches!(verify_token(&packet.token), Ok(true)) {
        log::error!("Unauthorized remove_comment request");
        return StatusCode::UNAUTHORIZED;
    }

    match Box::pin(remove_wallpaper_impl(packet)).await {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored remove_image {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn like(packet: Bytes) -> impl IntoResponse {
    let packet: TokenUuidLikedPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize upvote_image packet: {:?}", e);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    if !matches!(verify_token(&packet.token), Ok(true)) {
        log::error!("Unauthorized upvote_image request");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Set the vote state
    let result = (|| -> Result<()> {
        let tree = sled::open(DATABASE_PATH)?.open_tree(IMAGES_TREE)?;

        let mut wallpaper_data: WallpaperData = bincode::deserialize(
            &tree
                .get(packet.uuid)?
                .ok_or_else(|| anyhow::anyhow!("Image not found"))?,
        )?;
        wallpaper_data.vote_state = packet.liked;
        tree.insert(packet.uuid, bincode::serialize(&wallpaper_data)?)?;

        Ok(())
    })();

    match result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            log::error!("Failed to upvote image: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn generate_wallpaper_impl() -> Result<()> {
    log::info!("Generating wallpaper");

    let id = Uuid::new_v4();
    let datetime = OffsetDateTime::now_utc();
    let format = format_description::parse("[day]/[month]/[year] [hour]:[minute]")?;
    let datetime_text = datetime.format(&format)?;

    // Generate image prompt
    let prompt = prompt::generate().await?;
    log::info!("Generated prompt: {}", prompt);

    // Generate image
    let client = Client::new();
    let api_token =
        env::var("REPLICATE_API_TOKEN").expect("REPLICATE_API_TOKEN environment variable not set");
    let (image_url, image) = image_diffusion(&client, &api_token, &prompt).await?;
    log::info!("Generated image: {}", &image_url);

    // Upscale the image using Real-ESRGAN
    let (upscaled_url, upscaled_image) = upscale_image(&client, &api_token, &image_url).await?;
    log::info!("Upscaled image: {}", &upscaled_url);
    // If upscaled image is larger than 3840x2160 downscale it
    let upscaled_image = if upscaled_image.width() > 4096 || upscaled_image.height() > 4096 {
        upscaled_image.resize_to_fill(3820, 2160, imageops::FilterType::Lanczos3)
    } else {
        upscaled_image
    };

    // Resize the image to thumbnail
    let thumbnail = image.thumbnail(32, 32);
    let thumbhash = rgba_to_thumb_hash(
        thumbnail.width() as usize,
        thumbnail.height() as usize,
        thumbnail.into_rgba8().as_raw(),
    );

    // Save to file
    let dir = Path::new("wallpapers");
    fs::create_dir_all(dir).await?;

    let datetime_str = datetime.format(&Rfc3339)?;

    // Save the original image
    let file_name = format!("{datetime_str}.webp");
    {
        let file_writer = std::fs::File::create(dir.join(&file_name))?;
        let encoder = image::codecs::webp::WebPEncoder::new_lossless(file_writer);
        encoder.encode(
            &image.to_rgba8(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )?;
    }
    let original_file = ImageFile {
        file_name,
        width: image.width(),
        height: image.height(),
    };

    // Save the upscaled image
    let upscaled_file_name = format!("{datetime_str}_upscaled.webp");
    {
        let encoder = webp::Encoder::from_image(&upscaled_image).unwrap();
        let webp = encoder.encode(90.0);
        std::fs::write(dir.join(&upscaled_file_name), &*webp)?;
    }
    let upscaled_file = Some(ImageFile {
        file_name: upscaled_file_name,
        width: upscaled_image.width(),
        height: upscaled_image.height(),
    });

    let image_data = WallpaperData {
        id,
        image_type: WallpaperImageType::Desktop16x9,
        datetime,
        datetime_text,
        prompt,
        original_file,
        upscaled_file,
        upscale_state: UpscaleState::Basic,
        thumbhash,
        vote_state: LikedState::None,
    };

    // Store a new database entry
    sled::open(DATABASE_PATH)
        .map_err(|e| anyhow!("Failed to open database: {:?}", e))?
        .open_tree(IMAGES_TREE)
        .map_err(|e| anyhow!("Failed to open tree: {:?}", e))?
        .insert(id, bincode::serialize(&image_data)?)
        .map_err(|e| anyhow!("Failed to insert into tree: {:?}", e))?;

    Ok(())
}

async fn remove_wallpaper_impl(packet: TokenUuidPacket) -> Result<()> {
    // Open the database and find the entry
    let db = sled::open(DATABASE_PATH)?;
    let tree = db.open_tree(IMAGES_TREE)?;

    // Retrieve the existing entry
    if let Some(data) = tree.get(packet.uuid)? {
        let wallpaper_data: WallpaperData = bincode::deserialize(&data)?;

        // Construct the file path and remove the file
        let dir = Path::new("wallpapers");
        let file_path = dir.join(&wallpaper_data.original_file.file_name);
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
        if let Some(upscaled_file) = wallpaper_data.upscaled_file {
            let upscaled_file = dir.join(&upscaled_file.file_name);
            if upscaled_file.exists() {
                fs::remove_file(upscaled_file).await?;
            }
        }

        // Remove the database entry
        tree.remove(packet.uuid)?;
    } else {
        return Err(anyhow::anyhow!("No entry found for UUID"));
    }

    Ok(())
}

/// <https://replicate.com/black-forest-labs/flux-schnell>
async fn image_diffusion(
    client: &Client,
    api_token: &str,
    prompt: &str,
) -> Result<(String, DynamicImage)> {
    let status_url = replicate_request_prediction(
        client,
        api_token,
        "black-forest-labs/flux-schnell",
        &json!({
            "input": {
                "prompt": prompt,
                "num_outputs": 1,
                "aspect_ratio": "16:9",
                "output_format": "png",
                "output_quality": 80
            }
        }),
    )
    .await?;
    let image_url = replicate_poll_status(client, api_token, &status_url).await?;

    let img_data = client.get(&image_url).send().await?.bytes().await?;
    let img = ImageReader::new(std::io::Cursor::new(img_data))
        .with_guessed_format()?
        .decode()?;

    Ok((image_url, img))
}

/// <https://replicate.com/nightmareai/real-esrgan>
async fn upscale_image(
    client: &Client,
    api_token: &str,
    image_url: &str,
) -> Result<(String, DynamicImage)> {
    let status_url = replicate_request_prediction(
        client,
        api_token,
        "",
        &json!({
            "version": "f121d640bd286e1fdc67f9799164c1d5be36ff74576ee11c803ae5b665dd46aa",
            "input": {
                "image": image_url,
                "scale": 4,
                "face_enhance": false
            }
        }),
    )
    .await?;
    let image_url = replicate_poll_status(client, api_token, &status_url).await?;

    let img_data = client.get(&image_url).send().await?.bytes().await?;
    let img = ImageReader::new(std::io::Cursor::new(img_data))
        .with_guessed_format()?
        .decode()?;

    Ok((image_url, img))
}

async fn replicate_request_prediction(
    client: &Client,
    api_token: &str,
    model: &str,
    input_json: &serde_json::Value,
) -> Result<String> {
    let url = if model.is_empty() {
        "https://api.replicate.com/v1/predictions"
    } else {
        &format!("https://api.replicate.com/v1/models/{model}/predictions")
    };
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {api_token}"))
        .header("Content-Type", "application/json")
        .json(input_json)
        .send()
        .await?;

    let response_json = response.json::<serde_json::Value>().await?;
    let status_url = response_json["urls"]["get"]
        .as_str()
        .ok_or_else(|| anyhow!("No valid status URL found"))?
        .to_string();

    Ok(status_url)
}

async fn replicate_poll_status(
    client: &Client,
    api_token: &str,
    status_url: &str,
) -> Result<String> {
    for _ in 0..TIMEOUT {
        let status_response = client
            .get(status_url)
            .header("Authorization", format!("Bearer {api_token}"))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status_json = status_response.json::<serde_json::Value>().await?;

        if status_json["status"] == "succeeded" {
            if let Some(url) = status_json["output"].as_str() {
                return Ok(url.to_string());
            }
            if let Some(url) = status_json["output"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
            {
                return Ok(url.to_string());
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Err(anyhow!("Operation timed out or failed"))
}
