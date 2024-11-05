use crate::common::{Database, DatabaseStyle, LikedState, PromptData};
use crate::server::{format_duration, read_database};
use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

const PROMPT_GUIDELINES: &str = "A well-crafted FLUX.1 prompt typically includes the following components:
    Subject: The main focus of the image.
    Style: The artistic approach or visual aesthetic.
    Composition: How elements are arranged within the frame.
    Lighting: The type and quality of light in the scene.
    Color Palette: The dominant colors or color scheme.
    Mood/Atmosphere: The emotional tone or ambiance of the image.
    Technical Details: Camera settings, perspective, or specific visual techniques.
    Additional Elements: Supporting details or background information.

Prompt Crafting Techniques

FLUX.1 thrives on detailed information. Instead of vague descriptions, provide specific details about your subject and scene.

Poor: “A portrait of a woman”
Better: “A close-up portrait of a middle-aged woman with curly red hair, green eyes, and freckles, wearing a blue silk blouse”

Example Prompt: A hyperrealistic portrait of a weathered sailor in his 60s, with deep-set blue eyes, a salt-and-pepper beard, and sun-weathered skin. He’s wearing a faded blue captain’s hat and a thick wool sweater. The background shows a misty harbor at dawn, with fishing boats barely visible in the distance.

Specify Technical Details
    Including camera settings, angles, and other technical aspects can significantly influence the final image.
    Example Prompt: Capture a street food vendor in Tokyo at night, shot with a wide-angle lens (24mm) at f/1.8. Use a shallow depth of field to focus on the vendor’s hands preparing takoyaki, with the glowing street signs and bustling crowd blurred in the background. High ISO setting to capture the ambient light, giving the image a slight grain for a cinematic feel.

Use Contrast and Juxtaposition
    Creating contrast within your prompt can lead to visually striking and thought-provoking images.
    Example Prompt: Create an image that juxtaposes the delicate beauty of nature with the harsh reality of urban decay. Show a vibrant cherry blossom tree in full bloom growing out of a cracked concrete sidewalk in a dilapidated city alley. The tree should be the focal point, with its pink petals contrasting against the gray, graffiti-covered walls of surrounding buildings. Include a small bird perched on one of the branches to emphasize the theme of resilience.

Incorporate Mood and Atmosphere
    Describing the emotional tone or atmosphere can help FLUX.1 generate images with the desired feel.
    Example Prompt: Depict a cozy, warmly lit bookstore cafe on a rainy evening. The atmosphere should be inviting and nostalgic, with soft yellow lighting from vintage lamps illuminating rows of well-worn books. Show patrons reading in comfortable armchairs, steam rising from their coffee cups. The large front window should reveal a glistening wet street outside, with blurred lights from passing cars. Emphasize the contrast between the warm interior and the cool, rainy exterior.


Tips for Optimal Results
    Iterate and Refine: Don’t be afraid to generate multiple images and refine your prompt based on the results.
    Balance Detail and Freedom: While specific details can guide FLUX.1, leaving some aspects open to interpretation can lead to surprising and creative results.
    Use Natural Language: FLUX.1 understands natural language, so write your prompts in a clear, descriptive manner rather than using keyword-heavy language.
    Explore Diverse Themes: FLUX.1 has a broad knowledge base, so don’t hesitate to explore various subjects, from historical scenes to futuristic concepts.
    Leverage Technical Terms: When appropriate, use photography, art, or design terminology to guide the image creation process.
    Consider Emotional Impact: Think about the feeling or message you want to convey and incorporate emotional cues into your prompt.

Common Pitfalls to Avoid
    Overloading the Prompt: While FLUX.1 can handle complex prompts, overloading with too many conflicting ideas can lead to confused outputs.
    Neglecting Composition: Don’t forget to guide the overall composition of the image, not just individual elements.
    Ignoring Lighting and Atmosphere: These elements greatly influence the mood and realism of the generated image.
    Being Too Vague: Extremely general prompts may lead to generic or unpredictable results.
    Forgetting About Style: Unless specified, FLUX.1 may default to a realistic style. Always indicate if you want a particular artistic approach.


Examples

A captivating abstract portrait of a woman's face that artfully blends her features with a nighttime forest landscape filled with fireflies. The eyes, nose, and lips stand out in vivid contrast to the surrounding colors, creating a mesmerizing interplay of deep blues, purples, and vibrant hues of red, green, and yellow. The silhouette of the face, with its intense and evocative mood, is enhanced by the dynamic, chaotic environment in the background. This masterful piece combines elements of wildlife photography, illustration, painting, and conceptual art, evoking a powerful sense of emotion and passion., conceptual art, graffiti, wildlife photography, dark fantasy, painting, vibrant, illustration

A visually striking dark fantasy portrait of a majestic horse galloping through a stormy, fiery landscape. The horse's glossy black coat is a stark contrast to its vivid, flame-like mane and tail, which seems to be made of real fire. Its glowing, fiery hooves leave a trail of embers behind, while its intense, glistening eyes reflect a fierce, unbridled energy. The background features a haunting, stormy red sky filled with ominous lightning, adding to the overall sense of mystique and intrigue. This captivating image blends the mediums of photo, painting, and portrait photography to create a unique, conceptual art piece., painting, portrait photography, vibrant, photo, conceptual art, dark fantasy

Blend the surrealism of Salvador Dalí with the geometric abstraction of Piet Mondrian to depict a melting cityscape. Use Dalí's soft, drooping forms for skyscrapers that are liquefying, but render them in Mondrian's characteristic primary colors and black grid lines. The sky should be divided into rectangles of different shades of blue and white, with a few of Dalí's signature clouds scattered about

Create a split-screen image. On the left, show a extreme close-up of a human eye, with intricate details of the iris visible. On the right, depict a vast spiral galaxy. The colors and patterns of the iris should mirror the structure of the galaxy, implying a connection between the micro and macro scales. Use a rich color palette with deep blues, purples, and flecks of gold in both halves of the image

Illustrate the four seasons of a single landscape in one continuous panoramic image. From left to right, transition from winter to spring to summer to fall. Start with a snow-covered forest, then show the same trees budding with new leaves, followed by lush summer growth, and ending with autumn colors. Include a small cabin that remains constant throughout, but show how its surroundings change. Subtly alter the lighting from cool winter tones to warm summer hues and back to the golden light of fall

Create an abstract representation of the emotional journey from depression to hope using only colors, shapes, and textures. Start from the left with dark, heavy shapes in blues and greys, gradually transitioning to lighter, more vibrant colors on the right. Use rough, jagged textures on the left, slowly morphing into smoother, flowing forms. End with bright yellows and soft, rounded shapes. Don't include any recognizable objects or figures – focus solely on the emotive power of abstract visual elements

Create a surreal, ethereal dreamscape with floating islands, bioluminescent plants, and a sky filled with multiple moons of different colors. Include a solitary figure on one of the islands, gazing at the celestial display

Design a mythical creature that combines elements of a lion, an eagle, and a dragon. Place this creature in a majestic mountain setting with a dramatic sunset in the background

Create an abstract representation of the emotion 'hope' using a palette of warm colors. Incorporate flowing shapes and subtle human silhouettes to suggest a sense of movement and aspiration
";

pub async fn generate_prompt(client: &Client, api_key: &str) -> Result<(String, DatabaseStyle)> {
    // Read the database
    let database = match read_database().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("Failed accessing database {:?}", e);
            Database {
                style: DatabaseStyle::default(),
                wallpapers: HashMap::new(),
                comments: HashMap::new(),
            }
        }
    };

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
    database_history.sort_by_key(|(datetime, _, _)| *datetime);

    let cur_time = Utc::now();
    let mut history_string = Vec::new();
    let (mut discarded_loves, mut discarded_likes, mut discarded_dislikes, mut discarded_others) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (i, (date, wallpaper, comment)) in database_history.iter().rev().enumerate() {
        let datetime_text = format_duration(cur_time - date);
        if let Some(wallpaper) = wallpaper {
            if i < match wallpaper.liked_state {
                LikedState::Loved => 30,
                LikedState::Liked | LikedState::Disliked => 15,
                LikedState::Neutral => 10,
            } {
                history_string.push(format!(
                    "{datetime_text} ago -{} '{}'",
                    match wallpaper.liked_state {
                        LikedState::Loved => " (user LOVED this)",
                        LikedState::Liked => " (user liked this)",
                        LikedState::Disliked => " (user disliked this)",
                        LikedState::Neutral => "",
                    },
                    wallpaper.prompt_data.shortened_prompt
                ));
            } else if i < 60 {
                let text = wallpaper.prompt_data.shortened_prompt.clone();
                match wallpaper.liked_state {
                    LikedState::Loved => {
                        discarded_loves.push(text);
                    }
                    LikedState::Liked => {
                        discarded_likes.push(text);
                    }
                    LikedState::Disliked => {
                        discarded_dislikes.push(text);
                    }
                    LikedState::Neutral => {
                        discarded_others.push(text);
                    }
                }
            }
        }
        if let Some(comment) = comment {
            if i < 10 {
                history_string.push(format!(
                    "{datetime_text} - User commented: '{}'",
                    comment.comment
                ));
            }
        }
    }

    // Use gpt mini to summarise the discarded string into the key elements
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": format!(
                    "Summarise this history of image descriptions, taking out just the key concepts to create 3 comma separated lists of them without new lines, do not include common things like seasons, time of day etc, do not repeat similar items and err on the side of fewer items, ideally 1 word per item, max 3 words per item if needed\nExample output: (user LOVED: item, item) (user liked: item, item, item) (user disliked: item, item) (others: item, item)\n\nLoved items: {}\nLiked items: {}\nDisliked items: {}\nOther items: {}\nOutput:",
                    discarded_loves.join(", "),
                    discarded_likes.join(", "),
                    discarded_dislikes.join(", "),
                    discarded_others.join(", ")
                )
            }
        ],
        "max_completion_tokens": 512
    });
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;
    let response_json: Value = response.json().await?;
    let discarded_summary = response_json["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map_or_else(
            || Err(anyhow!("No content found in response {}", response_json)),
            |content| Ok(content.to_string()),
        )?;
    history_string.push(format!("\n\nSummary of older history: {discarded_summary}"));

    // Create the image description
    let history_string = history_string.join("\n");

    Ok((history_string, database.style))
}

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let client = Client::new();
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    let user_message = message.map_or_else(String::new, |message| format!("'User messaged '{message}', this takes precedence over any previous comments and prompts', "));

    let (history_string, style) = generate_prompt(&client, &api_key).await?;
    let request_body = json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "name": "history",
                "content": format!("History of previous prompts and comments:\n{history_string}")
            },
            {
                "role": "system",
                "content": format!(
                    "You are a wallpaper image description generator, describe a wallpaper image within 10 words\nDescribe in the simplest of terms without detail, prioritise users comments as feedback, aim for variety above all else, every image should be totally refreshing with little in common with the previous few\nTypes of content to include (not exhaustive just take inspiration) '{}'\nNever include anything '{}'",
                    style.contents.replace('\n', " "),
                    style.negative_contents.replace('\n', " ")
                )
            },
            {
                "role": "user",
                "content": format!("Create me a new image prompt, {}Prompt:", user_message)
            }
        ],
        "max_completion_tokens": 60,
        "temperature": 1.4,
        "presence_penalty": 0.6
    });
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;
    let response_json: Value = response.json().await?;
    let image_description = response_json["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map_or_else(
            || Err(anyhow!("No content found in response {}", response_json)),
            |content| Ok(content.to_string()),
        )?;
    log::info!("Generated description: {}", image_description);

    // Make another gpt request to write out the full prompt in the correct format
    let request_body = json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "name": "prompt_guidelines",
                "content": PROMPT_GUIDELINES
            },
            {
                "role": "system",
                "content": format!(
                    "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines, follow the prompt guidelines for best results\nThe overall style direction is '{}' (include the guiding style in every prompt, not exact wording but the meaning)\nNever include anything '{}'",
                    style.style.replace('\n', " "),
                    style.negative_contents.replace('\n', " ")
                )
            },
            {
                "role": "user",
                "content": format!("Create me a new image prompt from this description (use this only as a guide not a strict command, expand on it, alter details etc as you see fit) '{}', {}Prompt:", image_description, user_message)
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "prompt_data",
                "schema": {
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string" },
                        "shortened_prompt": {
                            "type": "string",
                            "description": "A shortened version of the prompt, only including the image description not style, max 25 words",
                        },
                    },
                    "required": ["prompt", "shortened_prompt"],
                    "additionalProperties": false
                },
                "strict": true
            }
        },
        "max_completion_tokens": 256
    });
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;
    let response_json: Value = response.json().await?;
    let parsed_response: PromptData = serde_json::from_str(
        &response_json["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["content"].as_str())
            .map_or_else(
                || Err(anyhow!("No content found in response {}", response_json)),
                |content| Ok(content.to_string()),
            )?,
    )?;

    Ok(parsed_response)
}
