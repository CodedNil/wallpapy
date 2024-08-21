use anyhow::Result;
use async_openai::{
    types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};

pub async fn generate() -> Result<String> {
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4096u32)
        .model("gpt-4o")
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content("Prompt for an image for a wallpaper in two sentences, fantasy landscape")
            .build()?
            .into()])
        .build()?;

    let response = client.chat().create(request).await?;

    response.choices.first().map_or_else(
        || Err(anyhow::anyhow!("No choices found")),
        |choice| {
            choice.message.content.as_ref().map_or_else(
                || Err(anyhow::anyhow!("Content is missing")),
                |content| Ok(content.clone()),
            )
        },
    )
}
