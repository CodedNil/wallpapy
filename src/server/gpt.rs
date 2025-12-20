use crate::{
    common::{Database, DatabaseStyle, LikedState, PromptData},
    server::read_database,
};
use anyhow::{Result, anyhow};
use log::error;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{collections::HashMap, env, error::Error, sync::LazyLock};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

async fn llm_parse<T>(
    context: Vec<String>,
    message: String,
) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: JsonSchema + DeserializeOwned,
{
    // Set up request headers.
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Generate the JSON schema dynamically using `schemars`.
    let mut schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>();
    schema_object.remove("$schema");

    // Create the inputs
    let payload = json!({
        "model": "deepseek/deepseek-v3.2",
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
        "reasoning": {
          "enabled": false
        }
    });

    // Send the request and check for errors
    let response = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions".to_string())
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

    // Parse response JSON and extract inner text.
    let response_json: Value = response.json().await?;
    let inner_text = response_json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    // If serialization fails, return an error including the inner text
    Ok(serde_json::from_str(inner_text)
        .map_err(|e| format!("Serialization failed: {e} - Outputted text: {inner_text}"))?)
}

pub async fn generate_prompt() -> Result<(String, DatabaseStyle)> {
    let database = read_database().await.unwrap_or_else(|e| {
        error!("Failed accessing database {e:?}");
        Database {
            style: DatabaseStyle::default(),
            wallpapers: HashMap::new(),
            comments: HashMap::new(),
        }
    });

    // Collect the images and comments into a single list, sorted by datetime
    let mut database_history = database
        .wallpapers
        .into_values()
        .map(|wallpaper| (wallpaper.datetime, Some(wallpaper), None))
        .chain(
            database
                .comments
                .into_values()
                .map(|comment| (comment.datetime, None, Some(comment))),
        )
        .collect::<Vec<_>>();
    database_history.sort_by_key(|(datetime, ..)| *datetime);

    let mut history_string = Vec::with_capacity(database_history.len().min(100));
    for (i, (_, wallpaper, comment)) in database_history.iter().rev().enumerate() {
        if i < 100 {
            if let Some(wallpaper) = wallpaper {
                history_string.push(format!(
                    "'{}'{}",
                    wallpaper.prompt_data.prompt,
                    match wallpaper.liked_state {
                        LikedState::Loved => " (user LOVED this)",
                        LikedState::Liked => " (user liked this)",
                        LikedState::Disliked => " (user disliked this)",
                        LikedState::Neutral => "",
                    },
                ));
            } else if let Some(comment) = comment {
                history_string.push(format!("User commented: '{}'", comment.comment));
            }
        }
    }
    Ok((history_string.join("\n"), database.style))
}

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let user_message = message.map_or_else(String::new, |message| format!("'User messaged '{message}', this takes precedence over any previous comments and prompts', "));

    let (history_string, style) = generate_prompt().await?;
    let context = vec![
        format!(
            "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines\nPrioritise users comments as feedback, aim for variety above all else, every image should be totally refreshing with little in common with the previous few\nTypes of content to include (not exhaustive just take inspiration) '{}'\nThe overall style direction is '{}' (include the guiding style in every prompt, not exact wording but the meaning)\nNever include anything '{}'",
            style.contents.replace('\n', " "),
            style.style.replace('\n', " "),
            style.negative_contents.replace('\n', " ")
        ),
        format!(
            "Think about this history before responding to avoid repeating previous prompts - history of previous prompts and comments, most recent first (AVOID anything similar to this list):\n{history_string}"
        ),
    ];
    llm_parse::<PromptData>(
        context,
        format!("Create me a new image prompt, {user_message}\nPrompt:"),
    )
    .await
    .map_err(|err| anyhow!("Failed to generate prompt: {err}"))
}
