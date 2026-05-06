use crate::{
    common::{GenerationStage, ImageFile, LikedState, PromptData, WallpaperData},
    database::{WALLPAPERS_DIR, read_database, write_database},
    gpt, routing,
};
use anyhow::{Result, anyhow};
use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use chrono::{Timelike, Utc};
use image::{DynamicImage, GenericImageView, ImageReader, Pixel, imageops::FilterType};
use reqwest::Client;
use serde_json::json;
use std::{
    env,
    hash::{DefaultHasher, Hash, Hasher},
    io::Cursor,
    sync::LazyLock,
    time::Duration,
};
use tap::Tap;
use tokio::fs;
use tracing::{error, info};
use uuid::Uuid;

const TIMEOUT: u64 = 360;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

async fn serve_file(file_name: &str) -> Result<(StatusCode, HeaderMap, Vec<u8>), StatusCode> {
    let path = WALLPAPERS_DIR.join(file_name);
    let data = fs::read(&path).await.map_err(|e| {
        error!("Failed to read image file {file_name:?}: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert("Content-Type", v);
    }
    if let Ok(v) = HeaderValue::from_str(&format!("inline; filename=\"{file_name}\"")) {
        headers.insert("Content-Disposition", v);
    }
    Ok((StatusCode::OK, headers, data))
}

pub async fn latest() -> Result<impl IntoResponse, StatusCode> {
    let db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let Some(wallpaper) = db.wallpapers.into_values().max_by_key(|w| w.datetime) else {
        error!("No wallpapers found");
        return Err(StatusCode::NOT_FOUND);
    };
    serve_file(&wallpaper.image_file.file_name).await
}

pub async fn favourites() -> Result<impl IntoResponse, StatusCode> {
    let db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let wallpapers = db
        .wallpapers
        .into_values()
        .filter(|w| matches!(w.liked_state, LikedState::Liked))
        .collect::<Vec<_>>()
        .tap_mut(|v| v.sort_by_key(|w| w.id));

    if wallpapers.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Total hours since Unix epoch as the hash index
    let hour_seed = Utc::now().timestamp() / 3600;
    let index =
        (DefaultHasher::new().tap_mut(|h| hour_seed.hash(h)).finish() as usize) % wallpapers.len();

    serve_file(&wallpapers[index].image_file.file_name).await
}

pub async fn smartget() -> Result<impl IntoResponse, StatusCode> {
    let db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let now = Utc::now();
    let brightness_range = match now.hour() {
        7..=9 | 17..=21 => (0.3, 0.6),
        10..=16 => (0.5, 1.0),
        _ => (0.0, 0.55),
    };

    let wallpapers = db
        .wallpapers
        .into_values()
        .filter(|w| matches!(w.liked_state, LikedState::Liked | LikedState::Loved))
        .collect::<Vec<_>>()
        .tap_mut(|v| {
            // If we have a wallpaper with correct brightness range, filter down to just those
            let has_match = v
                .iter()
                .any(|w| w.brightness >= brightness_range.0 && w.brightness <= brightness_range.1);
            if has_match {
                v.retain(|w| {
                    w.brightness >= brightness_range.0 && w.brightness <= brightness_range.1
                });
            }
            v.sort_by_key(|w| w.id);
        });

    if wallpapers.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Total hours since Unix epoch as the hash index
    let hour_seed = now.timestamp() / 3600;
    let index =
        (DefaultHasher::new().tap_mut(|h| hour_seed.hash(h)).finish() as usize) % wallpapers.len();

    serve_file(&wallpapers[index].image_file.file_name).await
}

pub async fn generate_wallpaper_impl(
    prompt_data: Option<PromptData>,
    message: Option<String>,
    id: Uuid,
) -> Result<()> {
    info!("Generating wallpaper");

    routing::update_generation_event(id, GenerationStage::WaitingForPrompt).await;

    let datetime = Utc::now();
    let client = &*HTTP_CLIENT;
    let api_token =
        env::var("REPLICATE_API_TOKEN").expect("REPLICATE_API_TOKEN environment variable not set");

    let prompt_data = match prompt_data {
        Some(p) => p,
        None => gpt::generate(message)
            .await?
            .tap(|p| info!("Generated prompt: {}", p.prompt)),
    };

    routing::update_generation_event(
        id,
        GenerationStage::ReceivedPrompt {
            prompt: prompt_data.shortened_prompt.clone(),
        },
    )
    .await;

    let (image_url, image) = image_diffusion(client, &api_token, &prompt_data.prompt).await?;
    info!("Generated image: {}", &image_url);

    routing::update_generation_event(id, GenerationStage::ReceivedImage).await;

    let datetime_str = datetime.to_rfc3339();

    let file_name = format!("{datetime_str}.webp");
    fs::write(WALLPAPERS_DIR.join(&file_name), encode_webp(&image, 90.0)).await?;
    let image_file = ImageFile {
        file_name,
        width: image.width(),
        height: image.height(),
    };

    let thumb_image = image.resize_to_fill(640, 360, FilterType::Lanczos3);
    let thumb_file_name = format!("{datetime_str}_thumb.webp");
    fs::write(
        WALLPAPERS_DIR.join(&thumb_file_name),
        encode_webp(&thumb_image, 70.0),
    )
    .await?;
    let thumbnail_file = ImageFile {
        file_name: thumb_file_name,
        width: thumb_image.width(),
        height: thumb_image.height(),
    };

    let wallpaper = WallpaperData {
        id,
        datetime,
        prompt_data,
        image_file,
        brightness: top_20_percent_brightness(&thumb_image),
        thumbnail_file,
        liked_state: LikedState::Neutral,
        comment: None,
    };

    let mut database = read_database().await?;
    database.wallpapers.insert(id, wallpaper);
    write_database(&database).await?;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        routing::remove_generation_event(id).await;
    });

    Ok(())
}

fn encode_webp(img: &DynamicImage, quality: f32) -> Vec<u8> {
    webp::Encoder::from_image(img)
        .unwrap()
        .encode(quality)
        .to_vec()
}

fn top_20_percent_brightness(img: &DynamicImage) -> f32 {
    let brightness_values: Vec<f32> = img
        .pixels()
        .map(|(_, _, pixel)| {
            let [r, g, b] = pixel.to_rgb().0;
            let (r, g, b) = (
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
            );
            0.114f32.mul_add(b, 0.299f32.mul_add(r, 0.587f32 * g))
        })
        .collect::<Vec<_>>()
        .tap_mut(|v| v.sort_unstable_by(f32::total_cmp));

    if brightness_values.is_empty() {
        return 0.0;
    }
    let len = brightness_values.len();
    let top_index = ((len as f32) * 0.80).ceil() as usize - 1;
    brightness_values[top_index.min(len - 1)]
}

/// <https://replicate.com/bytedance/seedream-4>
async fn image_diffusion(
    client: &Client,
    api_token: &str,
    prompt: &str,
) -> Result<(String, DynamicImage)> {
    let result_url = replicate_request_prediction(
        client,
        api_token,
        "https://api.replicate.com/v1/models/bytedance/seedream-4/predictions",
        &json!({
            "input": {
                "prompt": prompt,
                "size": "custom",
                "width": 3840,
                "height": 2160,
                "max_images": 1,
                "image_input": [],
                "aspect_ratio": "4:3",
                "sequential_image_generation": "disabled"
            }
        }),
    )
    .await?;

    let img_data = client.get(&result_url).send().await?.bytes().await?;
    let img = ImageReader::new(Cursor::new(img_data))
        .with_guessed_format()?
        .decode()?;

    Ok((result_url, img))
}

async fn replicate_request_prediction(
    client: &Client,
    api_token: &str,
    url: &str,
    input_json: &serde_json::Value,
) -> Result<String> {
    let auth_header = format!("Bearer {api_token}");
    let response = client
        .post(url)
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .json(input_json)
        .send()
        .await?;

    let response_json = response.json::<serde_json::Value>().await?;
    let status_url = response_json["urls"]["get"]
        .as_str()
        .ok_or_else(|| anyhow!("No valid status URL found"))?
        .to_string();

    for _ in 0..TIMEOUT {
        let status_response = client
            .get(&status_url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status_json = status_response.json::<serde_json::Value>().await?;

        if status_json["status"] == "succeeded" {
            let url = status_json["output"]
                .as_str()
                .or_else(|| status_json["output"].as_array()?.first()?.as_str());
            if let Some(url) = url {
                return Ok(url.to_string());
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Err(anyhow!("Operation timed out or failed"))
}
