use crate::common::{
    Brightness, ColorPalette, DatabaseObjectType, ImageMood, LikedState, PromptData, Season,
    SubjectMatter, TimeOfDay, VisionData,
};
use crate::server::{read_database, Database};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::DynamicImage;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use strum::VariantNames;
use time::format_description;

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

pub async fn generate(message: Option<String>) -> Result<PromptData> {
    let client = Client::new();
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    // Read the database
    let database = match read_database().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("Failed accessing database {:?}", e);
            Database {
                key_style: String::new(),
                wallpapers: HashMap::new(),
                comments: HashMap::new(),
            }
        }
    };

    // Collect the images and comments into a single list, sorted by datetime
    let mut database_history = database
        .wallpapers
        .values()
        .map(|wallpaper| {
            (
                wallpaper.datetime,
                DatabaseObjectType::Wallpaper(wallpaper.clone()),
            )
        })
        .chain(database.comments.values().map(|comment| {
            (
                comment.datetime,
                DatabaseObjectType::Comment(comment.clone()),
            )
        }))
        .collect::<Vec<_>>();
    database_history.sort_by_key(|(datetime, _)| *datetime);

    let mut history_string = Vec::new();
    for (date, data) in database_history {
        let format = format_description::parse("[day]/[month]/[year] [hour]:[minute]").unwrap();
        let datetime_text = date.format(&format).unwrap();
        history_string.push(match data {
            DatabaseObjectType::Wallpaper(wallpaper) => {
                let liked_state: &str = match wallpaper.liked_state {
                    LikedState::Loved => " (user LOVED this)",
                    LikedState::Liked => " (user liked this)",
                    LikedState::Disliked => " (user disliked this)",
                    LikedState::None => "",
                };
                let vision = wallpaper.vision_data;
                let details = [
                    format!("{datetime_text} - Wallpaper{liked_state} created"),
                    format!(
                        "Image Description: '{}'",
                        wallpaper.prompt_data.shortened_prompt
                    ),
                    format!("Time: {} - Season: {}", vision.time_of_day, vision.season,),
                ];
                let filtered_details: Vec<String> = details
                    .into_iter()
                    .filter(|s| !s.trim().ends_with(':'))
                    .collect();
                filtered_details.join("\n")
            }
            DatabaseObjectType::Comment(comment) => {
                let comment = comment.comment;
                format!("{datetime_text}: User commented: '{comment}'")
            }
        });
    }
    let history_string = history_string.join("\n\n");

    let user_message = message.map_or_else(String::new, |message| format!("'{message}' "));
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "name": "prompt_guidelines",
                "content": PROMPT_GUIDELINES
            },
            {
                "role": "system",
                "name": "history",
                "content": format!("History of previous prompts and comments:\n{history_string}")
            },
            {
                "role": "system",
                "content": format!("You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines, follow the prompt guidelines for best results, prioritise users comments as feedback, aim for variety above all else, every image should be totally distinct to the previous ones, the overall style direction is '{}' (include this in every prompt, not exact wording but the meaning)", database.key_style)
            },
            {
                "role": "user",
                "content": format!("Create me a new image prompt, {}Prompt:", user_message)
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
                    "description": "A shortened version of the prompt, only including the image description, max 25 words",
                },
              },
              "required": ["prompt", "shortened_prompt"],
              "additionalProperties": false
            },
            "strict": true
          }
        },
        "max_tokens": 4096
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;
    let response_json: Value = response.json().await?;
    let json_response = response_json["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map_or_else(
            || Err(anyhow!("No content found in response {}", response_json)),
            |content| Ok(content.to_string()),
        )?;

    let parsed_response: PromptData = serde_json::from_str(&json_response)?;

    // Log token usage
    let prompt_tokens = response_json["usage"]["prompt_tokens"].as_u64().unwrap();
    let completition_tokens = response_json["usage"]["completion_tokens"]
        .as_u64()
        .unwrap();
    let (prompt_ppm, completition_ppm) = (0.15, 0.6);
    let (prompt_cost, completition_cost) = (
        (prompt_ppm / 1_000_000.0) * prompt_tokens as f32,
        (completition_ppm / 1_000_000.0) * completition_tokens as f32,
    );
    let total_cost = prompt_cost + completition_cost;
    log::info!(
        "Generated prompt using {} prompt tokens and {} completition tokens at ${}",
        prompt_tokens,
        completition_tokens,
        total_cost,
    );

    Ok(parsed_response)
}

pub async fn vision_image(image: DynamicImage) -> Result<VisionData> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = reqwest::Client::new();

    let image = image.resize(854, 640, FilterType::Lanczos3);
    let mut bytes = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut bytes, 90);
    image.write_with_encoder(encoder)?;
    let image_uri = format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes));

    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": [
                  {
                    "type": "text",
                    "text": "View this image and output data about it in required json schema'"
                  },
                  {
                    "type": "image_url",
                    "image_url": {
                      "url": image_uri,
                      "detail": "low"
                    }
                  }
                ]
            }
        ],
        "response_format": {
          "type": "json_schema",
          "json_schema": {
            "name": "vision_data",
            "schema": {
              "type": "object",
              "properties": {
                "time_of_day": {
                    "type": "string",
                    "description": "The time of day for the image",
                    "enum": TimeOfDay::VARIANTS
                },
                "season": {
                    "type": "string",
                    "description": "The season for the image",
                    "enum": Season::VARIANTS
                },

                "tags": {
                    "type": "array",
                    "description":  "The tags for the image, max 5, titlecase",
                    "items": {
                        "type": "string",
                    }
                },
                "image_mood": {
                    "type": "array",
                     "description": "The moods for the image, max 3",
                     "items": {
                         "type": "string",
                         "enum": ImageMood::VARIANTS
                     }
                 },
                 "brightness": {
                     "type": "string",
                     "description": "How bright is the image",
                     "enum": Brightness::VARIANTS
                 },
                 "color_palette": {
                     "type": "array",
                     "description": "The color palette for the image, max 3",
                     "items": {
                         "type": "string",
                         "enum": ColorPalette::VARIANTS
                     }
                 },
                 "key_colors": {
                     "type": "array",
                     "description": "The main colors in the image, in order of frequency/importance, max 6",
                     "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The name of the color, one or two words, focusing on more refined shades. For example, instead of saying purple you might specify lilac"
                            },
                            "rgb_values": {
                                "type": "array",
                                "description": "The rgb color, 3 u8 integers",
                                "items": {
                                "type": "number",
                                },
                            }
                        },
                        "required": ["name", "rgb_values"],
                        "additionalProperties": false
                    }
                 },
                 "subject_matter": {
                     "type": "array",
                     "description":   "The subject matter for the image, max 3",
                     "items": {
                         "type": "string",
                         "enum": SubjectMatter::VARIANTS
                     }
                 },
              },
              "required": ["time_of_day", "season", "tags", "image_mood", "brightness", "color_palette", "key_colors", "subject_matter"],
              "additionalProperties": false
            },
            "strict": true
          }
        },
        "max_tokens": 4096
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;
    let response_json: Value = response.json().await?;
    let json_response = response_json["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map_or_else(
            || Err(anyhow!("No content found in response {}", response_json)),
            |content| Ok(content.to_string()),
        )?;

    let parsed_response: VisionData = serde_json::from_str(&json_response)?;

    // Log token usage
    let prompt_tokens = response_json["usage"]["prompt_tokens"].as_u64().unwrap();
    let completition_tokens = response_json["usage"]["completion_tokens"]
        .as_u64()
        .unwrap();
    let (prompt_ppm, completition_ppm) = (0.15, 0.6);
    let (prompt_cost, completition_cost) = (
        (prompt_ppm / 1_000_000.0) * prompt_tokens as f32,
        (completition_ppm / 1_000_000.0) * completition_tokens as f32,
    );
    let total_cost = prompt_cost + completition_cost;
    log::info!(
        "Visioned image using {} prompt tokens and {} completition tokens at ${}",
        prompt_tokens,
        completition_tokens,
        total_cost,
    );

    Ok(parsed_response)
}
