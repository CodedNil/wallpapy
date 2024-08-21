use anyhow::{anyhow, Result};
use async_openai::{
    types::{CreateImageRequestArgs, Image, ImageModel, ImageSize, ResponseFormat},
    Client,
};
use chrono::Utc;
use reqwest::get;
use std::path::Path;
use tokio::{fs, io::AsyncWriteExt};

pub async fn generate(prompt: &str) -> Result<String> {
    let client = Client::new();

    let request = CreateImageRequestArgs::default()
        .prompt(prompt)
        .n(1)
        .model(ImageModel::DallE3)
        .response_format(ResponseFormat::Url)
        .size(ImageSize::S1792x1024)
        .user("wallpapy")
        .build()?;

    let response = client.images().create(request).await?;

    let url = response
        .data
        .first()
        .and_then(|arc_image| {
            if let Image::Url { url, .. } = &**arc_image {
                Some(url.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("No valid image URL found"))?;

    let response = get(&url).await?;
    let dir = Path::new("wallpapers");
    fs::create_dir_all(dir).await?;

    let datetime = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let file_name = format!("{datetime}.jpg");
    let file_path = dir.join(file_name.clone());
    let mut file = fs::File::create(file_path).await?;
    let content = response.bytes().await?;
    file.write_all(&content).await?;

    Ok(file_name)
}
