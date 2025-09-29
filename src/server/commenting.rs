use crate::{
    common::{CommentData, NetworkPacket, StyleBody, StyleVariant},
    server::{decode_and_verify, gpt, with_db},
};
use axum::{body::Bytes, http::StatusCode};
use chrono::Utc;
use log::error;
use uuid::Uuid;

pub async fn add(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<String> = decode_and_verify(packet).await?;

    with_db(|db| {
        let new_id = Uuid::new_v4();
        db.comments.insert(new_id, CommentData {
            id: new_id,
            datetime: Utc::now(),
            comment: pkt.data,
        });
        Ok(())
    })
    .await?;

    Ok(StatusCode::OK)
}

pub async fn remove(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<Uuid> = decode_and_verify(packet).await?;

    with_db(|db| {
        db.comments.retain(|id, _| *id != pkt.data);
        Ok(())
    })
    .await?;

    Ok(StatusCode::OK)
}

pub async fn styles(packet: Bytes) -> Result<StatusCode, StatusCode> {
    let pkt: NetworkPacket<StyleBody> = decode_and_verify(packet).await?;

    with_db(|db| {
        match pkt.data.variant {
            StyleVariant::Style => &mut db.style.style,
            StyleVariant::Contents => &mut db.style.contents,
            StyleVariant::NegativeContents => &mut db.style.negative_contents,
        }
        .clone_from(&pkt.data.string);
        Ok(())
    })
    .await?;

    Ok(StatusCode::OK)
}

pub async fn query_prompt(packet: Bytes) -> Result<(StatusCode, String), StatusCode> {
    let _: NetworkPacket<()> = decode_and_verify(packet).await?;

    // Query GPT for the prompt it would send to create an image
    let (body, _) = gpt::generate_prompt().await.map_err(|e| {
        error!("gpt error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok((StatusCode::OK, body))
}
