use crate::{
    ai::{prompts, schemas},
    utils::config,
};
use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

pub struct Claude;

impl Claude {
    pub async fn get_pr_review(diff: &str, commit_messages: &[String]) -> Result<PrReviewResponse> {
        tracing::info!("Generating PR analysis");

        let commit_messages = commit_messages.join("\n");

        let user_prompt = format!(
            "<Diff>{diff}</Diff>
            <CommitMessages>{commit_messages}</CommitMessages>"
        );

        let response = Self::make_request(
            prompts::PR_BUG_ANALYSIS,
            user_prompt,
            schemas::pr_review_response(),
            "pr_review",
        )
        .await?;

        Ok(response)
    }

    async fn make_request<T: DeserializeOwned>(
        system_input: &'static str,
        user_input: String,
        tool_schema: Value,
        tool_name: &'static str,
    ) -> Result<T> {
        let base_url = config::get("CLAUDE_BASE_URL");
        let api_key = config::get("CLAUDE_API_KEY");
        let model = config::get("CLAUDE_MODEL");

        let response = reqwest::Client::new()
            .post(format!("{base_url}/v1/messages"))
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", api_key)
            .json(&json!({
                "model": model,
                "max_tokens": 1024,
                "temperature": 0.0,
                "system": system_input,
                "messages": [{ "role": "user", "content": user_input }],
                "tools": [tool_schema],
                "tool_choice": { "type": "tool", "name": tool_name }
            }))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error making Claude request: {e}"))?
            .json::<ApiResponse<T>>()
            .await?
            .content
            .into_iter()
            .next()
            .expect("At least one item to be returned")
            .input;

        Ok(response)
    }
}

#[derive(Deserialize)]
pub struct PrReviewResponse {
    pub verdict: Verdict,
    pub feedback: String,
}

impl PrReviewResponse {
    pub fn is_positive(&self) -> bool {
        matches!(self.verdict, Verdict::Positive)
    }
}

#[derive(Deserialize, Debug)]
pub enum Verdict {
    Positive,
    Negative,
}

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    pub content: Vec<ContentItem<T>>,
}

#[derive(Deserialize)]
pub struct ContentItem<T> {
    pub input: T,
}
