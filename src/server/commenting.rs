use crate::common::{CommentData, TokenStringPacket, TokenUuidPacket};
use crate::server::{auth::verify_token, COMMENTS_TREE, DATABASE_PATH};
use anyhow::Result;
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use time::{format_description, OffsetDateTime};
use uuid::Uuid;

pub async fn add_comment(packet: Bytes) -> impl IntoResponse {
    let packet: TokenStringPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize add_comment packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !matches!(verify_token(&packet.token), Ok(true)) {
        log::error!("Unauthorized add_comment request");
        return StatusCode::UNAUTHORIZED;
    }

    // Store a new database entry
    let result = (|| -> Result<()> {
        let id = Uuid::new_v4();
        let datetime = OffsetDateTime::now_utc();
        let format = format_description::parse("[day]/[month]/[year] [hour]:[minute]")?;
        let datetime_text = datetime.format(&format)?;

        sled::open(DATABASE_PATH)?
            .open_tree(COMMENTS_TREE)?
            .insert(
                id,
                bincode::serialize(&CommentData {
                    id,
                    datetime,
                    datetime_text,
                    comment: packet.string,
                })?,
            )?;

        Ok(())
    })();

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored add_comment {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn remove_comment(packet: Bytes) -> impl IntoResponse {
    let packet: TokenUuidPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize remove_comment packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !matches!(verify_token(&packet.token), Ok(true)) {
        log::error!("Unauthorized remove_comment request");
        return StatusCode::UNAUTHORIZED;
    }

    // Remove the database entry
    let result = (|| -> Result<()> {
        sled::open(DATABASE_PATH)?
            .open_tree(COMMENTS_TREE)?
            .remove(packet.uuid)?;
        Ok(())
    })();

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored remove_comment {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
