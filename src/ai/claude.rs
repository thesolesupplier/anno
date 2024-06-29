use super::prompt::RELEASE_SUMMARY_PROMPT;
use crate::utils::config;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

pub async fn summarise_release(diff: &str, commit_messages: &[String]) -> Result<String> {
    let base_url = config::get("ANTHROPIC_BASE_URL")?;
    let api_key = config::get("ANTHROPIC_API_KEY")?;
    let model = config::get("ANTHROPIC_MODEL")?;

    let commit_messages = commit_messages.join("\n");

    let mut response = reqwest::Client::new()
        .post(format!("{base_url}/v1/messages"))
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header("x-api-key", api_key)
        .json(&json!({
            "model": model,
            "max_tokens": 1024,
            "temperature": 0.0,
            "system": format!("Prompt: {RELEASE_SUMMARY_PROMPT}"),
            "messages": [{
                "role": "user",
                "content": format!("
                    <Diff>{diff}</Diff>
                    <CommitMessages>{commit_messages}</CommitMessages>
                ")
            }]
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<Response>()
        .await?;

    let summary = response.content.remove(0).text;

    Ok(summary)
}

#[derive(Deserialize)]
pub struct Response {
    content: Vec<TextContent>,
}

#[derive(Deserialize)]
struct TextContent {
    text: String,
}
