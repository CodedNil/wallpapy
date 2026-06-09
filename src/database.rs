use crate::common::{ImageFile, LikedState, PromptData, WallpaperData};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{
    QueryBuilder, Row, SqlitePool,
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

fn pool() -> Result<&'static SqlitePool> {
    DB.get()
        .context("Database has not been initialised; call database::init first")
}

fn wallpaper_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<WallpaperData> {
    Ok(WallpaperData {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        datetime: DateTime::parse_from_rfc3339(row.get::<&str, _>("datetime"))?.with_timezone(&Utc),
        prompt_data: PromptData {
            prompt: row.get("prompt"),
            shortened_prompt: row.get("shortened_prompt"),
        },
        image_file: ImageFile {
            file_name: row.get("file_name"),
            width: row.get::<i64, _>("width") as u32,
            height: row.get::<i64, _>("height") as u32,
        },
        brightness: row.get::<f64, _>("brightness") as f32,
        liked_state: row
            .get::<&str, _>("liked_state")
            .parse::<LikedState>()
            .map_err(|e| anyhow::anyhow!("Invalid liked_state: {e}"))?,
        comment: row.get("comment"),
    })
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
pub async fn get_all_wallpapers() -> Result<Vec<WallpaperData>> {
    let rows = sqlx::query(
        "SELECT id, datetime, prompt, shortened_prompt, file_name, width, height, brightness, liked_state, comment
         FROM wallpapers
         ORDER BY datetime DESC",
    )
    .fetch_all(pool()?)
    .await?;

    rows.iter().map(wallpaper_from_row).collect()
}

/// Get wallpapers for the gallery — excludes Disliked.
pub async fn get_gallery_wallpapers() -> Result<Vec<WallpaperData>> {
    let rows = sqlx::query(
        "SELECT id, datetime, prompt, shortened_prompt, file_name, width, height, brightness, liked_state, comment
         FROM wallpapers
         WHERE liked_state != 'Disliked'
         ORDER BY datetime DESC",
    )
    .fetch_all(pool()?)
    .await?;

    rows.iter().map(wallpaper_from_row).collect()
}

/// Get the most recently created wallpaper.
pub async fn get_latest_wallpaper() -> Result<Option<WallpaperData>> {
    let row = sqlx::query(
        "SELECT id, datetime, prompt, shortened_prompt, file_name, width, height, brightness, liked_state, comment
         FROM wallpapers
         ORDER BY datetime DESC
         LIMIT 1",
    )
    .fetch_optional(pool()?)
    .await?;

    row.as_ref().map(wallpaper_from_row).transpose()
}

/// Get the datetime of the most recent wallpaper, or None if no wallpapers exist.
pub async fn get_latest_datetime() -> Result<Option<DateTime<Utc>>> {
    let datetime: Option<String> =
        sqlx::query_scalar("SELECT datetime FROM wallpapers ORDER BY datetime DESC LIMIT 1")
            .fetch_optional(pool()?)
            .await?;

    datetime
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| anyhow::anyhow!("Invalid datetime in database: {e}"))
        })
        .transpose()
}

pub async fn insert_wallpaper(wallpaper: &WallpaperData) -> Result<()> {
    sqlx::query(
        "INSERT INTO wallpapers
            (id, datetime, prompt, shortened_prompt, file_name, width, height, brightness, liked_state, comment)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(wallpaper.id.to_string())
    .bind(wallpaper.datetime.to_rfc3339())
    .bind(&wallpaper.prompt_data.prompt)
    .bind(&wallpaper.prompt_data.shortened_prompt)
    .bind(&wallpaper.image_file.file_name)
    .bind(i64::from(wallpaper.image_file.width))
    .bind(i64::from(wallpaper.image_file.height))
    .bind(f64::from(wallpaper.brightness))
    .bind(wallpaper.liked_state.to_string())
    .bind(&wallpaper.comment)
    .execute(pool()?)
    .await?;
    Ok(())
}

/// Update liked state for a wallpaper. Returns `true` if the row existed.
pub async fn update_liked_state(id: Uuid, state: LikedState) -> Result<bool> {
    let affected = sqlx::query("UPDATE wallpapers SET liked_state = ? WHERE id = ?")
        .bind(state.to_string())
        .bind(id.to_string())
        .execute(pool()?)
        .await?
        .rows_affected();
    Ok(affected > 0)
}

/// When a wallpaper is disliked, remove its file from disk but keep the row so
/// it still appears in prompt context history.
pub async fn dislike_and_remove_file(id: Uuid) -> Result<()> {
    let file_name: Option<String> =
        sqlx::query_scalar("SELECT file_name FROM wallpapers WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool()?)
            .await?;

    // Update state to Disliked and null out image columns.
    sqlx::query(
        "UPDATE wallpapers SET liked_state = 'Disliked', file_name = NULL, width = NULL, height = NULL WHERE id = ?",
    )
    .bind(id.to_string())
    .execute(pool()?)
    .await?;

    // Remove the file from disk.
    if let Some(file_name) = file_name {
        let file_path = WALLPAPERS_DIR.join(&file_name);
        if file_path.exists() {
            tokio::fs::remove_file(file_path).await?;
        }
    }

    Ok(())
}

/// Get the current liked state for a wallpaper.
pub async fn get_liked_state(id: Uuid) -> Result<Option<LikedState>> {
    let state: Option<String> =
        sqlx::query_scalar("SELECT liked_state FROM wallpapers WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool()?)
            .await?;
    state
        .map(|s| {
            s.parse::<LikedState>()
                .map_err(|e| anyhow::anyhow!("Invalid liked_state: {e}"))
        })
        .transpose()
}

/// Update comment for a wallpaper. Returns `true` if the row existed.
pub async fn update_comment(id: Uuid, comment: Option<&str>) -> Result<bool> {
    let affected = sqlx::query("UPDATE wallpapers SET comment = ? WHERE id = ?")
        .bind(comment)
        .bind(id.to_string())
        .execute(pool()?)
        .await?
        .rows_affected();
    Ok(affected > 0)
}

/// Return all wallpapers whose `liked_state` is in the given set.
pub async fn get_wallpapers_by_liked_state(states: &[LikedState]) -> Result<Vec<WallpaperData>> {
    if states.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = QueryBuilder::new(
        "SELECT id, datetime, prompt, shortened_prompt, file_name, width, height, brightness, liked_state, comment
         FROM wallpapers
         WHERE liked_state IN (",
    );
    let mut sep = query.separated(", ");
    for state in states {
        sep.push_bind(state.to_string());
    }
    sep.push_unseparated(")");
    query.push(" ORDER BY datetime DESC");

    let rows = query.build().fetch_all(pool()?).await?;
    rows.iter().map(wallpaper_from_row).collect()
}
