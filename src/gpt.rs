use crate::{common::LikedState, database};
use anyhow::{Result, anyhow};
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{env, error::Error, fmt::Write as _, sync::LazyLock};
use tap::Tap;
use tracing::{error, info};
use uuid::Uuid;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

async fn llm_parse<T>(message: &str) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: DeserializeOwned,
{
    let payload = json!({
        "model": env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "deepseek/deepseek-v4-flash".to_string()),
        "provider": { "only": ["siliconflow/fp8", "atlas-cloud/fp8"] },
        "input": message,
        "reasoning": {
            "enabled": true
        },
    });

    // Write payload to payload.json
    let payload_str = serde_json::to_string_pretty(&payload).unwrap_or_default();
    std::fs::write("target/payload.json", payload_str).ok();

    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/responses")
        .header(CONTENT_TYPE, "application/json")
        .header(
            AUTHORIZATION,
            format!(
                "Bearer {}",
                env::var("OPENROUTER").expect("OPENROUTER not set")
            ),
        )
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!(
            "Request failed {}: {}",
            response.status(),
            response.text().await?
        )
        .into());
    }

    let response_json: Value = response.json().await?;

    // Write output to output.json
    let payload_str = serde_json::to_string_pretty(&response_json).unwrap_or_default();
    std::fs::write("target/output.json", payload_str).ok();

    // Scan through the output array to find the final output text
    let inner_text = response_json["output"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|step| step["content"].as_array())
        .flatten()
        .find(|item| item["type"] == "output_text")
        .and_then(|item| item["text"].as_str())
        .ok_or("Failed to extract output_text from response")?;

    // Get cost of the prompt
    let cost = response_json["usage"]["cost"]
        .as_f64()
        .ok_or("Failed to extract cost from response")?;
    info!(
        "[GENERATION] '{}' - cost: ${}",
        inner_text
            .replace('\n', " ")
            .chars()
            .take(40)
            .collect::<String>(),
        cost,
    );

    serde_json::from_str(inner_text).map_err(|e| {
        format!(
            "Serialization failed: {e} - Output received: {}",
            inner_text
                .replace('\n', " ")
                .chars()
                .take(500)
                .collect::<String>()
        )
        .into()
    })
}

async fn build_context() -> Result<String> {
    let wallpapers = database::get_all_wallpapers()
        .await
        .inspect_err(|e| error!("Failed accessing database {e:?}"))
        .unwrap_or_default();

    let timeline: Vec<(chrono::DateTime<chrono::Utc>, String)> = wallpapers
        .iter()
        .map(|w| {
            let feedback = match w.liked_state {
                LikedState::Loved => " [LOVED by user]",
                LikedState::Liked => " [liked by user]",
                LikedState::Disliked => " [disliked by user]",
                LikedState::Neutral => "",
            };
            let mut entry = format!("• {}{}", w.shortened_prompt, feedback);
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

    Ok(recent)
}

#[derive(Deserialize)]
pub struct PromptData {
    pub prompt: String,
    pub shortened_prompt: String,
}

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let recent_history = build_context().await?;

    let user_note = message
        .as_deref()
        .map(|m| format!("\n\nUser request (prioritise this above all else): {m}"))
        .unwrap_or_default();

    let generation_id = Uuid::new_v4(); // Random uuid to give the prompt some noise
    let context = format!(
        r#"You are a professional digital journalist. Synthesize the provided sources into a high-quality, long-form digital article.

You are a creative wallpaper image prompt generator. Write a vivid, prompt for a desktop wallpaper, don't go heavy on flowery language, keep it simple and descriptive

You MUST return a JSON object with this exact structure:
{{
    "prompt": "The prompt to send to the image generator, aim for 14 words, max 30 words.",
    "shortened_prompt": "A concise version of the prompt, only including the image description not style, aim for 6 words, max 18 words.",
}}

Start directly with the main subject, omitting leading articles. Use sentence case, single line only, no colons, use only commas for punctuation.
Style: Digital paintings (request this in the prompt most of the time, digital painting, oil painting, chalk sketch etc, always specify a style for the artwork in the prompt), colourful, looks great as a desktop wallpaper even when heavily blurred behind apps
No people, avoid high complexity.{user_note}

Generation id: {generation_id}
If user liked an image of a butterfly, don't assume that means they want to see more similar butterfly wallpapers, they already have a butterfly wallpaper now so just take it as learning what style they like. They also might like the composition and colours not just the contents.
RECENT HISTORY (newest first) — a reference for the users taste profile and also to avoid repeating similar prompts:
{recent_history}"#
    );

    llm_parse::<PromptData>(&context)
        .await
        .map_err(|err| anyhow!("Failed to generate prompt: {err}"))
}
