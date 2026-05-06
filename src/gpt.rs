use crate::{
    common::{LikedState, PromptData},
    database::{Database, read_database},
};
use anyhow::{Result, anyhow};
use rand::seq::IndexedRandom;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{env, error::Error, fmt::Write as _, sync::LazyLock};
use tap::Tap;
use tracing::error;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

async fn llm_parse<T>(
    context: Vec<String>,
    message: String,
) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: JsonSchema + DeserializeOwned,
{
    let schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>()
    .tap_mut(|s| {
        s.remove("$schema");
    });

    let payload = json!({
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string()),
        "structured_outputs": true,
        "messages": [
            { "role": "system", "content": context.join("\n\n") },
            { "role": "user",   "content": message }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "schema",
                "strict": true,
                "schema": schema_object
            }
        }
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

    serde_json::from_str(inner_text)
        .map_err(|e| format!("Serialization failed: {e} - Outputted text: {inner_text}").into())
}

/// Returns `(recent_timeline, liked_examples)` strings for use in prompt context.
async fn build_context() -> Result<(String, String, String)> {
    let database = read_database()
        .await
        .inspect_err(|e| error!("Failed accessing database {e:?}"))
        .unwrap_or_default();

    let Database {
        style: style_prompt,
        wallpapers,
    } = database;
    let wallpapers: Vec<_> = wallpapers.into_values().collect();

    let timeline: Vec<(chrono::DateTime<chrono::Utc>, String)> = wallpapers
        .iter()
        .map(|w| {
            let feedback = match w.liked_state {
                LikedState::Loved => " [LOVED]",
                LikedState::Liked => " [liked]",
                LikedState::Disliked => " [disliked]",
                LikedState::Neutral => "",
            };
            let mut entry = format!("• {}{}", w.prompt_data.shortened_prompt, feedback);
            if let Some(note) = &w.comment {
                let _ = write!(entry, "  user note: {note}");
            }
            (w.datetime, entry)
        })
        .collect::<Vec<_>>()
        .tap_mut(|t| t.sort_by_key(|(dt, _)| *dt));

    let recent: String = timeline
        .iter()
        .rev()
        .take(50)
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let liked: Vec<String> = wallpapers
        .iter()
        .filter(|w| matches!(w.liked_state, LikedState::Loved | LikedState::Liked))
        .map(|w| {
            let label = if w.liked_state == LikedState::Loved {
                "LOVED"
            } else {
                "liked"
            };
            format!("• {} [{label}]", w.prompt_data.shortened_prompt)
        })
        .collect::<Vec<_>>()
        .tap_mut(|v| v.sort_unstable());

    Ok((recent, liked.join("\n"), style_prompt))
}

const NOUN_POOL: &str = include_str!("nounlist.txt");

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let (recent_history, liked_examples, style_prompt) = build_context().await?;

    let user_note = message
        .as_deref()
        .map(|m| format!("\n\nUser request (prioritise this above all else): {m}"))
        .unwrap_or_default();

    let nouns = {
        let mut rng = rand::rng();
        NOUN_POOL
            .lines()
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .as_slice()
            .sample(&mut rng, 20)
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    };

    let mut context = vec![
        format!(
            "You are a creative wallpaper image prompt generator. Write a vivid, detailed prompt for a desktop wallpaper\n
             Start directly with the main subject, omitting leading articles. Use sentence case, single line only, no colons, use only commas for punctuation.\n
             \n
             {style_prompt}
             \n
             To keep every wallpaper feeling completely fresh, aim for a design utterly unique to anything seen recently. Here are a few random nouns to inspire (absolutely don't need to use any of them): [{nouns}]"
        ),
        format!(
            "RECENT HISTORY (newest first) — the subject, setting, and mood of each must NOT be repeated: [{recent_history}]"
        ),
    ];

    if !liked_examples.is_empty() {
        context.push(format!(
            "QUALITY REFERENCE — the user loved/liked these. Aim for this level of quality and evocativeness, but choose a completely different subject and setting:\n{liked_examples}"
        ));
    }

    llm_parse::<PromptData>(
        context,
        format!(
            "Generate a wallpaper prompt with a subject, setting, and mood that does not appear in the recent history above.{user_note}"
        ),
    )
    .await
    .map_err(|err| anyhow!("Failed to generate prompt: {err}"))
}
