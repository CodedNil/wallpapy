use crate::common::{LikedState, WallpaperData};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{
    QueryBuilder, Sqlite, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    env,
    path::{Path, PathBuf},
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

fn sqlite_file_path(database_url: &str) -> Option<&Path> {
    database_url
        .strip_prefix("sqlite://")
        .filter(|path| *path != ":memory:")
        .map(|path| path.split_once('?').map_or(path, |(path, _)| path))
        .map(Path::new)
}

/// Initialise the database pool and run pending migrations.
pub async fn init() -> Result<()> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite://{}", DATA_DIR.join("wallpapy.db").display()));
    if let Some(path) = sqlite_file_path(&database_url)
        && let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create database directory {}", parent.display()))?;
    }
    tokio::fs::create_dir_all(&*WALLPAPERS_DIR)
        .await
        .with_context(|| {
            format!(
                "Failed to create wallpapers directory {}",
                WALLPAPERS_DIR.display()
            )
        })?;

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

#[derive(sqlx::FromRow)]
pub struct PromptContext {
    pub shortened_prompt: String,
    pub liked_state: LikedState,
    pub comment: Option<String>,
}

/// Get recent wallpaper context for prompt generation.
pub async fn get_prompt_context(limit: i64) -> Result<Vec<PromptContext>, sqlx::Error> {
    sqlx::query_as(
        "SELECT shortened_prompt, liked_state, comment
         FROM wallpapers
         ORDER BY datetime DESC
         LIMIT ?",
    )
    .bind(limit)
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

/// Get the most recently created wallpaper image file.
pub async fn get_latest_image_file() -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT image_file FROM wallpapers
         WHERE liked_state != 'Disliked'
         ORDER BY datetime DESC
         LIMIT 1",
    )
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

/// Update liked state for a wallpaper. Returns `true` if the row was updated.
///
/// `Disliked` is terminal: once set, future state changes are ignored. When a
/// wallpaper first becomes disliked, its image file is removed from disk.
pub async fn update_liked_state(id: Uuid, state: LikedState) -> Result<bool> {
    if state == LikedState::Disliked {
        if let Some(file_name) = sqlx::query_scalar::<Sqlite, String>(
            "UPDATE wallpapers
             SET liked_state = ?
             WHERE id = ? AND liked_state != 'Disliked'
             RETURNING image_file",
        )
        .bind(state)
        .bind(id)
        .fetch_optional(pool())
        .await?
        {
            if let Err(e) = tokio::fs::remove_file(WALLPAPERS_DIR.join(file_name)).await
                && e.kind() != std::io::ErrorKind::NotFound
            {
                return Err(e).context("Failed to delete wallpaper file");
            }
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        let affected = sqlx::query(
            "UPDATE wallpapers SET liked_state = ? WHERE id = ? AND liked_state != 'Disliked'",
        )
        .bind(state)
        .bind(id)
        .execute(pool())
        .await?
        .rows_affected();
        Ok(affected > 0)
    }
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

#[derive(sqlx::FromRow)]
pub struct WallpaperChoice {
    pub id: Uuid,
    pub image_file: String,
    pub image_brightness: f32,
}

/// Get image selection data whose `liked_state` is in the given set.
pub async fn get_wallpaper_choices_by_liked_state(
    states: &[LikedState],
) -> Result<Vec<WallpaperChoice>, sqlx::Error> {
    if states.is_empty() {
        return Ok(Vec::new());
    }
    let mut query =
        QueryBuilder::<Sqlite>::new("SELECT id, image_file, image_brightness FROM wallpapers");
    query.push(" WHERE liked_state IN (");
    let mut separated = query.separated(", ");
    for state in states {
        separated.push_bind(*state);
    }
    query.push(") ORDER BY datetime DESC");

    query.build_query_as().fetch_all(pool()).await
}
