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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WallpaperData {
    pub id: Uuid,
    pub datetime: OffsetDateTime,
    pub prompt: String,
    pub file_name: String,
    pub width: u32,
    pub height: u32,
    pub thumbhash: Vec<u8>,
    pub vote_state: LikedState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentData {
    pub id: Uuid,
    pub datetime: OffsetDateTime,
    pub comment: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetWallpapersResponse {
    pub images: Vec<WallpaperData>,
    pub comments: Vec<CommentData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LikedState {
    None,
    Liked,
    Disliked,
}
