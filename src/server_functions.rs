use crate::common::{GenerationEvent, LikedState, WallpaperData};
use dioxus::fullstack::payloads::ServerEvents;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::{
    database::{WALLPAPERS_DIR, read_database, with_db},
    image::generate_wallpaper_impl,
    routing::{EVENTS_SENDER, GENERATION_EVENTS, remove_generation_event},
};
#[cfg(feature = "server")]
use axum::http::StatusCode;
#[cfg(feature = "server")]
use tokio::sync::broadcast;

#[derive(Serialize, Deserialize, Clone)]
pub struct GalleryPageData {
    pub items: Vec<WallpaperData>,
    pub style_prompt: String,
}

#[get("/api/gallery")]
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

#[get("/api/events")]
pub async fn stream_generation_events() -> Result<ServerEvents<Vec<GenerationEvent>>, ServerFnError>
{
    let mut rx = EVENTS_SENDER.subscribe();
    let initial: Vec<GenerationEvent> = GENERATION_EVENTS.lock().await.values().cloned().collect();

    Ok(ServerEvents::new(move |mut tx| async move {
        if tx.send(initial).await.is_err() {
            return;
        }
        loop {
            match rx.recv().await {
                Ok(snapshot) => {
                    if tx.send(snapshot).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }))
}

#[post("/api/generate")]
pub async fn action_generate(prompt: Option<String>) -> Result<(), ServerFnError> {
    let prompt = prompt.filter(|p| !p.trim().is_empty());
    let id = Uuid::new_v4();
    if let Err(e) = generate_wallpaper_impl(None, prompt, id).await {
        error!("generate failed: {e:?}");
        remove_generation_event(id).await;
    }
    Ok(())
}

#[post("/api/wallpapers/{id}/like")]
pub async fn action_like(id: Uuid, state: LikedState) -> Result<(), ServerFnError> {
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

#[delete("/api/wallpapers/{id}")]
pub async fn action_delete(id: Uuid) -> Result<(), ServerFnError> {
    let file_to_remove = with_db(|db| {
        db.wallpapers
            .remove(&id)
            .map(|w| w.image_file.file_name)
            .ok_or(StatusCode::NOT_FOUND)
    })
    .await
    .map_err(|status| ServerFnError::new(format!("Database error: {status}")))?;

    let file_path = WALLPAPERS_DIR.join(file_to_remove);
    if file_path.exists() {
        tokio::fs::remove_file(file_path)
            .await
            .map_err(|e| ServerFnError::new(format!("File removal failed: {e}")))?;
    }
    Ok(())
}

#[post("/api/wallpapers/{id}/recreate")]
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

    let id = Uuid::new_v4();
    tokio::spawn(async move {
        if let Err(e) = generate_wallpaper_impl(Some(prompt_data), None, id).await {
            error!("Failed to recreate image: {e:?}");
            remove_generation_event(id).await;
        }
    });
    Ok(())
}

#[post("/api/wallpapers/{id}/comment")]
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

#[post("/api/styles")]
pub async fn action_styles(style_prompt: String) -> Result<(), ServerFnError> {
    with_db(|db| {
        db.style = style_prompt;
        Ok(())
    })
    .await
    .map_err(|e| ServerFnError::new(format!("styles failed: {e:?}")))
}
