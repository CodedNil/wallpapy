use crate::{
    common::{Database, HasToken},
    server::auth::verify_token,
};
use anyhow::Result;
use axum::{body::Bytes, http::StatusCode};
use chrono::Duration;
use log::error;
use postcard::from_bytes;
use serde::de::DeserializeOwned;
use std::{env, path::PathBuf, sync::LazyLock};
use tokio::fs;

static DATA_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| env::var("DATA_DIR").map_or_else(|_| PathBuf::from("data"), PathBuf::from));
pub static WALLPAPERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("wallpapers"));
static AUTH_FILE: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("auth.ron"));
static DATABASE_FILE: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("database.ron"));

mod auth;
mod commenting;
mod gpt;
mod image;
pub mod routing;

pub async fn decode_and_verify<P>(bytes: Bytes) -> Result<P, StatusCode>
where
    P: DeserializeOwned + HasToken,
{
    let pkt = from_bytes::<P>(&bytes).map_err(|e| {
        error!("failed to deserialize packet: {e:?}");
        StatusCode::BAD_REQUEST
    })?;

    if !verify_token(pkt.token()).await.unwrap_or(false) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(pkt)
}

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

fn format_duration(duration: Duration) -> String {
    let (n, unit) = if duration.num_weeks() >= 1 {
        (duration.num_weeks(), "week")
    } else if duration.num_days() >= 1 {
        (duration.num_days(), "day")
    } else if duration.num_hours() >= 1 {
        (duration.num_hours(), "hour")
    } else if duration.num_minutes() >= 1 {
        (duration.num_minutes(), "minute")
    } else {
        return "less than a minute".to_string();
    };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}
