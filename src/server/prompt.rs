use crate::common::{CommentData, DatabaseObjectType, LikedState, WallpaperData};
use crate::server::{COMMENTS_TREE, DATABASE_PATH, IMAGES_TREE};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::env;
use time::format_description;

pub async fn generate() -> Result<String> {
    let database_history = match sled::open(DATABASE_PATH)
        .and_then(|db| Ok((db.clone(), db.open_tree(IMAGES_TREE)?)))
        .and_then(|(db, images_tree)| Ok((images_tree, db.open_tree(COMMENTS_TREE)?)))
    {
        Ok((images_tree, comments_tree)) => {
            let images: Vec<WallpaperData> = images_tree
                .iter()
                .values()
                .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
                .collect();
            let comments: Vec<CommentData> = comments_tree
                .iter()
                .values()
                .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
                .collect();

            // Collect the images and comments into a single list, sorted by datetime
            let mut combined_list = images
                .iter()
                .map(|wallpaper| {
                    (
                        wallpaper.datetime,
                        DatabaseObjectType::Wallpaper(wallpaper.clone()),
                    )
                })
                .chain(comments.iter().map(|comment| {
                    (
                        comment.datetime,
                        DatabaseObjectType::Comment(comment.clone()),
                    )
                }))
                .collect::<Vec<_>>();
            combined_list.sort_by_key(|(datetime, _)| *datetime);
            combined_list
        }
        Err(e) => {
            log::error!("Failed accessing database {:?}", e);
            Vec::new()
        }
    };

    let mut history_string = String::new();
    for (date, data) in database_history {
        let format = format_description::parse("[day]/[month]/[year] [hour]:[minute]").unwrap();
        let datetime_text = date.format(&format).unwrap();
        let mut cur_string = format!("{datetime_text}: ");
        match data {
            DatabaseObjectType::Wallpaper(wallpaper) => {
                cur_string.push_str(&format!(
                    "Wallpaper created with prompt: '{}'",
                    wallpaper.prompt
                ));
                if wallpaper.vote_state != LikedState::None {
                    cur_string.push_str(&format!(
                        " (user {} this)",
                        match wallpaper.vote_state {
                            LikedState::Liked => "liked",
                            LikedState::Disliked => "disliked",
                            LikedState::None => "unknown",
                        }
                    ));
                }
            }
            DatabaseObjectType::Comment(comment) => {
                cur_string.push_str(format!("User commented: '{}'", comment.comment).as_str());
            }
        }
        history_string.push_str(&cur_string);
        history_string.push('\n');
    }

    let client = reqwest::Client::new();

    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in two sentences, works best with simple, short phrases that describe what you want to see. Avoid long lists of requests and instructions. Instead of: 'Show me a picture of lots of blooming California poppies, make them bright, vibrant orange, and draw them in an illustrated style with colored pencils' Try: 'Bright orange California poppies drawn with colored pencils'\nCreate something new and exciting, while respecting the users previous feedback, you can experiment with new themes and styles to keep it fresh."
            },
            {
                "role": "user",
                "content": format!("Create a new image, history of previous prompts and comments:\n{history_string}")
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
