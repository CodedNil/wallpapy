use crate::common::{GenerationEvent, LikedState, WallpaperData};
use dioxus::{fullstack::ServerEvents, prelude::*};
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::common::GenerationStage;
#[cfg(feature = "server")]
use crate::{
    database,
    image::generate_wallpaper_impl,
    server::{EVENTS_SENDER, GENERATION_EVENTS, remove_generation_event, update_generation_event},
};
#[cfg(feature = "server")]
use dioxus::fullstack::StatusCode;
#[cfg(feature = "server")]
use std::{fmt::Display, time::Duration};
#[cfg(feature = "server")]
use tokio::sync::broadcast;

#[cfg(feature = "server")]
fn server_error(error: impl Display) -> ServerFnError {
    ServerFnError::new(error)
}

#[cfg(feature = "server")]
fn not_found(error: impl Display) -> ServerFnError {
    ServerFnError::ServerError {
        message: error.to_string(),
        code: StatusCode::NOT_FOUND.as_u16(),
        details: None,
    }
}

#[get("/api/gallery")]
pub async fn load_gallery_data() -> Result<Vec<WallpaperData>, ServerFnError> {
    database::get_gallery_wallpapers()
        .await
        .map_err(server_error)
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
            let snapshot = match rx.recv().await {
                Ok(snapshot) => snapshot,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            };
            if tx.send(snapshot).await.is_err() {
                return;
            }
        }
    }))
}

#[post("/api/generate")]
pub async fn action_generate(prompt: Option<String>) -> Result<(), ServerFnError> {
    let prompt = prompt.filter(|p| !p.trim().is_empty());
    let id = Uuid::new_v4();
    if let Err(e) = generate_wallpaper_impl(prompt, id).await {
        error!("Failed to generate wallpaper: {}", e);
        update_generation_event(
            id,
            GenerationStage::Failed {
                reason: e.to_string(),
            },
        )
        .await;

        tokio::time::sleep(Duration::from_secs(5)).await;
        remove_generation_event(id).await;
    }
    Ok(())
}

#[post("/api/wallpapers/{id}/like")]
pub async fn action_like(id: Uuid, state: LikedState) -> Result<(), ServerFnError> {
    let current = database::get_liked_state(id)
        .await
        .map_err(server_error)?
        .ok_or_else(|| not_found("wallpaper not found"))?;

    database::update_liked_state(
        id,
        if current == state {
            LikedState::Neutral
        } else {
            state
        },
    )
    .await
    .map_err(|e| server_error(format!("like failed: {e:?}")))?;

    Ok(())
}

#[post("/api/wallpapers/{id}/comment")]
pub async fn action_comment(id: Uuid, comment: Option<String>) -> Result<(), ServerFnError> {
    let comment = comment.filter(|s| !s.trim().is_empty());
    let existed = database::update_comment(id, comment.as_deref())
        .await
        .map_err(|e| server_error(format!("comment failed: {e:?}")))?;
    if existed {
        Ok(())
    } else {
        tracing::error!("set_image_comment: wallpaper not found {id}");
        Err(not_found("wallpaper not found"))
    }
}
