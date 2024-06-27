use super::prompt::RELEASE_SUMMARY_PROMPT;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::env;

pub async fn summarise_release(diff: &str, commit_messages: &[String]) -> Result<String> {
    let claude_base_url = env::var("CLAUDE_BASE_URL").expect("CLAUDE_BASE_URL should be set");
    let claude_api_key = env::var("CLAUDE_API_KEY").expect("CLAUDE_API_KEY should be set");

    let commit_messages = commit_messages.join("\n");

    let mut response = reqwest::Client::new()
        .post(format!("{claude_base_url}/v1/messages"))
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header("x-api-key", claude_api_key)
        .json(&json!({
            "model": "claude-3-opus-20240229",
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
