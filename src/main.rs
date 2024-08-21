mod image;
mod prompt;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const DATABASE_PATH: &str = "database";
const IMAGES_TREE: &str = "images";

#[derive(Debug, Serialize, Deserialize)]
struct ImageData {
    prompt: String,
    image_path: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    create_wallpaper().await.unwrap();
}

async fn create_wallpaper() -> Result<()> {
    let prompt = prompt::generate().await?;
    let image_path = image::generate(&prompt).await?;

    let db = sled::open(DATABASE_PATH).map_err(|e| anyhow!("Failed to open database: {:?}", e))?;
    let tree = db
        .open_tree(IMAGES_TREE)
        .map_err(|e| anyhow!("Failed to open tree: {:?}", e))?;

    // Store a new database entry with datetime as key and image_path and prompt
    let datetime = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let image_data = ImageData {
        prompt: prompt.clone(),
        image_path,
    };
    tree.insert(datetime.as_bytes(), bincode::serialize(&image_data)?)
        .map_err(|e| anyhow!("Failed to insert into tree: {:?}", e))?;

    Ok(())
}
