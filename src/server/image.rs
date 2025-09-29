use crate::{
    WALLPAPERS_DIR,
    common::{
        ColorData, ImageFile, LikeBody, LikedState, NetworkPacket, PromptData, WallpaperData,
    },
    server::{decode_and_verify, gpt, read_database, with_db, write_database},
};
use anyhow::{Result, anyhow};
use axum::{
    body::Bytes,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{Timelike, Utc};
use image::{
    DynamicImage, GenericImageView, ImageReader, Pixel, codecs::jpeg::JpegEncoder,
    imageops::FilterType,
};
use log::{error, info};
use rand::seq::IteratorRandom;
use reqwest::Client;
use serde_json::json;
use std::{env, io::Cursor, path::Path, time::Duration};
use thumbhash::rgba_to_thumb_hash;
use tokio::fs;
use uuid::Uuid;

const TIMEOUT: u64 = 360;

pub async fn generate(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<String> = decode_and_verify(packet).await?;

    let prompt = (!pkt.data.is_empty()).then_some(pkt.data);
    generate_wallpaper_impl(None, prompt)
        .await
        .map(|()| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to generate wallpaper: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn latest() -> Result<impl IntoResponse, StatusCode> {
    let db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Find latest wallpaper by datetime
    let wallpaper = db
        .wallpapers
        .into_values()
        .max_by_key(|w| w.datetime)
        .ok_or_else(|| {
            error!("No wallpapers found");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let file_name = wallpaper
        .upscaled_file
        .as_ref()
        .map_or(&wallpaper.original_file.file_name, |f| &f.file_name);

    let image_path = Path::new(WALLPAPERS_DIR).join(file_name);
    let data = fs::read(&image_path).await.map_err(|e| {
        error!("Failed to read image file {file_name:?}: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mime_type = mime_guess::from_path(&image_path).first_or_octet_stream();
    let mut headers = HeaderMap::new();
    if let Ok(content_type) = HeaderValue::from_str(mime_type.as_ref()) {
        headers.insert("Content-Type", content_type);
    }

    Ok((StatusCode::OK, headers, data))
}

pub async fn favourites() -> Result<impl IntoResponse, StatusCode> {
    let db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Find random liked wallpaper
    let file_name = {
        let mut rng = rand::rng();
        let wallpaper = db
            .wallpapers
            .into_values()
            .filter(|w| matches!(w.liked_state, LikedState::Liked))
            .choose(&mut rng)
            .ok_or_else(|| {
                error!("No liked wallpapers found");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        wallpaper
            .upscaled_file
            .as_ref()
            .map_or(&wallpaper.original_file.file_name, |f| &f.file_name)
            .clone()
    };

    let image_path = Path::new(WALLPAPERS_DIR).join(&file_name);
    let data = fs::read(&image_path).await.map_err(|e| {
        error!("Failed to read image file {file_name:?}: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mime_type = mime_guess::from_path(&image_path).first_or_octet_stream();
    let mut headers = HeaderMap::new();
    if let Ok(content_type) = HeaderValue::from_str(mime_type.as_ref()) {
        headers.insert("Content-Type", content_type);
    }

    Ok((StatusCode::OK, headers, data))
}

pub async fn smartget() -> Result<impl IntoResponse, StatusCode> {
    let now = Utc::now();
    let hour = now.hour();

    // Define acceptable brightness range based on the time of day.
    let acceptable_brightness_range = if (hour > 6 && hour < 10) || hour > 16 && hour < 22 {
        (0.3, 0.6)
    } else if (10..=16).contains(&hour) {
        (0.5, 1.0)
    } else {
        (0.0, 0.55)
    };

    let db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Find random wallpaper that meets the criteria
    let file_name = {
        let mut rng = rand::rng();
        let wallpaper = db
            .wallpapers
            .into_values()
            .filter(|wallpaper| {
                matches!(wallpaper.liked_state, LikedState::Liked | LikedState::Loved)
                    && wallpaper.color_data.top_20_percent_brightness
                        >= acceptable_brightness_range.0
                    && wallpaper.color_data.top_20_percent_brightness
                        <= acceptable_brightness_range.1
            })
            .choose(&mut rng)
            .ok_or_else(|| {
                error!("No liked wallpapers found");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        wallpaper
            .upscaled_file
            .as_ref()
            .map_or(&wallpaper.original_file.file_name, |f| &f.file_name)
            .clone()
    };

    let image_path = Path::new(WALLPAPERS_DIR).join(&file_name);
    let data = fs::read(&image_path).await.map_err(|e| {
        error!("Failed to read image file {file_name:?}: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mime_type = mime_guess::from_path(&image_path).first_or_octet_stream();
    let mut headers = HeaderMap::new();
    if let Ok(content_type) = HeaderValue::from_str(mime_type.as_ref()) {
        headers.insert("Content-Type", content_type);
    }

    Ok((StatusCode::OK, headers, data))
}

pub async fn remove(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<Uuid> = decode_and_verify(packet).await?;

    remove_wallpaper_impl(pkt)
        .await
        .map(|()| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to remove wallpaper: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn like(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<LikeBody> = decode_and_verify(packet).await?;

    // Set the vote state
    let wallpaper = with_db(|db| {
        if let Some(wallpaper) = db.wallpapers.get_mut(&pkt.data.uuid) {
            wallpaper.liked_state = if wallpaper.liked_state == pkt.data.liked {
                LikedState::Neutral
            } else {
                pkt.data.liked
            };
            Ok(wallpaper.clone())
        } else {
            error!("Like: wallpaper not found {}", pkt.data.uuid);
            Err(StatusCode::NOT_FOUND)
        }
    })
    .await?;

    // Run the upscaling if the image was liked
    if wallpaper.upscaled_file.is_none()
        && matches!(wallpaper.liked_state, LikedState::Liked | LikedState::Loved)
    {
        tokio::spawn(async move {
            if let Err(e) = upscale_wallpaper_impl(pkt.data.uuid, wallpaper).await {
                error!("Failed to upscale wallpaper: {e:?}");
            }
        });
    }

    Ok(StatusCode::OK)
}

pub async fn recreate(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<Uuid> = decode_and_verify(packet).await?;

    // Get the prompt
    let prompt_data = read_database()
        .await
        .map_err(|e| {
            error!("DB read failed: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .wallpapers
        .get(&pkt.data)
        .map(|w| w.prompt_data.clone())
        .ok_or_else(|| {
            error!("Recreate: wallpaper not found {}", pkt.data);
            StatusCode::NOT_FOUND
        })?;

    generate_wallpaper_impl(Some(prompt_data), None)
        .await
        .map(|()| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to recreate image: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn generate_wallpaper_impl(
    prompt_data: Option<PromptData>,
    message: Option<String>,
) -> Result<()> {
    info!("Generating wallpaper");

    let id = Uuid::new_v4();
    let datetime = Utc::now();
    let client = Client::new();
    let api_token =
        env::var("REPLICATE_API_TOKEN").expect("REPLICATE_API_TOKEN environment variable not set");

    // Generate image prompt
    let prompt_data = if let Some(prompt_data) = prompt_data {
        prompt_data
    } else {
        let new = gpt::generate(message).await?;
        info!("Generated prompt: {}", new.prompt);
        new
    };

    // Generate image
    let (image_url, image) = image_diffusion(&client, &api_token, &prompt_data.prompt).await?;
    info!("Generated image: {}", &image_url);

    // Resize the image to thumbnail
    let thumbnail = image.thumbnail(32, 32);
    let thumbhash = rgba_to_thumb_hash(
        thumbnail.width() as usize,
        thumbnail.height() as usize,
        thumbnail.into_rgba8().as_raw(),
    );

    // Save to file
    let dir = Path::new(WALLPAPERS_DIR);
    fs::create_dir_all(dir).await?;

    let datetime_str = datetime.to_rfc3339();

    // Save the original image
    let file_name = format!("{datetime_str}.webp");
    std::fs::write(
        dir.join(&file_name),
        &*webp::Encoder::from_image(&image).unwrap().encode(90.0),
    )?;
    // image.save_with_format(dir.join(&file_name), ImageFormat::Avif)?;
    let original_file = ImageFile {
        file_name,
        width: image.width(),
        height: image.height(),
    };

    // Downscale to 480p and save as thumbnail file
    let thumb_image = image.resize_to_fill(640, 360, FilterType::Lanczos3);
    let thumb_file_name = format!("{datetime_str}_thumb.webp");
    std::fs::write(
        dir.join(&thumb_file_name),
        &*webp::Encoder::from_image(&thumb_image)
            .unwrap()
            .encode(90.0),
    )?;
    // thumb_image.save_with_format(dir.join(&thumb_file_name), ImageFormat::Avif)?;
    let thumbnail_file = ImageFile {
        file_name: thumb_file_name,
        width: thumb_image.width(),
        height: thumb_image.height(),
    };

    // Calculate average color and brightness
    let color_data = calculate_color_data(&thumb_image);

    let wallpaper = WallpaperData {
        id,
        datetime,

        prompt_data,
        original_file,
        upscaled_file: None,
        color_data,

        thumbnail_file,
        thumbhash,
        liked_state: LikedState::Neutral,
    };

    // Store a new database entry
    let mut database = read_database().await?;
    database.wallpapers.insert(id, wallpaper);
    write_database(&database).await?;

    Ok(())
}

pub async fn upscale_wallpaper_impl(id: Uuid, wallpaper: WallpaperData) -> Result<()> {
    info!("Upscaling wallpaper {id}");

    // Prepare client
    let client = Client::new();
    let api_token =
        env::var("REPLICATE_API_TOKEN").expect("REPLICATE_API_TOKEN environment variable not set");

    // Open image file
    let image_path = Path::new(WALLPAPERS_DIR).join(wallpaper.original_file.file_name.clone());
    let image = image::open(&image_path)?;

    // Upscale the image using the high quality upscaler
    let (upscaled_url, upscaled_image) = upscale_image(&client, &api_token, &image).await?;
    info!("Upscaled image: {}", &upscaled_url);
    let upscaled_image = upscaled_image.resize_to_fill(3840, 2160, FilterType::Lanczos3);

    // Save to file
    let dir = Path::new(WALLPAPERS_DIR);
    fs::create_dir_all(dir).await?;
    let datetime_str = wallpaper.datetime.to_rfc3339();

    // Save the upscaled image
    let upscaled_file_name = format!("{datetime_str}_upscaled.webp");
    std::fs::write(
        dir.join(&upscaled_file_name),
        &*webp::Encoder::from_image(&upscaled_image)
            .unwrap()
            .encode(90.0),
    )?;
    // upscaled_image.save_with_format(dir.join(&upscaled_file_name), ImageFormat::Avif)?;
    let upscaled_file = Some(ImageFile {
        file_name: upscaled_file_name,
        width: upscaled_image.width(),
        height: upscaled_image.height(),
    });

    // Downscale to 480p and save as thumbnail file
    let thumb_image = upscaled_image.resize_to_fill(640, 360, FilterType::Lanczos3);
    let thumb_file_name = format!("{datetime_str}_thumb.webp");
    std::fs::write(
        dir.join(&thumb_file_name),
        &*webp::Encoder::from_image(&thumb_image)
            .unwrap()
            .encode(90.0),
    )?;
    // thumb_image.save_with_format(dir.join(&thumb_file_name), ImageFormat::Avif)?;
    let thumbnail_file = ImageFile {
        file_name: thumb_file_name,
        width: thumb_image.width(),
        height: thumb_image.height(),
    };

    // Calculate average color and brightness
    let color_data = calculate_color_data(&thumb_image);

    let wallpaper = WallpaperData {
        upscaled_file,
        color_data,
        thumbnail_file,
        ..wallpaper
    };

    // Update the database entry
    let mut database = read_database().await?;
    database.wallpapers.insert(id, wallpaper);
    write_database(&database).await?;

    Ok(())
}

fn calculate_color_data(img: &DynamicImage) -> ColorData {
    let (width, height) = img.dimensions();
    let total_pixels = (width * height) as f32;

    // Sum up all the RGB values and brightness
    let (sum_r, sum_g, sum_b, mut brightness_values) = img.pixels().fold(
        (0.0, 0.0, 0.0, Vec::new()),
        |(acc_r, acc_g, acc_b, mut brightness_values), (_, _, pixel)| {
            let [r, g, b] = pixel.to_rgb().0;
            let (r, g, b) = (
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
            );
            let brightness = 0.114f32.mul_add(b, 0.299f32.mul_add(r, 0.587f32 * g));
            brightness_values.push(brightness);
            (acc_r + r, acc_g + g, acc_b + b, brightness_values)
        },
    );

    let avg_r = sum_r / total_pixels;
    let avg_g = sum_g / total_pixels;
    let avg_b = sum_b / total_pixels;

    let (hue, saturation, lightness) = rgb_to_hsl(avg_r, avg_g, avg_b);
    let chroma = calculate_chroma_hsl(lightness, saturation);

    // Compute brightness percentiles
    brightness_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let top_20_percent_brightness =
        brightness_values[(brightness_values.len() as f32 * 0.80).ceil() as usize - 1];
    let bottom_20_percent_brightness =
        brightness_values[(brightness_values.len() as f32 * 0.20).floor() as usize];

    // Calculate contrast ratio
    let contrast_ratio = (top_20_percent_brightness + 0.05) / (bottom_20_percent_brightness + 0.05);

    ColorData {
        average_color: (avg_r, avg_b, avg_g),
        hue,
        saturation,
        lightness,
        chroma,
        top_20_percent_brightness,
        bottom_20_percent_brightness,
        contrast_ratio,
    }
}

/// Convert RGB to HSL, each value is in the range [0,1]
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = min.midpoint(max);

    let mut hue = 0.0;
    let mut saturation = 0.0;
    if (max - min).abs() > f32::EPSILON {
        let d = max - min;
        saturation = if lightness > 0.5 {
            d / (2.0 - d)
        } else {
            d / (max + min)
        };

        if (max - r).abs() > f32::EPSILON {
            hue = (g - b) / d + if g < b { 6.0 } else { 0.0 };
        } else if (max - g).abs() > f32::EPSILON {
            hue = (b - r) / d + 2.0;
        } else {
            hue = (r - g) / d + 4.0;
        }
        hue /= 6.0;
    }

    (hue, saturation, lightness)
}

/// Calculate chroma (perceived intensity of color) from hue and saturation in HSL.
fn calculate_chroma_hsl(lightness: f32, saturation: f32) -> f32 {
    (1.0 - 2.0f32.mul_add(lightness, -1.0).abs()) * saturation
}

async fn remove_wallpaper_impl(packet: NetworkPacket<Uuid>) -> Result<()> {
    let mut database = read_database().await?;

    let wallpaper = database
        .wallpapers
        .remove(&packet.data)
        .ok_or_else(|| anyhow!("No entry found for UUID"))?;

    // Remove all associated files
    for file_name in vec![
        Some(&wallpaper.original_file.file_name),
        Some(&wallpaper.thumbnail_file.file_name),
        wallpaper.upscaled_file.as_ref().map(|f| &f.file_name),
    ]
    .into_iter()
    .flatten()
    {
        let file_path = Path::new(WALLPAPERS_DIR).join(file_name);
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
    }

    // Save the updated database
    write_database(&database).await?;

    Ok(())
}

/// <https://replicate.com/bytedance/seedream-3>
async fn image_diffusion(
    client: &Client,
    api_token: &str,
    prompt: &str,
) -> Result<(String, DynamicImage)> {
    let result_url = replicate_request_prediction(
        client,
        api_token,
        "https://api.replicate.com/v1/models/bytedance/seedream-3/predictions",
        &json!({
            "input": {
                "prompt": prompt,
                "size": "big",
                "aspect_ratio": "custom",
                "width": 1920,
                "height": 1080,
                "guidance_scale": 2.5
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

/// <https://replicate.com/philz1337x/clarity-upscaler>
async fn upscale_image(
    client: &Client,
    api_token: &str,
    image: &DynamicImage,
) -> Result<(String, DynamicImage)> {
    let mut bytes = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut bytes, 90);
    image.write_with_encoder(encoder)?;
    let image_uri = format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes));

    let result_url = replicate_request_prediction(
        client,
        api_token,
        "https://api.replicate.com/v1/models/recraft-ai/recraft-crisp-upscale/predictions",
        &json!({
            "input": {
                "image": image_uri,
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
    model: &str,
    input_json: &serde_json::Value,
) -> Result<String> {
    let url = if model.is_empty() {
        "https://api.replicate.com/v1/predictions"
    } else {
        model
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

    for _ in 0..TIMEOUT {
        let status_response = client
            .get(status_url.clone())
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
