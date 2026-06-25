use crate::{
    common::{GenerationStage, LikedState, WallpaperData},
    database, gpt, server,
};
use anyhow::{Result, anyhow};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose};
use chrono::{Timelike, Utc};
use image::{DynamicImage, GenericImageView, ImageReader, codecs::avif::AvifEncoder};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{env, io::Cursor, sync::LazyLock, time::Duration};
use tokio::fs;
use tower_http::services::ServeFile;
use tracing::{error, info, warn};
use uuid::Uuid;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

fn hourly_index(timestamp: i64, len: usize) -> usize {
    let mut x = u64::try_from(timestamp.div_euclid(3600)).unwrap_or_default();
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((x ^ (x >> 31)) as usize) % len
}

async fn serve_wallpaper(file_name: &str) -> Result<Response, StatusCode> {
    ServeFile::new(database::WALLPAPERS_DIR.join(file_name))
        .try_call(Request::new(Body::empty()))
        .await
        .map(IntoResponse::into_response)
        .map_err(|e| {
            error!("Failed to serve image file {file_name:?}: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn latest() -> Result<Response, StatusCode> {
    let Some(image_file) = database::get_latest_image_file().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    else {
        error!("No wallpapers found");
        return Err(StatusCode::NOT_FOUND);
    };
    serve_wallpaper(&image_file).await
}

pub async fn favourites() -> Result<Response, StatusCode> {
    let wallpapers =
        database::get_wallpaper_choices_by_liked_state(&[LikedState::Liked, LikedState::Loved])
            .await
            .map_err(|e| {
                error!("db read error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    if wallpapers.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let index = hourly_index(Utc::now().timestamp(), wallpapers.len());
    serve_wallpaper(&wallpapers[index].image_file).await
}

pub async fn smartget() -> Result<Response, StatusCode> {
    let mut wallpapers =
        database::get_wallpaper_choices_by_liked_state(&[LikedState::Liked, LikedState::Loved])
            .await
            .map_err(|e| {
                error!("db read error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    let now = Utc::now();
    let brightness_range = match now.hour() {
        7..=9 | 17..=21 => (0.5, 0.65),
        10..=16 => (0.65, 1.0),
        _ => (0.0, 0.5),
    };

    // If we have a wallpaper with correct brightness range, filter down to just those.
    let has_match = wallpapers.iter().any(|w| {
        w.image_brightness >= brightness_range.0 && w.image_brightness <= brightness_range.1
    });
    if has_match {
        wallpapers.retain(|w| {
            w.image_brightness >= brightness_range.0 && w.image_brightness <= brightness_range.1
        });
    }
    wallpapers.sort_by_key(|w| w.id);

    if wallpapers.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let index = hourly_index(now.timestamp(), wallpapers.len());
    serve_wallpaper(&wallpapers[index].image_file).await
}

pub async fn generate_wallpaper_impl(message: Option<String>, id: Uuid) -> Result<()> {
    info!("Generating wallpaper");

    server::update_generation_event(id, GenerationStage::WaitingForPrompt).await;

    let datetime = Utc::now();
    let client = &*HTTP_CLIENT;

    let prompt_data = gpt::generate(message).await?;
    info!("Generated prompt: {}", prompt_data.prompt);

    server::update_generation_event(
        id,
        GenerationStage::ReceivedPrompt {
            prompt: prompt_data.prompt.clone(),
        },
    )
    .await;

    let image = image_diffusion(client, &prompt_data.prompt).await?;
    info!("Generated image {}x{}", image.width(), image.height());

    let datetime_str = datetime.to_rfc3339();
    let file_name = format!("{datetime_str}.avif");
    let mut buffer = Vec::new();
    image.write_with_encoder(AvifEncoder::new_with_speed_quality(&mut buffer, 2, 92))?;
    fs::write(database::WALLPAPERS_DIR.join(&file_name), buffer).await?;

    let wallpaper = WallpaperData {
        id,
        datetime,

        prompt: prompt_data.prompt,
        shortened_prompt: prompt_data.shortened_prompt,
        image_file: file_name,
        image_width: image.width(),
        image_height: image.height(),
        image_brightness: top_20_percent_brightness(&image),

        liked_state: LikedState::Neutral,
        comment: None,
    };

    database::insert_wallpaper(wallpaper).await?;
    server::update_generation_event(id, GenerationStage::ReceivedImage).await;

    tokio::time::sleep(Duration::from_secs(5)).await;
    server::remove_generation_event(id).await;

    Ok(())
}

fn top_20_percent_brightness(img: &DynamicImage) -> f32 {
    let sample_rate = 25;
    let mut histogram = [0usize; 256];
    let mut samples = 0usize;

    for (_, _, pixel) in img.pixels().step_by(sample_rate) {
        let [r, g, b, _] = pixel.0;
        let brightness = (77 * usize::from(r) + 150 * usize::from(g) + 29 * usize::from(b)) >> 8;
        histogram[brightness] += 1;
        samples += 1;
    }

    if samples == 0 {
        return 0.0;
    }

    let target = samples * 4 / 5;
    let mut seen = 0;
    let brightness = histogram
        .iter()
        .enumerate()
        .find_map(|(brightness, count)| {
            seen += count;
            (seen > target).then_some(brightness)
        })
        .unwrap_or(255);
    brightness as f32 / 255.0
}

#[derive(Deserialize)]
struct ImageGenerationResponse {
    data: Vec<GeneratedImage>,
    usage: Option<ImageGenerationUsage>,
}

#[derive(Deserialize)]
struct GeneratedImage {
    b64_json: String,
}

#[derive(Deserialize)]
struct ImageGenerationUsage {
    cost: Option<f64>,
}

/// <https://openrouter.ai/bytedance-seed/seedream-4.5>
async fn image_diffusion(client: &Client, prompt: &str) -> Result<DynamicImage> {
    let response = client
        .post("https://openrouter.ai/api/v1/images")
        .bearer_auth(env::var("OPENROUTER").expect("OPENROUTER environment variable not set"))
        .json(&json!({
            "model": "bytedance-seed/seedream-4.5",
            "prompt": prompt,
            "resolution": "4K",
            "aspect_ratio": "16:9",
            "n": 1,
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Image generation failed {}: {}",
            response.status(),
            response.text().await?,
        ));
    }

    let response = response.json::<ImageGenerationResponse>().await?;
    let cost = response.usage.and_then(|usage| usage.cost);
    if let Some(cost) = cost {
        info!("[IMAGE GENERATION] cost: ${cost}");
    } else {
        warn!("[IMAGE GENERATION] cost unavailable");
    }

    let image_data = response
        .data
        .first()
        .ok_or_else(|| anyhow!("Image data missing from response"))?;

    ImageReader::new(Cursor::new(
        general_purpose::STANDARD.decode(&image_data.b64_json)?,
    ))
    .with_guessed_format()?
    .decode()
    .map_err(|e| anyhow!(e))
}
