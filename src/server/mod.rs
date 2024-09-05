use crate::common::{CommentData, WallpaperData};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncReadExt,
};
use uuid::Uuid;

mod auth;
mod commenting;
mod image;
mod prompt;
pub mod routing;

const DATABASE_FILE: &str = "database.ron";

#[derive(Serialize, Deserialize)]
struct Database {
    wallpapers: HashMap<Uuid, WallpaperData>,
    comments: HashMap<Uuid, CommentData>,
}

async fn read_database() -> Result<Database> {
    if fs::metadata(DATABASE_FILE).await.is_err() {
        return Ok(Database {
            wallpapers: HashMap::new(),
            comments: HashMap::new(),
        });
    }

    let mut file = OpenOptions::new().read(true).open(DATABASE_FILE).await?;
    let mut data = String::new();
    file.read_to_string(&mut data).await?;
    let database: Database = ron::from_str(&data)?;
    Ok(database)
}

async fn write_database(database: &Database) -> Result<()> {
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let data = ron::ser::to_string_pretty(database, pretty)?;
    fs::write(DATABASE_FILE, data).await?;
    Ok(())
}
