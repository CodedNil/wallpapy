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
use image::{
    DynamicImage, GenericImageView, ImageReader, codecs::avif::AvifEncoder, imageops::FilterType,
};
use reqwest::Client;
use serde_json::json;
use std::{
    env,
    hash::{DefaultHasher, Hash, Hasher},
    io::Cursor,
    sync::LazyLock,
    time::Duration,
};
use tokio::fs;
use tower_http::services::ServeFile;
use tracing::{error, info};
use uuid::Uuid;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

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
    let Some(wallpaper) = database::get_latest_wallpaper().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    else {
        error!("No wallpapers found");
        return Err(StatusCode::NOT_FOUND);
    };
    serve_wallpaper(&wallpaper.image_file).await
}

pub async fn favourites() -> Result<Response, StatusCode> {
    let wallpapers =
        database::get_wallpapers_by_liked_state(&[LikedState::Liked, LikedState::Loved])
            .await
            .map_err(|e| {
                error!("db read error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    if wallpapers.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Total hours since Unix epoch as the hash index
    let hour_seed = Utc::now().timestamp() / 3600;
    let mut hasher = DefaultHasher::new();
    hour_seed.hash(&mut hasher);
    let index = hasher.finish() as usize % wallpapers.len();

    serve_wallpaper(&wallpapers[index].image_file).await
}

pub async fn smartget() -> Result<Response, StatusCode> {
    let mut wallpapers =
        database::get_wallpapers_by_liked_state(&[LikedState::Liked, LikedState::Loved])
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

    // Total hours since Unix epoch as the hash index
    let hour_seed = now.timestamp() / 3600;
    let mut hasher = DefaultHasher::new();
    hour_seed.hash(&mut hasher);
    let index = hasher.finish() as usize % wallpapers.len();

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
    let image = image.resize_to_fill(3840, 2160, FilterType::Lanczos3);
    info!("Generated image");

    let datetime_str = datetime.to_rfc3339();
    let file_name = format!("{datetime_str}.avif");
    fs::write(
        database::WALLPAPERS_DIR.join(&file_name),
        encode_avif(&image, 4, 80)?,
    )
    .await?;

    let small_image = image.resize_to_fill(640, 360, FilterType::Lanczos3);
    let wallpaper = WallpaperData {
        id,
        datetime,

        prompt: prompt_data.prompt,
        shortened_prompt: prompt_data.shortened_prompt,
        image_file: file_name,
        image_width: image.width(),
        image_height: image.height(),
        image_brightness: top_20_percent_brightness(&small_image),

        liked_state: LikedState::Neutral,
        comment: None,
    };

    database::insert_wallpaper(wallpaper).await?;
    server::update_generation_event(id, GenerationStage::ReceivedImage).await;

    tokio::time::sleep(Duration::from_secs(5)).await;
    server::remove_generation_event(id).await;

    Ok(())
}

fn encode_avif(img: &DynamicImage, speed: u8, quality: u8) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_with_encoder(AvifEncoder::new_with_speed_quality(
        &mut buf, speed, quality,
    ))?;
    Ok(buf)
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

/// <https://openrouter.ai/bytedance-seed/seedream-4.5>
async fn image_diffusion(client: &Client, prompt: &str) -> Result<DynamicImage> {
    let response = client
        .post("https://openrouter.ai/api/v1/responses")
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                env::var("OPENROUTER").expect("OPENROUTER environment variable not set")
            ),
        )
        .json(&json!({
            "model": "bytedance-seed/seedream-4.5",
            "modalities": ["image"],
            "input": prompt,
            "image_config": {
                "image_size": "4K",
                "aspect_ratio": "16:9"
            }
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

    // Decode and parse the image
    ImageReader::new(Cursor::new(
        general_purpose::STANDARD.decode(
            response.json::<serde_json::Value>().await?["output"][0]["result"]
                .as_str()
                .ok_or_else(|| anyhow!("Image data missing from response"))?
                .split("base64,")
                .nth(1)
                .ok_or_else(|| anyhow!("Malformed data URL string"))?,
        )?,
    ))
    .with_guessed_format()?
    .decode()
    .map_err(|e| anyhow!(e))
}
