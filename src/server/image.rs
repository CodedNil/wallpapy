use super::{auth::verify_token, prompt, DATABASE_PATH};
use crate::common::{TokenPacket, WallpaperData};
use anyhow::{anyhow, Result};
use async_openai::{
    types::{CreateImageRequestArgs, Image, ImageModel, ImageSize, ResponseFormat},
    Client,
};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::Utc;
use image::GenericImageView;
use std::path::Path;
use thumbhash::rgba_to_thumb_hash;
use tokio::fs;
use uuid::Uuid;

const IMAGES_TREE: &str = "images";

pub async fn generate_wallpaper(packet: Bytes) -> impl IntoResponse {
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

pub async fn get_wallpapers() -> impl IntoResponse {
    let db = match sled::open(DATABASE_PATH) {
        Ok(db) => db,
        Err(e) => {
            return {
                log::error!("{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            .into_response()
        }
    };
    let tree = match db.open_tree(IMAGES_TREE) {
        Ok(tree) => tree,
        Err(e) => {
            return {
                log::error!("{:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            .into_response()
        }
    };

    let images: Vec<WallpaperData> = tree
        .iter()
        .values()
        .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
        .collect();

    match bincode::serialize(&images) {
        Ok(data) => (StatusCode::OK, data).into_response(),
        Err(e) => {
            log::error!("{:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
        .into_response(),
    }
}

async fn generate_wallpaper_impl() -> Result<()> {
    log::info!("Generating wallpaper");

    let id = Uuid::new_v4();
    let datetime = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    // Generate image
    let prompt = prompt::generate().await?;
    let img_data = generate(&prompt).await?;

    // Decode the image data to get dimensions and get in RGBA format
    let img = image::load_from_memory(&img_data)?;
    let (width, height) = img.dimensions();

    // Resize the image to thumbnail
    let thumbnail = img.thumbnail(32, 32);
    let thumbhash = rgba_to_thumb_hash(
        thumbnail.width() as usize,
        thumbnail.height() as usize,
        thumbnail.into_rgba8().as_raw(),
    );

    // Save to file
    let dir = Path::new("wallpapers");
    fs::create_dir_all(dir).await?;
    let file_name = format!("{datetime}.jpg");
    fs::write(&dir.join(&file_name), img_data).await?;

    let image_data = WallpaperData {
        id,
        datetime,
        prompt,
        file_name,
        width,
        height,
        thumbhash,
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

async fn generate(prompt: &str) -> Result<Vec<u8>> {
    let client = Client::new();

    let response = client
        .images()
        .create(
            CreateImageRequestArgs::default()
                .prompt(prompt)
                .n(1)
                .model(ImageModel::DallE3)
                .response_format(ResponseFormat::B64Json)
                .size(ImageSize::S1792x1024)
                .user("wallpapy")
                .build()?,
        )
        .await?;

    let b64_json = response
        .data
        .first()
        .and_then(|arc_image| match **arc_image {
            Image::B64Json { ref b64_json, .. } => Some(b64_json),
            Image::Url { .. } => None,
        })
        .ok_or_else(|| anyhow!("No valid image data found"))?;

    let img_data = BASE64_STANDARD.decode(&**b64_json)?;

    Ok(img_data)
}
