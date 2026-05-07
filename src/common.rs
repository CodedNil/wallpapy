use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Eq)]
pub struct PromptData {
    /// The prompt to send to the image generator, aim for 14 words, max 30 words
    pub prompt: String,
    /// A concise version of the prompt, only including the image description not style, aim for 6 words, max 18 words
    pub shortened_prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
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
