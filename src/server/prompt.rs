use crate::common::{CommentData, DatabaseObjectType, LikedState, WallpaperData};
use crate::server::{COMMENTS_TREE, DATABASE_PATH, IMAGES_TREE};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
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

pub async fn generate(message: Option<String>) -> Result<(String, String)> {
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
        history_string.push_str(&match data {
            DatabaseObjectType::Wallpaper(wallpaper) => {
                let liked_state = match wallpaper.liked_state {
                    LikedState::Liked => " (user liked this)",
                    LikedState::Disliked => " (user disliked this)",
                    LikedState::None => "",
                };
                let prompt = wallpaper.prompt.replace('\n', "  ");
                format!("{datetime_text}: Wallpaper{liked_state} created with prompt: '{prompt}'")
            }
            DatabaseObjectType::Comment(comment) => {
                let comment = comment.comment;
                format!("{datetime_text}: User commented: '{comment}'")
            }
        });
        history_string.push('\n');
    }
    if let Some(message) = message {
        history_string.push_str(&format!("For this image the user requested: '{message}'"));
        history_string.push('\n');
    }

    let client = reqwest::Client::new();

    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    let prompt = gpt(&client, &api_key, json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "name": "prompt_guidelines",
                "content": PROMPT_GUIDELINES
            },
            {
                "role": "system",
                "content": "You are a wallpaper image prompt generator, write a prompt for an wallpaper image in a few sentences without new lines, follow the prompt guidelines for best results, prioritise users comments as feedback"
            },
            {
                "role": "system",
                "name": "history",
                "content": format!("History of previous prompts and comments:\n{history_string}")
            },
            {
                "role": "user",
                "content": "Create me a new image prompt, Prompt:"
            }
        ],
        "max_tokens": 4096
    })).await?;

    let prompt_short = gpt(&client, &api_key, json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": format!("Take this input '{prompt}' and return a shortened version of it, max 12 words")
            }
        ],
        "max_tokens": 4096
    })).await?;

    Ok((prompt, prompt_short))
}

async fn gpt(client: &Client, api_key: &str, request_body: Value) -> Result<String> {
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await?;

    let response_json: Value = response.json().await?;
    response_json["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map_or_else(
            || Err(anyhow!("No content found in response")),
            |content| Ok(content.to_string()),
        )
}
