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
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::HashMap, env, error::Error, sync::LazyLock};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

struct LLMSettings {
    model: Model,
}
enum Model {
    Gemini25Flash,
    Gemini25FlashLite,
}
impl Model {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Gemini25Flash => "gemini-2.5-flash",
            Self::Gemini25FlashLite => "gemini-2.5-flash-lite",
        }
    }
}

async fn llm_parse<T>(
    context: Vec<String>,
    settings: LLMSettings,
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
    let system_message = context
        .into_iter()
        .map(|msg| json!({"text": msg}))
        .collect::<Vec<_>>();
    let payload = json!({
        "model": format!("google/{}", settings.model.as_str()),
        "structured_outputs": true,
        "messages": [
            {
                "role": "system",
                "content": system_message
            },
            {
                "role": "user",
                "content": message
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "weather",
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

#[derive(Serialize, Deserialize, JsonSchema)]
struct DiscardedSummary {
    /// Summary of the users loved descriptions, do not include common things like seasons, time of
    /// day etc. do not repeat similar items and err on the side of fewer items, ideally 2-4 word
    /// per item, max 7 words per item if needed
    loved: Vec<String>,
    /// Summary of the users liked descriptions, same rules as for loved
    liked: Vec<String>,
    /// Summary of the users disliked descriptions, same rules as for loved
    disliked: Vec<String>,
    /// Summary of all other descriptions, same rules as for loved
    others: Vec<String>,
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

    let mut history_string = Vec::new();
    let mut discarded: HashMap<LikedState, Vec<String>> = HashMap::new();

    for (i, (_, wallpaper, comment)) in database_history.iter().rev().enumerate() {
        if let Some(wallpaper) = wallpaper {
            if i < match wallpaper.liked_state {
                LikedState::Loved => 60,
                LikedState::Liked | LikedState::Disliked => 30,
                LikedState::Neutral => 20,
            } {
                history_string.push(format!(
                    "{}'{}'",
                    match wallpaper.liked_state {
                        LikedState::Loved => "(user LOVED this) ",
                        LikedState::Liked => "(user liked this) ",
                        LikedState::Disliked => "(user disliked this) ",
                        LikedState::Neutral => "",
                    },
                    wallpaper.prompt_data.shortened_prompt
                ));
            } else if i < 100 {
                discarded
                    .entry(wallpaper.liked_state)
                    .or_default()
                    .push(wallpaper.prompt_data.shortened_prompt.clone());
            }
        }
        if let Some(comment) = comment
            && i < 30
        {
            history_string.push(format!("User commented: '{}'", comment.comment));
        }
    }

    // Use LLM to summarize the discarded string into the key elements
    if discarded.values().any(|v| !v.is_empty()) {
        let summary_text = [
            (LikedState::Loved, "Loved items"),
            (LikedState::Liked, "Liked items"),
            (LikedState::Disliked, "Disliked items"),
            (LikedState::Neutral, "Other items"),
        ]
        .iter()
        .filter_map(|(state, label)| {
            discarded
                .get(state)
                .filter(|items| !items.is_empty())
                .map(|items| format!("{}: {}", label, items.join(", ")))
        })
        .collect::<Vec<_>>()
        .join("\n");
        match llm_parse::<DiscardedSummary>(
            vec![],
            LLMSettings {
                model: Model::Gemini25FlashLite,
            },
            summary_text,
        )
        .await
        {
            Ok(output) => {
                let summary_parts = [
                    &("user LOVED", output.loved),
                    &("user liked", output.liked),
                    &("user disliked", output.disliked),
                    &("others", output.others),
                ]
                .iter()
                .filter_map(|(text, list)| {
                    if list.is_empty() {
                        None
                    } else {
                        Some(format!("({}: {})", text, list.join(", ")))
                    }
                })
                .collect::<Vec<_>>();
                if !summary_parts.is_empty() {
                    history_string.push(format!(
                        "Summary of older history: {}",
                        summary_parts.join(" ")
                    ));
                }
            }
            Err(e) => {
                error!("Failed to parse discarded summary: {e:?}");
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
            "Prioritise users comments as feedback, aim for variety above all else, every image should be totally refreshing with little in common with the previous few.\nThink about this history before responding to avoid repeating previous prompts\nHistory of previous prompts and comments, most recent first (AVOID anything similar to this list):\n{history_string}"
        ),
        format!(
            "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines\nTypes of content to include (not exhaustive just take inspiration) '{}'\nThe overall style direction is '{}' (include the guiding style in every prompt, not exact wording but the meaning)\nNever include anything '{}'",
            style.contents.replace('\n', " "),
            style.style.replace('\n', " "),
            style.negative_contents.replace('\n', " ")
        ),
    ];
    llm_parse::<PromptData>(
        context,
        LLMSettings {
            model: Model::Gemini25Flash,
        },
        format!("Create me a new image prompt, {user_message}\nPrompt:"),
    )
    .await
    .map_err(|err| anyhow!("Failed to generate prompt: {err}"))
}
