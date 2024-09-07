use serde::{Deserialize, Serialize};
use strum_macros::{Display, VariantNames};
use time::OffsetDateTime;
use uuid::Uuid;

pub mod utils;

#[derive(Serialize, Deserialize, Clone)]
pub struct WallpaperData {
    pub id: Uuid,

    pub datetime: OffsetDateTime,
    pub datetime_text: String,

    pub prompt_data: PromptData,
    pub vision_data: VisionData,

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

    pub datetime: OffsetDateTime,
    pub datetime_text: String,

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
    pub prompt: String,
    pub shortened_prompt: String,
    pub style: String,

    pub tags: Vec<String>,

    pub time_of_day: TimeOfDay,
    pub season: Season,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VisionData {
    pub primary_color: String,
    pub primary_color_rgb: [u8; 3],
    pub secondary_color: String,
    pub secondary_color_rgb: [u8; 3],
    pub tertiary_color: String,
    pub tertiary_color_rgb: [u8; 3],
    pub brightness: Brightness,

    pub what_worked_well: String,
    pub what_didnt_work: String,
    pub differences_from_prompt: String,
    pub how_to_improve: String,

    pub time_of_day: TimeOfDay,
    pub season: Season,
    pub weather: Vec<Weather>,

    pub tags: Vec<String>,
    pub image_mood: Vec<ImageMood>,
    pub color_palette: Vec<ColorPalette>,
    pub subject_matter: Vec<SubjectMatter>,
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum LikedState {
    None,
    Disliked,
    Liked,
    Loved,
}

// Prompt enums
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum Brightness {
    Dark,
    Dim,
    Normal,
    Bright,
    Light,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum TimeOfDay {
    Dawn,
    Sunrise,
    Morning,
    Midday,
    Afternoon,
    Sunset,
    Dusk,
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
    Aquatic,

    // Intensity
    Vibrant,
    Bold,
    Subdued,

    Other,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum Season {
    EarlySpring,
    MidSpring,
    LateSpring,
    EarlySummer,
    MidSummer,
    LateSummer,
    EarlyAutumn,
    MidAutumn,
    LateAutumn,
    EarlyWinter,
    MidWinter,
    LateWinter,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, VariantNames, Display)]
pub enum Weather {
    // Clear and Calm
    Clear,

    // Cloudy and Overcast
    Overcast,
    Foggy,
    Mist,

    // Rain and Storms
    Drizzle,
    Rainy,
    TorrentialRain,
    Thunderstorm,

    // Winter Weather
    Snowy,
    Blizzard,
    Sleet,
    Frost,

    // Severe Weather
    Hail,
    Hurricane,
    Tornado,
    IceStorm,
    DustStorm,
    Sandstorm,

    // Wind and Humidity
    Windy,
    Humid,
    Dew,

    // Special Phenomena
    Rainbow,
    Aurora,
    Ethereal,
    Magical,
    Lightning,
    MeteorShower,
    Other,
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

#[derive(Serialize, Deserialize)]
pub struct GetWallpapersResponse {
    pub images: Vec<WallpaperData>,
    pub comments: Vec<CommentData>,
}
