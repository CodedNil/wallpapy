use crate::common::utils::vec_str;
use crate::common::{
    Brightness, ColorPalette, DatabaseObjectType, ImageMood, LikedState, PromptData, Season,
    SubjectMatter, TimeOfDay, VisionData, Weather,
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

2. Use Artistic References

Referencing specific artists, art movements, or styles can help guide FLUX.1’s output.

Example Prompt: Create an image in the style of Vincent van Gogh’s “Starry Night,” but replace the village with a futuristic cityscape. Maintain the swirling, expressive brushstrokes and vibrant color palette of the original, emphasizing deep blues and bright yellows. The city should have tall, glowing skyscrapers that blend seamlessly with the swirling sky.

3. Specify Technical Details

Including camera settings, angles, and other technical aspects can significantly influence the final image.

Example Prompt: Capture a street food vendor in Tokyo at night, shot with a wide-angle lens (24mm) at f/1.8. Use a shallow depth of field to focus on the vendor’s hands preparing takoyaki, with the glowing street signs and bustling crowd blurred in the background. High ISO setting to capture the ambient light, giving the image a slight grain for a cinematic feel.

4. Blend Concepts

FLUX.1 excels at combining different ideas or themes to create unique images.

Example Prompt: Illustrate “The Last Supper” by Leonardo da Vinci, but reimagine it with robots in a futuristic setting. Maintain the composition and dramatic lighting of the original painting, but replace the apostles with various types of androids and cyborgs. The table should be a long, sleek metal surface with holographic displays. In place of bread and wine, have the robots interfacing with glowing data streams.

5. Use Contrast and Juxtaposition

Creating contrast within your prompt can lead to visually striking and thought-provoking images.

Example Prompt: Create an image that juxtaposes the delicate beauty of nature with the harsh reality of urban decay. Show a vibrant cherry blossom tree in full bloom growing out of a cracked concrete sidewalk in a dilapidated city alley. The tree should be the focal point, with its pink petals contrasting against the gray, graffiti-covered walls of surrounding buildings. Include a small bird perched on one of the branches to emphasize the theme of resilience.

6. Incorporate Mood and Atmosphere

Describing the emotional tone or atmosphere can help FLUX.1 generate images with the desired feel.

Example Prompt: Depict a cozy, warmly lit bookstore cafe on a rainy evening. The atmosphere should be inviting and nostalgic, with soft yellow lighting from vintage lamps illuminating rows of well-worn books. Show patrons reading in comfortable armchairs, steam rising from their coffee cups. The large front window should reveal a glistening wet street outside, with blurred lights from passing cars. Emphasize the contrast between the warm interior and the cool, rainy exterior.

7. Leverage FLUX.1’s Text Rendering Capabilities

FLUX.1’s superior text rendering allows for creative use of text within images.

Example Prompt: Create a surreal advertisement poster for a fictional time travel agency. The background should depict a swirling vortex of clock faces and historical landmarks from different eras. In the foreground, place large, bold text that reads “CHRONO TOURS: YOUR PAST IS OUR FUTURE” in a retro-futuristic font. The text should appear to be partially disintegrating into particles that are being sucked into the time vortex. Include smaller text at the bottom with fictional pricing and the slogan “History is just a ticket away!”

8. Experiment with Unusual Perspectives

Challenging FLUX.1 with unique viewpoints can result in visually interesting images.

Example Prompt: Illustrate a “bug’s-eye view” of a picnic in a lush garden. The perspective should be from ground level, looking up at towering blades of grass and wildflowers that frame the scene. In the distance, show the underside of a red and white checkered picnic blanket with the silhouettes of picnic foods and human figures visible through the semi-transparent fabric. Include a few ants in the foreground carrying crumbs, and a ladybug climbing a blade of grass. The lighting should be warm and dappled, as if filtering through leaves.

Advanced Techniques
1. Layered Prompts

For complex scenes, consider breaking down your prompt into layers, focusing on different elements of the image.

Example Prompt: Create a bustling marketplace in a fantastical floating city.

Layer 1 (Background): Depict a city of interconnected floating islands suspended in a pastel sky. The islands should have a mix of whimsical architecture styles, from towering spires to quaint cottages. Show distant airships and flying creatures in the background.

Layer 2 (Middle ground): Focus on the main marketplace area. Illustrate a wide plaza with colorful stalls and shops selling exotic goods. Include floating platforms that serve as walkways between different sections of the market.

Layer 3 (Foreground): Populate the scene with a diverse array of fantasy creatures and humanoids. Show vendors calling out to customers, children chasing magical floating bubbles, and a street performer juggling balls of light. In the immediate foreground, depict a detailed stall selling glowing potions and mystical artifacts.

Atmosphere: The overall mood should be vibrant and magical, with soft, ethereal lighting that emphasizes the fantastical nature of the scene.

2. Style Fusion

Combine multiple artistic styles to create unique visual experiences.

Example Prompt: Create an image that fuses the precision of M.C. Escher’s impossible geometries with the bold colors and shapes of Wassily Kandinsky’s abstract compositions. The subject should be a surreal cityscape where buildings seamlessly transform into musical instruments. Use Escher’s techniques to create paradoxical perspectives and interconnected structures, but render them in Kandinsky’s vibrant, non-representational style. Incorporate musical notations and abstract shapes that flow through the scene, connecting the architectural elements. The color palette should be rich and varied, with particular emphasis on deep blues, vibrant reds, and golden yellows.

3. Temporal Narratives

Challenge FLUX.1 to convey a sense of time passing or a story unfolding within a single image.

Example Prompt: Illustrate the life cycle of a monarch butterfly in a single, continuous image. Divide the canvas into four seamlessly blending sections, each representing a stage of the butterfly’s life.

Start on the left with a milkweed plant where tiny eggs are visible on the underside of a leaf. As we move right, show the caterpillar stage with the larva feeding on milkweed leaves. In the third section, depict the chrysalis stage, with the green and gold-flecked pupa hanging from a branch.

Finally, on the right side, show the fully formed adult butterfly emerging, with its wings gradually opening to reveal the iconic orange and black pattern. Use a soft, natural color palette dominated by greens and oranges. The background should subtly shift from spring to summer as we move from left to right, with changing foliage and lighting to indicate the passage of time.

4. Emotional Gradients

Direct FLUX.1 to create images that convey a progression of emotions or moods.

Example Prompt: Create a panoramic image that depicts the progression of a person’s emotional journey from despair to hope. The scene should be a long, winding road that starts in a dark, stormy landscape and gradually transitions to a bright, sunlit meadow.

On the left, begin with a lone figure hunched against the wind, surrounded by bare, twisted trees and ominous storm clouds. As we move right, show the gradual clearing of the sky, with the road passing through a misty forest where hints of light begin to break through.

Continue the transition with the forest opening up to reveal distant mountains and a rainbow. The figure should become more upright and purposeful in their stride. Finally, on the far right, show the person standing tall in a sunlit meadow full of wildflowers, arms outstretched in a gesture of triumph or liberation.

Use color and lighting to enhance the emotional journey: start with a dark, desaturated palette on the left, gradually introducing more color and brightness as we move right, ending in a vibrant, warm color scheme. The overall composition should create a powerful visual metaphor for overcoming adversity and finding hope.

Tips for Optimal Results

    Experiment with Different Versions: FLUX.1 comes in different variants (Pro, Dev, and Schnell). Experiment with each to find the best fit for your needs.

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
                    format!("\tPrompt was: '{}'", wallpaper.prompt_data.prompt),
                    format!(
                        "\tColors: {}, {}, {}",
                        vision.primary_color, vision.secondary_color, vision.tertiary_color
                    ),
                    format!(
                        "\tTime: {} - Season: {} - Weather: {}",
                        vision.time_of_day,
                        vision.season,
                        vec_str(&vision.weather)
                    ),
                    format!("\tTags: {}", vec_str(&vision.tags)),
                    format!("\tMoods: {}", vec_str(&vision.image_mood)),
                    format!("\tPalette: {}", vec_str(&vision.color_palette)),
                    format!("\tSubject: {}", vec_str(&vision.subject_matter)),
                    format!("\tWhat worked well: {}", vision.what_worked_well),
                    format!("\tWhat didn't work: {}", vision.what_didnt_work),
                    format!(
                        "\tDifferences in image output compared to prompt:  {}",
                        vision.differences_from_prompt
                    ),
                    format!(
                        "\tHow to improve prompt creation:  {}",
                        vision.how_to_improve
                    ),
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
    if let Some(message) = message {
        history_string.push(format!("For this image the user requested: '{message}'"));
    }
    let history_string = history_string.join("\n\n");

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
                "content": "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines, follow the prompt guidelines for best results, prioritise users comments as feedback"
            },
            {
                "role": "user",
                "content": "Create me a new image prompt, Prompt:"
            }
        ],
        "response_format": {
          "type": "json_schema",
          "json_schema": {
            "name": "prompt_data",
            "schema": {
              "type": "object",
              "properties": {
                "style": {
                    "type": "string",
                    "description": "The style of the image, max 25 words",
                },
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
                    "description":  "The tags for the image, max 6, titlecase",
                    "items": {
                        "type": "string",
                    }
                },
                "prompt": { "type": "string" },
                "shortened_prompt": {
                    "type": "string",
                    "description": "A shortened version of the prompt, only including the image description, max 25 words",
                },
              },
              "required": ["style", "time_of_day", "season", "tags", "prompt", "shortened_prompt"],
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

pub async fn vision_image(image: DynamicImage, prompt: &str) -> Result<VisionData> {
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
                    "text": format!("View this image and output data about it in required json schema, the image is a desktop wallpaper image created by a generative diffusion model, based on this prompt: '{prompt}'")
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
                "primary_color": {
                    "type": "string",
                    "description": "The primary color of the image, color name"
                },
                "primary_color_rgb": {
                    "type": "array",
                    "description": "The primary color of the image, 3 u8 integers",
                    "items": {
                      "type": "number",
                    },
                },
                "secondary_color": {
                    "type": "string",
                    "description": "The secondary color of the image, color name, blank if there is none"
                },
                "secondary_color_rgb": {
                    "type": "array",
                    "description": "The primary color of the image, 3 u8 integers, 3 zeros if there is none",
                    "items": {
                        "type": "number",
                    },
                },
                "tertiary_color": {
                    "type": "string",
                    "description": "The tertiary color of the image, color name, blank if there is none"
                },
                "tertiary_color_rgb": {
                    "type": "array",
                    "description": "The primary color of the image, 3 u8 integers, 3 zeros if there is none",
                    "items": {
                        "type": "number",
                    },
                },
                "brightness": {
                    "type": "string",
                    "description": "How bright is the image",
                    "enum": Brightness::VARIANTS
                },

                "what_worked_well": {
                    "type": "string",
                    "description": "What worked well in the image, be concise"
                },
                "what_didnt_work": {
                    "type": "string",
                     "description": "What did not work well in the image, be concise"
                },
                "differences_from_prompt": {
                    "type": "string",
                    "description": "What was different about the image from the input prompt, describe in detail, be concise, leave blank if the image was accurate to the prompt"
                },
                "how_to_improve": {
                    "type": "string",
                    "description": "How could future prompts be written in a way that recognises the diffusion models weaknesses in following prompts, be concise, be general not specific to this image to apply to future prompts for new images, leave blank if the image was accurate to the prompt"
                },

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
                "weather": {
                    "type": "array",
                     "description": "The weather for the image, max 3",
                     "items": {
                         "type": "string",
                         "enum": Weather::VARIANTS
                     }
                 },

                "tags": {
                    "type": "array",
                    "description":  "The tags for the image, max 6, titlecase",
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
                 "color_palette": {
                     "type": "array",
                     "description": "The color palette for the image, max 3",
                     "items": {
                         "type": "string",
                         "enum": ColorPalette::VARIANTS
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
              "required": ["primary_color", "primary_color_rgb", "secondary_color", "secondary_color_rgb", "tertiary_color", "tertiary_color_rgb", "brightness", "what_worked_well", "what_didnt_work", "differences_from_prompt", "how_to_improve", "time_of_day", "season", "weather", "tags", "image_mood", "color_palette", "subject_matter"],
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
