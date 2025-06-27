use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Database {
    pub style: DatabaseStyle,
    pub wallpapers: HashMap<Uuid, WallpaperData>,
    pub comments: HashMap<Uuid, CommentData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DatabaseStyle {
    pub style: String, // The style that should be included in every prompt, painted etc.
    pub contents: String, // What kind of prompts to create, epic fantasy etc.
    pub negative_contents: String, // What to avoid including in the prompt
}
impl Default for DatabaseStyle {
    fn default() -> Self {
        Self {
            style: "Digital paintings".to_string(),
            contents: "Epic fantasy, surreal, abstract, landscapes".to_string(),
            negative_contents: "No people, don't go for highly complex".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WallpaperData {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,

    pub prompt_data: PromptData,
    pub original_file: ImageFile,
    pub upscaled_file: Option<ImageFile>,
    pub color_data: ColorData,

    pub thumbnail_file: ImageFile,
    pub thumbhash: Vec<u8>,

    pub liked_state: LikedState,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommentData {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,
    pub comment: String,
}

// Sub data types
#[derive(Serialize, Deserialize, Clone)]
pub struct ImageFile {
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct PromptData {
    /// The prompt to send to the image generator
    pub prompt: String,
    /// A shortened version of the prompt, only including the image description not style, aim for 6 words, max 20 words
    pub shortened_prompt: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ColorData {
    pub average_color: (f32, f32, f32),
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
    pub chroma: f32,
    pub top_20_percent_brightness: f32,
    pub bottom_20_percent_brightness: f32,
    pub contrast_ratio: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub enum LikedState {
    Loved,
    Liked,
    Neutral,
    Disliked,
}

// Network packets
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginPacket {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenPacket {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenStringPacket {
    pub token: String,
    pub string: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenUuidPacket {
    pub token: String,
    pub uuid: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct TokenUuidLikedPacket {
    pub token: String,
    pub uuid: Uuid,
    pub liked: LikedState,
}

#[derive(Serialize, Deserialize)]
pub struct SetStylePacket {
    pub token: String,
    pub variant: StyleVariant,
    pub string: String,
}

#[derive(Serialize, Deserialize)]
pub enum StyleVariant {
    Style,
    Contents,
    NegativeContents,
}
