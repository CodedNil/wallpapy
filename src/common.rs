use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct WallpaperData {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,

    pub prompt_data: PromptData,
    pub image_file: ImageFile,
    pub brightness: f32,

    pub liked_state: LikedState,
    pub comment: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ImageFile {
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PromptData {
    pub prompt: String,
    pub shortened_prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, Display, EnumString)]
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
