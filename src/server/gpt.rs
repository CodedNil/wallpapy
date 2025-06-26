use crate::common::{Database, DatabaseStyle, LikedState, PromptData};
use crate::server::{format_duration, read_database};
use anyhow::{Result, anyhow};
use chrono::Utc;
use log::error;
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, SchemaGenerator};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, env, error::Error, sync::LazyLock};

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub async fn llm_parse<T>(
    context: Vec<String>,
    message: String,
) -> Result<T, Box<dyn Error + Send + Sync>>
where
    T: JsonSchema + DeserializeOwned,
{
    // Construct the URL with proper variable substitution
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}",
        model = "gemini-2.5-flash-lite-preview-06-17",
        api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set")
    );

    // Set up request headers.
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Generate the JSON schema dynamically using `schemars`.
    let mut schema_object = SchemaGenerator::new(SchemaSettings::openapi3().with(|s| {
        s.inline_subschemas = true;
    }))
    .into_root_schema_for::<T>();
    if let Some(object) = schema_object.as_object_mut() {
        object.remove("$schema");
    }

    // Create the inputs
    let mut payload = json!({
        "contents": [{"parts": [{"text": message}]}],
        "generationConfig": {
            "response_mime_type": "application/json",
            "response_schema": schema_object
        }
    });
    if !context.is_empty() {
        let system_parts = context
            .into_iter()
            .map(|msg| json!({"text": msg}))
            .collect::<Vec<_>>();
        payload["system_instruction"] = json!({"parts": system_parts});
    }

    // Send the request and check for errors
    let response = HTTP_CLIENT
        .post(url)
        .headers(headers)
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
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .ok_or("Unexpected response structure")?;

    Ok(serde_json::from_str(inner_text)?)
}

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

#[derive(Serialize, Deserialize, JsonSchema)]
struct DiscardedSummary {
    /// Summary of the users loved descriptions, do not include common things like seasons, time of day etc, do not repeat similar items and err on the side of fewer items, ideally 1 word per item, max 3 words per item if needed
    loved: Vec<String>,
    /// Summary of the users liked descriptions, same rules as for loved
    liked: Vec<String>,
    /// Summary of the users disliked descriptions, same rules as for loved
    disliked: Vec<String>,
    /// Summary of all other descriptions, same rules as for loved
    others: Vec<String>,
}

pub async fn generate_prompt() -> Result<(String, DatabaseStyle)> {
    // Read the database
    let database = match read_database().await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed accessing database {:?}", e);
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

    // Use LLM to summarize the discarded string into the key elements
    match llm_parse::<DiscardedSummary>(
        vec![],
        format!(
            "Loved items: {}\nLiked items: {}\nDisliked items: {}\nOther items: {}",
            discarded_loves.join(", "),
            discarded_likes.join(", "),
            discarded_dislikes.join(", "),
            discarded_others.join(", ")
        ),
    )
    .await
    {
        Ok(output) => {
            let mut summary_parts = Vec::new();

            if !output.loved.is_empty() {
                summary_parts.push(format!("(user LOVED: {})", output.loved.join(", ")));
            }
            if !output.liked.is_empty() {
                summary_parts.push(format!("(user liked: {})", output.liked.join(", ")));
            }
            if !output.disliked.is_empty() {
                summary_parts.push(format!("(user disliked: {})", output.disliked.join(", ")));
            }
            if !output.others.is_empty() {
                summary_parts.push(format!("(others: {})", output.others.join(", ")));
            }

            if !summary_parts.is_empty() {
                history_string.push(format!(
                    "\n\nSummary of older history: {}",
                    summary_parts.join(" ")
                ));
            }
        }
        Err(err) => {
            error!("Failed to parse discarded summary: {}", err);
        }
    }

    Ok((history_string.join("\n"), database.style))
}

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let user_message = message.map_or_else(String::new, |message| format!("'User messaged '{message}', this takes precedence over any previous comments and prompts', "));

    let (history_string, style) = generate_prompt().await?;
    llm_parse::<PromptData>(vec![PROMPT_GUIDELINES.to_string(), format!("History of previous prompts and comments:\n{history_string}"), format!(
            "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines, follow the prompt guidelines for best results, prioritise users comments as feedback, aim for variety above all else, every image should be totally refreshing with little in common with the previous few\nTypes of content to include (not exhaustive just take inspiration) '{}'\nThe overall style direction is '{}' (include the guiding style in every prompt, not exact wording but the meaning)\nNever include anything '{}'",
            style.contents.replace('\n', " "),
            style.style.replace('\n', " "),
            style.negative_contents.replace('\n', " ")
        )], format!("Create me a new image prompt, {user_message}\nPrompt:")).await.map_err(|err| {
            anyhow!("Failed to generate prompt: {}", err)
        })
}
