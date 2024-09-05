use crate::common::{CommentData, TokenStringPacket, TokenUuidPacket};
use crate::server::{auth::verify_token, read_database, write_database};
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
    if !matches!(verify_token(&packet.token).await, Ok(true)) {
        log::error!("Unauthorized add_comment request");
        return StatusCode::UNAUTHORIZED;
    }

    // Store a new database entry
    let result = async {
        let mut database = read_database().await?;
        let id = Uuid::new_v4();
        let datetime = OffsetDateTime::now_utc();
        let format = format_description::parse("[day]/[month]/[year] [hour]:[minute]")?;
        let datetime_text = datetime.format(&format)?;

        database.comments.insert(
            id,
            CommentData {
                id,
                datetime,
                datetime_text,
                comment: packet.string,
            },
        );

        write_database(&database).await
    }
    .await;

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
    if !matches!(verify_token(&packet.token).await, Ok(true)) {
        log::error!("Unauthorized remove_comment request");
        return StatusCode::UNAUTHORIZED;
    }

    // Remove the database entry
    let result = async {
        let mut database = read_database().await?;
        database.comments.retain(|id, _| *id != packet.uuid);
        write_database(&database).await
    }
    .await;

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored remove_comment {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
