use super::Ai;
use crate::utils::config;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

pub struct Claude;

impl Ai for Claude {
    async fn make_request(system_input: &str, user_input: String) -> Result<String> {
        let base_url = config::get("CLAUDE_BASE_URL")?;
        let api_key = config::get("CLAUDE_API_KEY")?;
        let model = config::get("CLAUDE_MODEL")?;

        let mut response = reqwest::Client::new()
            .post(format!("{base_url}/v1/messages"))
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", api_key)
            .json(&json!({
                "model": model,
                "max_tokens": 1024,
                "temperature": 0.0,
                "system": system_input,
                "messages": [{ "role": "user", "content": user_input }]
            }))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error making Claude request: {e}"))?
            .json::<ApiResponse>()
            .await?;

        let summary = response.content.remove(0).text;

        Ok(summary)
    }
}

#[derive(Deserialize)]
pub struct ApiResponse {
    content: Vec<Content>,
}

#[derive(Deserialize)]
struct Content {
    text: String,
}
