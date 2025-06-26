use crate::common::{
    CommentData, SetStylePacket, StyleVariant, TokenPacket, TokenStringPacket, TokenUuidPacket,
};
use crate::server::{auth::verify_token, gpt, read_database, write_database};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use bincode::serde::decode_from_slice;
use chrono::Utc;
use uuid::Uuid;

pub async fn add(packet: Bytes) -> impl IntoResponse {
    let packet: TokenStringPacket = match decode_from_slice(&packet, bincode::config::standard()) {
        Ok((packet, _)) => packet,
        Err(e) => {
            log::error!("Failed to deserialize add_comment packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !verify_token(&packet.token).await.unwrap_or(false) {
        return StatusCode::UNAUTHORIZED;
    }

    // Store a new database entry
    let result = async {
        let mut database = read_database().await?;
        let id = Uuid::new_v4();
        let datetime = Utc::now();

        database.comments.insert(
            id,
            CommentData {
                id,
                datetime,
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

pub async fn remove(packet: Bytes) -> impl IntoResponse {
    let packet: TokenUuidPacket = match decode_from_slice(&packet, bincode::config::standard()) {
        Ok((packet, _)) => packet,
        Err(e) => {
            log::error!("Failed to deserialize remove_comment packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !verify_token(&packet.token).await.unwrap_or(false) {
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

pub async fn styles(packet: Bytes) -> impl IntoResponse {
    let packet: SetStylePacket = match decode_from_slice(&packet, bincode::config::standard()) {
        Ok((packet, _)) => packet,
        Err(e) => {
            log::error!("Failed to deserialize styles packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !verify_token(&packet.token).await.unwrap_or(false) {
        return StatusCode::UNAUTHORIZED;
    }

    let result = async {
        let mut database = read_database().await?;
        match packet.variant {
            StyleVariant::Style => {
                database.style.style = packet.string;
            }
            StyleVariant::Contents => {
                database.style.contents = packet.string;
            }
            StyleVariant::NegativeContents => {
                database.style.negative_contents = packet.string;
            }
        }
        write_database(&database).await
    }
    .await;

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored styles {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn query_prompt(packet: Bytes) -> impl IntoResponse {
    let packet: TokenPacket = match decode_from_slice(&packet, bincode::config::standard()) {
        Ok((packet, _)) => packet,
        Err(e) => {
            log::error!("Failed to deserialize query_prompt packet: {:?}", e);
            return (StatusCode::BAD_REQUEST, String::new());
        }
    };
    if !verify_token(&packet.token).await.unwrap_or(false) {
        return (StatusCode::UNAUTHORIZED, String::new());
    }

    // Query GPT for the prompt it would send to create an image
    let generate_result = gpt::generate_prompt().await;
    match generate_result {
        Ok((request_body, _)) => (StatusCode::OK, request_body),
        Err(e) => {
            log::error!("Errored query_prompt {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, String::new())
        }
    }
}
