use super::AiSummary;
use crate::utils::config;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

pub struct Claude;

impl AiSummary for Claude {
    async fn make_request(input: String) -> Result<String> {
        let base_url = config::get("ANTHROPIC_BASE_URL")?;
        let api_key = config::get("ANTHROPIC_API_KEY")?;
        let model = config::get("ANTHROPIC_MODEL")?;

        let mut response = reqwest::Client::new()
            .post(format!("{base_url}/v1/messages"))
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", api_key)
            .json(&json!({
                "model": model,
                "max_tokens": 1024,
                "temperature": 0.0,
                "system": format!("Prompt: {}", Self::SYSTEM_PROMPT),
                "messages": [{ "role": "user", "content": input }]
            }))
            .send()
            .await?
            .error_for_status()?
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
