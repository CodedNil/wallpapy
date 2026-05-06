use anyhow::Result;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, path::PathBuf, sync::LazyLock};
use tokio::fs;
use tracing::error;
use uuid::Uuid;

use crate::common::WallpaperData;

#[derive(Serialize, Deserialize, Clone)]
pub struct Database {
    pub style: String,
    pub wallpapers: HashMap<Uuid, WallpaperData>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            style: "Style: Digital paintings, colourful, looks great as a desktop wallpaper even when heavily blurred behind apps\nContents: Epic fantasy, surreal, abstract, landscapes\nAvoid: No people, don\'t go for highly complex".to_string(),
            wallpapers: HashMap::default(),
        }
    }
}

static DATA_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| env::var("DATA_DIR").map_or_else(|_| PathBuf::from("data"), PathBuf::from));
pub static WALLPAPERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("wallpapers"));
static DATABASE_FILE: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("database.ron"));

pub async fn read_database() -> Result<Database> {
    match fs::read_to_string(&*DATABASE_FILE).await {
        Ok(data) => Ok(ron::from_str(&data)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Database::default()),
        Err(e) => Err(e.into()),
    }
}

pub async fn write_database(database: &Database) -> Result<()> {
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let data = ron::ser::to_string_pretty(database, pretty)?;
    fs::write(&*DATABASE_FILE, data).await?;
    Ok(())
}

pub async fn with_db<F, T>(f: F) -> Result<T, StatusCode>
where
    F: FnOnce(&mut Database) -> Result<T, StatusCode>,
{
    let mut db = read_database().await.map_err(|e| {
        error!("db read error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let result = f(&mut db)?;

    write_database(&db).await.map_err(|e| {
        error!("db write error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(result)
}
