use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, VariantNames};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Database {
    pub key_style: String,
    pub wallpapers: HashMap<Uuid, WallpaperData>,
    pub comments: HashMap<Uuid, CommentData>,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptData {
    pub time_of_day: TimeOfDay,
    pub season: Season,
    pub image_mood: Vec<ImageMood>,
    pub color_palette: Vec<ColorPalette>,
    pub subject_matter: Vec<SubjectMatter>,

    pub prompt: String,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Color {
    pub name: String,
    pub rgb_values: [u8; 3],
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum LikedState {
    None,
    Disliked,
    Liked,
    Loved,
}

// Prompt enums
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum TimeOfDay {
    GoldenHour,
    Day,
    Night,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum ImageMood {
    // Positive Moods
    Joyful,
    Hopeful,
    Playful,
    Energetic,
    Triumphant,

    // Neutral Moods
    Reflective,
    Whimsical,
    Luminous,
    Tranquil,

    // Negative Moods
    Melancholic,
    Sombre,
    Tense,
    Foreboding,

    // Dramatic and Mysterious Moods
    Dramatic,
    Mysterious,
    Haunting,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum ColorPalette {
    // Temperature
    Warm,
    Neutral,
    Cool,

    // Style
    Pastel,
    Monochromatic,
    Earthy,
    Neon,
    Sepia,

    // Intensity
    Vibrant,
    Bold,
    Subdued,

    Other,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
    Other,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum SubjectMatter {
    Narrative,
    Historical,
    Symbolic,
    Abstract,

    Landscape,
    Seascape,

    Nature,
    Flora,
    Fauna,

    Fantasy,
    Mythological,
    Surreal,
    Whimsical,
    Celestial,
    Space,

    Cityscape,
    Interior,
    Industrial,
    Technological,
    Architectural,

    Other,
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

#[derive(Serialize, Deserialize, Clone)]
pub enum DatabaseObjectType {
    Wallpaper(WallpaperData),
    Comment(CommentData),
}
