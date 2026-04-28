use crate::{
    common::{DatabaseStyle, LikedState, PromptData},
    server::read_database,
};
use anyhow::{Result, anyhow};
use log::error;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error, sync::LazyLock};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

async fn llm_parse<T>(
    context: Vec<String>,
    message: String,
) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: JsonSchema + DeserializeOwned,
{
    let mut schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>();
    schema_object.remove("$schema");

    let payload = json!({
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "openai/gpt-oss-120b".to_string()),
        "structured_outputs": true,
        "messages": [
            {
                "role": "system",
                "content": context.join("\n\n")
            },
            {
                "role": "user",
                "content": message
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "schema",
                "strict": true,
                "schema": schema_object
            }
        },
    });

    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header(CONTENT_TYPE, "application/json")
        .header(
            AUTHORIZATION,
            &format!(
                "Bearer {}",
                env::var("OPENROUTER").expect("OPENROUTER not set")
            ),
        )
        .json(&payload)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!(
            "Request failed with status {}: {}",
            response.status(),
            response.text().await?
        )
        .into());
    }

    let response_json: Value = response.json().await?;
    let inner_text = response_json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    Ok(serde_json::from_str(inner_text)
        .map_err(|e| format!("Serialization failed: {e} - Outputted text: {inner_text}"))?)
}

pub async fn generate_prompt() -> Result<(String, String, DatabaseStyle)> {
    let database = read_database()
        .await
        .inspect_err(|e| error!("Failed accessing database {e:?}"))
        .unwrap_or_default();

    let wallpapers: Vec<_> = database.wallpapers.into_values().collect();
    let comments: Vec<_> = database.comments.into_values().collect();

    // Build a unified chronological timeline of wallpapers and comments.
    let mut timeline: Vec<(
        chrono::DateTime<chrono::Utc>,
        Option<&crate::common::WallpaperData>,
        Option<&crate::common::CommentData>,
    )> = wallpapers
        .iter()
        .map(|w| (w.datetime, Some(w), None))
        .chain(comments.iter().map(|c| (c.datetime, None, Some(c))))
        .collect();
    timeline.sort_by_key(|(dt, ..)| *dt);

    let recent: Vec<String> = timeline
        .iter()
        .rev()
        .take(50)
        .filter_map(|(_, wallpaper, comment)| {
            wallpaper.as_ref().map_or_else(
                || {
                    comment
                        .as_ref()
                        .map(|c| format!("• [user comment] {}", c.comment))
                },
                |w| {
                    let feedback = match w.liked_state {
                        LikedState::Loved => " [LOVED]",
                        LikedState::Liked => " [liked]",
                        LikedState::Disliked => " [disliked]",
                        LikedState::Neutral => "",
                    };
                    Some(format!("• {}{}", w.prompt_data.shortened_prompt, feedback))
                },
            )
        })
        .collect();

    // Loved/liked entries for quality reference — any age.
    let mut liked: Vec<String> = wallpapers
        .iter()
        .filter(|w| matches!(w.liked_state, LikedState::Loved | LikedState::Liked))
        .map(|w| {
            let label = if w.liked_state == LikedState::Loved {
                "LOVED"
            } else {
                "liked"
            };
            format!("• {} [{}]", w.prompt_data.shortened_prompt, label)
        })
        .collect();
    liked.sort_unstable();

    Ok((recent.join("\n"), liked.join("\n"), database.style))
}

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let (recent_history, liked_examples, style) = generate_prompt().await?;

    let user_note = message
        .as_deref()
        .map(|m| format!("\n\nUser request (prioritise this above all else): {m}"))
        .unwrap_or_default();

    let mut context = vec![
        format!(
            "You are a creative wallpaper image prompt generator. \
            Write a vivid, detailed prompt for a desktop wallpaper in a few sentences, no newlines.\n\
            \n\
            Style direction: '{}' — weave this naturally into every prompt.\n\
            Content categories (inspiration only, not exhaustive): '{}'.\n\
            Never include: '{}'.\n\
            \n\
            To keep every wallpaper feeling completely fresh, aim for a design utterly unique to anything seen recently.",
            style.style.replace('\n', " "),
            style.contents.replace('\n', " "),
            style.negative_contents.replace('\n', " "),
        ),
        format!(
            "RECENT HISTORY (newest first) — the subject, setting, and mood of each must NOT be repeated:\n{recent_history}"
        ),
    ];

    if !liked_examples.is_empty() {
        context.push(format!(
            "QUALITY REFERENCE — the user loved/liked these. \
            Aim for this level of quality and evocativeness, but choose a completely different subject and setting:\n{liked_examples}"
        ));
    }

    llm_parse::<PromptData>(
        context,
        format!("Generate a wallpaper prompt with a subject, setting, and mood that does not appear in the recent history above.{user_note}"),
    )
    .await
    .map_err(|err| anyhow!("Failed to generate prompt: {err}"))
}
