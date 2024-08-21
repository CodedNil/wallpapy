use anyhow::{anyhow, Result};
use async_openai::{
    types::{CreateImageRequestArgs, Image, ImageModel, ImageSize, ResponseFormat},
    Client,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::Utc;
use std::path::Path;
use tokio::fs;

pub async fn generate(prompt: &str) -> Result<String> {
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

    let dir = Path::new("wallpapers");
    fs::create_dir_all(dir).await?;

    let file_name = format!("{}.jpg", Utc::now().format("%Y-%m-%d_%H-%M-%S"));
    let file_path = dir.join(&file_name);
    fs::write(&file_path, img_data).await?;

    Ok(file_name)
}
