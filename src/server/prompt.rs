use anyhow::{anyhow, Result};
use serde_json::json;
use std::env;

pub async fn generate() -> Result<String> {
    let client = reqwest::Client::new();

    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": "Prompt for an image for a wallpaper in two sentences"
            }
        ],
        "max_tokens": 4096
    });
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;

    let response_json: serde_json::Value = response.json().await?;
    response_json["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map_or_else(
            || Err(anyhow!("No content found in response")),
            |content| Ok(content.to_string()),
        )
}
