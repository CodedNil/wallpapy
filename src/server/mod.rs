use crate::{
    common::{Database, DatabaseStyle, HasToken},
    server::auth::verify_token,
};
use anyhow::Result;
use axum::{body::Bytes, http::StatusCode};
use bincode::serde::decode_from_slice;
use chrono::Duration;
use log::error;
use serde::de::DeserializeOwned;
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

pub async fn decode_and_verify<P>(bytes: Bytes) -> Result<P, StatusCode>
where
    P: DeserializeOwned + HasToken,
{
    let (pkt, _): (P, usize) =
        decode_from_slice(&bytes, bincode::config::standard()).map_err(|e| {
            error!("failed to deserialize packet: {e:?}");
            StatusCode::BAD_REQUEST
        })?;

    if !verify_token(pkt.token()).await.unwrap_or(false) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(pkt)
}

pub async fn read_database() -> Result<Database> {
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

pub async fn write_database(database: &Database) -> Result<()> {
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let data = ron::ser::to_string_pretty(database, pretty)?;
    fs::write(DATABASE_FILE, data).await?;
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

fn format_duration(duration: Duration) -> String {
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = duration.num_weeks();

    if weeks >= 1 {
        return format!("{} week{}", weeks, if weeks == 1 { "" } else { "s" });
    }
    if days >= 1 {
        return format!("{} day{}", days, if days == 1 { "" } else { "s" });
    }
    if hours >= 1 {
        return format!("{} hour{}", hours, if hours == 1 { "" } else { "s" });
    }
    if minutes >= 1 {
        return format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" });
    }

    "less than a minute".to_string()
}
