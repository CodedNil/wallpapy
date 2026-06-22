use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct WallpaperData {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,

    pub prompt: String,
    pub shortened_prompt: String,

    pub image_file: String,
    pub image_width: u32,
    pub image_height: u32,
    pub image_brightness: f32,

    pub liked_state: LikedState,
    pub comment: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, Display, EnumString)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
pub enum LikedState {
    Loved,
    Liked,
    Neutral,
    Disliked,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum GenerationStage {
    WaitingForPrompt,
    ReceivedPrompt { prompt: String },
    ReceivedImage,
    Failed { reason: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenerationEvent {
    pub id: Uuid,
    pub start_time: DateTime<Utc>,
    pub stage: GenerationStage,
}
