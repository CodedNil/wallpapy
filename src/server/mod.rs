use crate::common::{CommentData, WallpaperData};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::Duration;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncReadExt,
};
use uuid::Uuid;

mod auth;
mod commenting;
mod gpt;
mod image;
pub mod routing;

const DATABASE_FILE: &str = "database.ron";

#[derive(Serialize, Deserialize)]
struct Database {
    key_style: String,
    wallpapers: HashMap<Uuid, WallpaperData>,
    comments: HashMap<Uuid, CommentData>,
}

async fn read_database() -> Result<Database> {
    if fs::metadata(DATABASE_FILE).await.is_err() {
        return Ok(Database {
            key_style: String::new(),
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

fn format_duration(duration: Duration) -> String {
    let minutes = duration.whole_minutes();
    let hours = duration.whole_hours();
    let days = duration.whole_days();
    let weeks = duration.whole_weeks();

    match (weeks, days, hours, minutes) {
        (w, _, _, _) if w >= 1 => format!("{} week{}", w, if w == 1 { "" } else { "s" }),
        (_, d, _, _) if d >= 1 => format!("{} day{}", d, if d == 1 { "" } else { "s" }),
        (_, _, h, _) if h >= 1 => format!("{} hour{}", h, if h == 1 { "" } else { "s" }),
        (_, _, _, m) if m >= 1 => format!("{} minute{}", m, if m == 1 { "" } else { "s" }),
        _ => "less than a minute".to_string(),
    }
}
