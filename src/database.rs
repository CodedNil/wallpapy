use crate::common::{LikedState, WallpaperData};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    env,
    path::PathBuf,
    str::FromStr,
    sync::{LazyLock, OnceLock},
};
use uuid::Uuid;

static DB: OnceLock<SqlitePool> = OnceLock::new();

static DATA_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| env::var("DATA_DIR").map_or_else(|_| PathBuf::from("."), PathBuf::from));
pub static WALLPAPERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("wallpapers"));

fn pool() -> &'static SqlitePool {
    DB.get()
        .expect("Database has not been initialised; call database::init first")
}

/// Initialise the database pool and run pending migrations.
pub async fn init() -> Result<()> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite://{}", DATA_DIR.join("wallpapy.db").display()));
    if let Some(path) = database_url.strip_prefix("sqlite://")
        && path != ":memory:"
        && let Some(parent) = std::path::Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await?;
    }

    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!().run(&db).await?;
    DB.set(db).ok().context("Database already initialised")?;

    Ok(())
}

/// Get all wallpapers, including Disliked ones (for prompt context).
pub async fn get_all_wallpapers() -> Result<Vec<WallpaperData>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM wallpapers ORDER BY datetime DESC")
        .fetch_all(pool())
        .await
}

/// Get wallpapers for the gallery — excludes Disliked.
pub async fn get_gallery_wallpapers() -> Result<Vec<WallpaperData>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM wallpapers
         WHERE liked_state != 'Disliked'
         ORDER BY datetime DESC",
    )
    .fetch_all(pool())
    .await
}

/// Get the most recently created wallpaper.
pub async fn get_latest_wallpaper() -> Result<Option<WallpaperData>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM wallpapers ORDER BY datetime DESC LIMIT 1")
        .fetch_optional(pool())
        .await
}

/// Get the datetime of the most recent wallpaper, or None if no wallpapers exist.
pub async fn get_latest_datetime() -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar("SELECT datetime FROM wallpapers ORDER BY datetime DESC LIMIT 1")
        .fetch_optional(pool())
        .await
}

pub async fn insert_wallpaper(wallpaper: WallpaperData) -> Result<(), sqlx::Error> {
    let WallpaperData {
        id,
        datetime,
        prompt,
        shortened_prompt,
        image_file,
        image_width,
        image_height,
        image_brightness,
        liked_state,
        comment,
    } = wallpaper;

    sqlx::query(
        "INSERT INTO wallpapers
            (id, datetime, prompt, shortened_prompt, image_file, image_width, image_height, image_brightness, liked_state, comment)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(datetime)
    .bind(prompt)
    .bind(shortened_prompt)
    .bind(image_file)
    .bind(image_width)
    .bind(image_height)
    .bind(image_brightness)
    .bind(liked_state)
    .bind(comment)
    .execute(pool())
    .await?;
    Ok(())
}

/// Update liked state for a wallpaper. Returns `true` if the row existed.
pub async fn update_liked_state(id: Uuid, state: LikedState) -> Result<bool, sqlx::Error> {
    let affected = sqlx::query("UPDATE wallpapers SET liked_state = ? WHERE id = ?")
        .bind(state)
        .bind(id)
        .execute(pool())
        .await?
        .rows_affected();
    Ok(affected > 0)
}

/// When a wallpaper is disliked, remove its file from disk but keep the row so
/// it still appears in prompt context history.
pub async fn dislike_and_remove_file(id: Uuid) -> Result<()> {
    let file_name: Option<String> = sqlx::query_scalar(
        "UPDATE wallpapers
         SET liked_state = 'Disliked', image_file = NULL, image_width = NULL, image_height = NULL, image_brightness = NULL
         WHERE id = ?
         RETURNING image_file",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;

    // Remove the file from disk.
    if let Some(file_name) = file_name {
        let file_path = WALLPAPERS_DIR.join(&file_name);
        if let Err(e) = tokio::fs::remove_file(file_path).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e).context("Failed to delete wallpaper file");
        }
    }

    Ok(())
}

/// Get the current liked state for a wallpaper.
pub async fn get_liked_state(id: Uuid) -> Result<Option<LikedState>, sqlx::Error> {
    sqlx::query_scalar("SELECT liked_state FROM wallpapers WHERE id = ?")
        .bind(id)
        .fetch_optional(pool())
        .await
}

/// Update comment for a wallpaper. Returns `true` if the row existed.
pub async fn update_comment(id: Uuid, comment: Option<&str>) -> Result<bool> {
    let affected = sqlx::query("UPDATE wallpapers SET comment = ? WHERE id = ?")
        .bind(comment)
        .bind(id)
        .execute(pool())
        .await?
        .rows_affected();
    Ok(affected > 0)
}

/// Return all wallpapers whose `liked_state` is in the given set.
pub async fn get_wallpapers_by_liked_state(
    states: &[LikedState],
) -> Result<Vec<WallpaperData>, sqlx::Error> {
    if states.is_empty() {
        return Ok(Vec::new());
    }
    let states_json = format!("[{}]", states.iter().map(|s| format!("\"{s}\"")).join(","));
    sqlx::query_as(
        "SELECT * FROM wallpapers
         WHERE liked_state IN (SELECT value FROM json_each(?))
         ORDER BY datetime DESC",
    )
    .bind(states_json)
    .fetch_all(pool())
    .await
}
