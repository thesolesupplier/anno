use super::Ai;
use crate::utils::config;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

pub struct ChatGpt;

impl Ai for ChatGpt {
    async fn make_request(system_input: &str, user_input: String) -> Result<String> {
        let base_url = config::get("CHAT_GPT_BASE_URL")?;
        let api_key = config::get("CHAT_GPT_API_KEY")?;
        let model = config::get("CHAT_GPT_MODEL")?;

        let mut response = reqwest::Client::new()
            .post(format!("{base_url}/chat/completions"))
            .header("content-type", "application/json")
            .bearer_auth(api_key)
            .json(&json!({
                "model": model,
                "temperature": 0.0,
                "frequency_penalty": 1.0,
                "messages": [
                    { "role": "system", "content": system_input },
                    { "role": "user", "content": user_input }
                ]
            }))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error making ChatGPT request: {e}"))?
            .json::<ApiResponse>()
            .await?;

        let summary = response.choices.remove(0).message.content;

        Ok(summary)
    }
}

#[derive(Deserialize)]
pub struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}
