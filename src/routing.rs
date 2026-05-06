use crate::{
    common::{GenerationEvent, GenerationStage},
    database::read_database,
    image::generate_wallpaper_impl,
};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
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

const NEW_WALLPAPER_INTERVAL: Duration = Duration::hours(6);

pub async fn start_server() {
    // let id = Uuid::new_v4();
    // tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
    // update_generation_event(id, GenerationStage::WaitingForPrompt).await;
    // tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
    // update_generation_event(
    //     id,
    //     GenerationStage::ReceivedPrompt {
    //         prompt: "test".to_string(),
    //     },
    // )
    // .await;
    // tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
    // update_generation_event(id, GenerationStage::ReceivedImage).await;
    // tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
    // remove_generation_event(id).await;
    loop {
        match read_database().await {
            Ok(database) => {
                let cur_time = Utc::now();
                let latest_time = database
                    .wallpapers
                    .values()
                    .max_by_key(|w| w.datetime)
                    .map_or(cur_time, |w| w.datetime);

                if cur_time - latest_time > NEW_WALLPAPER_INTERVAL {
                    let id = Uuid::new_v4();
                    if let Err(e) = generate_wallpaper_impl(None, None, id).await {
                        tracing::error!("generate failed: {e:?}");
                        remove_generation_event(id).await;
                    }
                }
            }
            Err(e) => error!("{e:?}"),
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
    }
}
