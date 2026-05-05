use crate::common::{LikedState, WallpaperData};
use chrono::{Duration, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::{
    database::{WALLPAPERS_DIR, read_database, with_db},
    image::generate_wallpaper_impl,
};
#[cfg(feature = "server")]
use axum::http::StatusCode;

#[cfg(feature = "server")]
const NEW_WALLPAPER_INTERVAL: Duration = Duration::hours(6);

#[cfg(feature = "server")]
pub async fn start_server() {
    loop {
        match read_database().await {
            Ok(database) => {
                let cur_time = Utc::now();
                let latest_time = database
                    .wallpapers
                    .values()
                    .max_by_key(|w| w.datetime)
                    .map_or(cur_time, |w| w.datetime);

                if cur_time - latest_time > NEW_WALLPAPER_INTERVAL
                    && let Err(e) = generate_wallpaper_impl(None, None).await
                {
                    error!("Error generating wallpaper: {e:?}");
                }
            }
            Err(e) => error!("{e:?}"),
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GalleryPageData {
    pub items: Vec<WallpaperData>,
    pub style_prompt: String,
}

#[server]
pub async fn load_gallery_data() -> Result<GalleryPageData, ServerFnError> {
    let db = read_database()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let mut items: Vec<WallpaperData> = db.wallpapers.values().cloned().collect();
    items.sort_by(|a, b| b.datetime.cmp(&a.datetime));
    Ok(GalleryPageData {
        items,
        style_prompt: db.style,
    })
}

#[server]
pub async fn action_generate(prompt: Option<String>) -> Result<(), ServerFnError> {
    let prompt = prompt.filter(|p| !p.trim().is_empty());
    if let Err(e) = generate_wallpaper_impl(None, prompt).await {
        tracing::error!("generate failed: {e:?}");
    }
    Ok(())
}

#[server]
pub async fn action_like(id: Uuid, state: LikedState) -> Result<(), ServerFnError> {
    info!("LIKED TEST: id={id:?}");
    with_db(|db| {
        let Some(wallpaper) = db.wallpapers.get_mut(&id) else {
            error!("Like: wallpaper not found {id}");
            return Err(StatusCode::NOT_FOUND);
        };
        wallpaper.liked_state = if wallpaper.liked_state == state {
            LikedState::Neutral
        } else {
            state
        };
        Ok(())
    })
    .await
    .map_err(|e| ServerFnError::new(format!("like failed: {e:?}")))
}

#[server]
pub async fn action_delete(id: Uuid) -> Result<(), ServerFnError> {
    let files_to_remove = with_db(|db| {
        db.wallpapers
            .remove(&id)
            .map(|w| vec![w.image_file.file_name, w.thumbnail_file.file_name])
            .ok_or(StatusCode::NOT_FOUND)
    })
    .await
    .map_err(|status| ServerFnError::new(format!("Database error: {status}")))?;

    for file_name in files_to_remove {
        let file_path = WALLPAPERS_DIR.join(file_name);
        if file_path.exists() {
            tokio::fs::remove_file(file_path)
                .await
                .map_err(|e| ServerFnError::new(format!("File removal failed: {e}")))?;
        }
    }

    Ok(())
}

#[server]
pub async fn action_recreate(id: Uuid) -> Result<(), ServerFnError> {
    let prompt_data = read_database()
        .await
        .map_err(|e| {
            error!("DB read failed: {e:?}");
            ServerFnError::new("Database read error")
        })?
        .wallpapers
        .get(&id)
        .map(|w| w.prompt_data.clone())
        .ok_or_else(|| {
            error!("Recreate: wallpaper not found {id}");
            ServerFnError::new("Wallpaper not found")
        })?;

    generate_wallpaper_impl(Some(prompt_data), None)
        .await
        .map_err(|e| {
            error!("Failed to recreate image: {e:?}");
            ServerFnError::new("Generation failed")
        })?;

    Ok(())
}

#[server]
pub async fn action_comment(id: Uuid, comment: Option<String>) -> Result<(), ServerFnError> {
    with_db(|db| {
        let Some(wallpaper) = db.wallpapers.get_mut(&id) else {
            error!("set_image_comment: wallpaper not found {id}");
            return Err(StatusCode::NOT_FOUND);
        };
        wallpaper.comment = comment.filter(|s| !s.trim().is_empty());
        Ok(())
    })
    .await
    .map_err(|e| ServerFnError::new(format!("comment failed: {e:?}")))
}

#[server]
pub async fn action_styles(style_prompt: String) -> Result<(), ServerFnError> {
    with_db(|db| {
        db.style = style_prompt;
        Ok(())
    })
    .await
    .map_err(|e| ServerFnError::new(format!("styles failed: {e:?}")))
}
