use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct WallpaperData {
    pub id: Uuid,

    pub datetime: OffsetDateTime,
    pub datetime_text: String,

    pub prompt: String,
    pub prompt_short: String,
    pub original_file: ImageFile,
    pub upscaled_file: Option<ImageFile>,

    pub average_color: (u8, u8, u8),
    pub brightness: u8,

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

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageFile {
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct GetWallpapersResponse {
    pub images: Vec<WallpaperData>,
    pub comments: Vec<CommentData>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum LikedState {
    None,
    Liked,
    Disliked,
}
