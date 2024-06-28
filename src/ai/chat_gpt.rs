use super::prompt::RELEASE_SUMMARY_PROMPT;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::env;

pub async fn summarise_release(diff: &str, commit_messages: &[String]) -> Result<String> {
    let openai_base_url = env::var("OPENAI_BASE_URL").expect("OPENAI_BASE_URL should be set");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY should be set");

    let commit_messages = commit_messages.join("\n");

    let mut response = reqwest::Client::new()
        .post(format!("{openai_base_url}/chat/completions"))
        .header("content-type", "application/json")
        .bearer_auth(openai_api_key)
        .json(&json!({
            "model": "gpt-4o",
            "temperature": 0.0,
            "frequency_penalty": 2.0,
            "messages": [
                {
                    "role": "system",
                    "content": RELEASE_SUMMARY_PROMPT
                },
                {
                    "role": "user",
                    "content": format!(
                        "<Diff>{diff}</Diff>
                        <Commit Messages>{commit_messages}<Commit Messages>"
                    )
                }
            ]
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<Response>()
        .await?;

    Ok(response.get_summary())
}

#[derive(Deserialize)]
pub struct Response {
    choices: Vec<Message>,
}

impl Response {
    pub fn get_summary(&mut self) -> String {
        self.choices.remove(0).message.content
    }
}

#[derive(Deserialize)]
struct Message {
    message: Content,
}

#[derive(Deserialize)]
struct Content {
    content: String,
}
