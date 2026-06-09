use crate::common::{GenerationEvent, GenerationStage, LikedState, WallpaperData};
use dioxus::{fullstack::ServerEvents, prelude::*};
use std::time::Duration;
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::{
    database,
    image::generate_wallpaper_impl,
    routing::{EVENTS_SENDER, GENERATION_EVENTS, remove_generation_event, update_generation_event},
};
#[cfg(feature = "server")]
use tokio::sync::broadcast;

#[get("/api/gallery")]
pub async fn load_gallery_data() -> Result<Vec<WallpaperData>, ServerFnError> {
    database::get_gallery_wallpapers()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
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
    if let Err(e) = generate_wallpaper_impl(prompt, id).await {
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
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("wallpaper not found".to_string()))?;

    let new_state = if current == state {
        LikedState::Neutral
    } else {
        state
    };

    if new_state == LikedState::Disliked {
        database::dislike_and_remove_file(id)
            .await
            .map_err(|e| ServerFnError::new(format!("dislike failed: {e:?}")))?;
    } else {
        database::update_liked_state(id, new_state)
            .await
            .map_err(|e| ServerFnError::new(format!("like failed: {e:?}")))?;
    }

    Ok(())
}

#[post("/api/wallpapers/{id}/comment")]
pub async fn action_comment(id: Uuid, comment: Option<String>) -> Result<(), ServerFnError> {
    let comment = comment.filter(|s| !s.trim().is_empty());
    let existed = database::update_comment(id, comment.as_deref())
        .await
        .map_err(|e| ServerFnError::new(format!("comment failed: {e:?}")))?;
    if !existed {
        tracing::error!("set_image_comment: wallpaper not found {id}");
        return Err(ServerFnError::new("wallpaper not found".to_string()));
    }
    Ok(())
}
