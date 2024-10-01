use crate::common::{CommentData, TokenStringPacket, TokenUuidPacket};
use crate::server::{auth::verify_token, gpt, read_database, write_database};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use uuid::Uuid;

pub async fn add(packet: Bytes) -> impl IntoResponse {
    let packet: TokenStringPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
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
    let packet: TokenUuidPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
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

pub async fn key_style(packet: Bytes) -> impl IntoResponse {
    let packet: TokenStringPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize key_style packet: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    if !verify_token(&packet.token).await.unwrap_or(false) {
        return StatusCode::UNAUTHORIZED;
    }

    let result = async {
        let mut database = read_database().await?;
        database.key_style = packet.string;
        write_database(&database).await
    }
    .await;

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            log::error!("Errored key_style {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn query_prompt(packet: Bytes) -> impl IntoResponse {
    let packet: TokenStringPacket = match bincode::deserialize(&packet) {
        Ok(packet) => packet,
        Err(e) => {
            log::error!("Failed to deserialize query_prompt packet: {:?}", e);
            return (StatusCode::BAD_REQUEST, String::new());
        }
    };
    if !verify_token(&packet.token).await.unwrap_or(false) {
        return (StatusCode::UNAUTHORIZED, String::new());
    }

    // Query GPT for the prompt it would send to create an image
    let generate_result = gpt::generate_prompt(
        &reqwest::Client::new(),
        &std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set"),
        if packet.string.is_empty() {
            None
        } else {
            Some(packet.string)
        },
    )
    .await;
    match generate_result {
        Ok((request_body, _)) => (
            StatusCode::OK,
            serde_json::to_string_pretty(&request_body).unwrap(),
        ),
        Err(e) => {
            log::error!("Errored query_prompt {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, String::new())
        }
    }
}
