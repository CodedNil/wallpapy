use crate::common::{
    CommentData, GetWallpapersResponse, LikedState, TokenPacket, TokenUuidLikedPacket,
    TokenUuidPacket, WallpaperData,
};
use crate::server::{auth::verify_token, prompt, COMMENTS_TREE, DATABASE_PATH, IMAGES_TREE};
use anyhow::{anyhow, Result};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use image::{DynamicImage, GenericImageView, ImageReader};
use serde_json::json;
use std::env;
use std::path::Path;
use thumbhash::rgba_to_thumb_hash;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
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

    // Remove the database entry
    let result = (|| -> Result<()> {
        sled::open(DATABASE_PATH)?
            .open_tree(IMAGES_TREE)?
            .remove(packet.uuid)?;
        Ok(())
    })();

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored remove_comment {:?}", e);
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

async fn generate_wallpaper_impl() -> Result<()> {
    log::info!("Generating wallpaper");

    let id = Uuid::new_v4();
    let datetime = OffsetDateTime::now_utc();

    // Generate image
    let prompt = prompt::generate().await?;
    let image = image_diffusion(&prompt).await?;
    let (width, height) = image.dimensions();

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
    let file_name = format!("{}.jpg", &datetime.format(&Rfc3339)?);
    image.save(dir.join(&file_name))?;

    let image_data = WallpaperData {
        id,
        datetime,
        prompt,
        file_name,
        width,
        height,
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

async fn image_diffusion(prompt: &str) -> Result<DynamicImage> {
    let client = reqwest::Client::new();

    let api_token =
        env::var("REPLICATE_API_TOKEN").expect("REPLICATE_API_TOKEN environment variable not set");
    let response = client
        .post("https://api.replicate.com/v1/models/black-forest-labs/flux-schnell/predictions")
        .header("Authorization", format!("Bearer {api_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "input": {
                "prompt": prompt,
                "num_outputs": 1,
                "aspect_ratio": "16:9",
                "output_format": "png",
                "output_quality": 80
            }
        }))
        .send()
        .await?;

    let response_json = response.json::<serde_json::Value>().await?;
    let status_url = response_json["urls"]["get"]
        .as_str()
        .ok_or_else(|| anyhow!("No valid status URL found"))?;

    let mut image_url = None;
    for _ in 0..TIMEOUT {
        let status_response = client
            .get(status_url)
            .header("Authorization", format!("Bearer {api_token}"))
            .header("Content-Type", "application/json")
            .send()
            .await?;
        let status_json = status_response.json::<serde_json::Value>().await?;

        if let Some(output) = status_json["output"].as_array() {
            if let Some(url) = output.first().and_then(|v| v.as_str()) {
                image_url = Some(url.to_string());
                break;
            }
        }

        if status_json["status"] == "succeeded" {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    let image_url = image_url.ok_or_else(|| anyhow!("Image generation timed out or failed"))?;
    let img_data = client.get(&image_url).send().await?.bytes().await?;

    let img = ImageReader::new(std::io::Cursor::new(img_data))
        .with_guessed_format()?
        .decode()?;

    Ok(img)
}
