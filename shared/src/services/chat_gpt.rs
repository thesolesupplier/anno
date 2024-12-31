use crate::utils::config;
use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

pub async fn make_request<T: DeserializeOwned>(
    system_input: &'static str,
    user_input: String,
    response_format: Value,
) -> Result<T> {
    let base_url = config::get("CHAT_GPT_BASE_URL");
    let api_key = config::get("CHAT_GPT_API_KEY");
    let model = config::get_optional("CHAT_GPT_MODEL");

    let response = reqwest::Client::new()
        .post(format!("{base_url}/chat/completions"))
        .header("content-type", "application/json")
        .bearer_auth(api_key)
        .json(&json!({
            "model": model.as_deref().unwrap_or("gpt-4o"),
            "temperature": 0.0,
            "frequency_penalty": 0.3,
            "messages": [
                { "role": "system", "content": system_input },
                { "role": "user", "content": user_input }
            ],
            "response_format": response_format
        }))
        .send()
        .await?
        .error_for_status()
        .inspect_err(|e| tracing::error!("Error making ChatGPT request: {e}"))?
        .json::<ApiResponse>()
        .await?
        .choices
        .into_iter()
        .next()
        .expect("At least one choice to be returned")
        .message
        .content;

    let parsed_response: T = serde_json::from_str(&response)?;

    Ok(parsed_response)
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
