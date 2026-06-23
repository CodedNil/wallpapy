use crate::{
    common::{GenerationEvent, GenerationStage},
    database,
    image::generate_wallpaper_impl,
};
use chrono::{Duration, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
use tokio::sync::{Mutex, broadcast};
use tracing::error;
use uuid::Uuid;

pub static GENERATION_EVENTS: LazyLock<Arc<Mutex<HashMap<Uuid, GenerationEvent>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static EVENTS_SENDER: LazyLock<broadcast::Sender<Vec<GenerationEvent>>> =
    LazyLock::new(|| broadcast::channel(16).0);

pub async fn update_generation_event(id: Uuid, stage: GenerationStage) {
    let snapshot = {
        let mut events = GENERATION_EVENTS.lock().await;
        if let Some(event) = events.get_mut(&id) {
            event.stage = stage;
        } else {
            events.insert(
                id,
                GenerationEvent {
                    id,
                    start_time: Utc::now(),
                    stage,
                },
            );
        }
        events.values().cloned().collect::<Vec<_>>()
    };
    let _ = EVENTS_SENDER.send(snapshot);
}

pub async fn remove_generation_event(id: Uuid) {
    let snapshot = {
        let mut events = GENERATION_EVENTS.lock().await;
        events.remove(&id);
        events.values().cloned().collect::<Vec<_>>()
    };
    let _ = EVENTS_SENDER.send(snapshot);
}

const NEW_WALLPAPER_INTERVAL: Duration = Duration::hours(12);

async fn try_generate() {
    let id = Uuid::new_v4();
    if let Err(e) = generate_wallpaper_impl(None, id).await {
        error!("generate failed: {e:?}");
        remove_generation_event(id).await;
    }
}

pub async fn start_server() {
    loop {
        match database::get_latest_datetime().await {
            Ok(Some(latest_time)) => {
                if Utc::now() - latest_time > NEW_WALLPAPER_INTERVAL {
                    try_generate().await;
                }
            }
            Ok(None) => try_generate().await,
            Err(e) => error!("{e:?}"),
        }

        tokio::time::sleep(tokio::time::Duration::from_mins(10)).await;
    }
}
