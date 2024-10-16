use crate::common::{Database, DatabaseStyle};
use anyhow::Result;
use chrono::Duration;
use std::collections::HashMap;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncReadExt,
};

mod auth;
mod commenting;
mod gpt;
mod image;
pub mod routing;

const DATABASE_FILE: &str = "data/database.ron";

async fn read_database() -> Result<Database> {
    if fs::metadata(DATABASE_FILE).await.is_err() {
        return Ok(Database {
            style: DatabaseStyle::default(),
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
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = duration.num_weeks();

    match (weeks, days, hours, minutes) {
        (w, _, _, _) if w >= 1 => format!("{} week{}", w, if w == 1 { "" } else { "s" }),
        (_, d, _, _) if d >= 1 => format!("{} day{}", d, if d == 1 { "" } else { "s" }),
        (_, _, h, _) if h >= 1 => format!("{} hour{}", h, if h == 1 { "" } else { "s" }),
        (_, _, _, m) if m >= 1 => format!("{} minute{}", m, if m == 1 { "" } else { "s" }),
        _ => "less than a minute".to_string(),
    }
}
