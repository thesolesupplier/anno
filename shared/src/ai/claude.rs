use crate::{ai::prompts, utils::config};
use anyhow::Result;
use regex_lite::Regex;
use serde::Deserialize;
use serde_json::json;

pub struct Claude;

impl Claude {
    pub async fn get_pr_bug_analysis(diff: &str, commit_messages: &[String]) -> Result<String> {
        tracing::info!("Generating PR analysis");

        let commit_messages = commit_messages.join("\n");

        let user_prompt = format!(
            "<Diff>{diff}</Diff>
            <CommitMessages>{commit_messages}</CommitMessages>"
        );

        let response = Self::make_request(prompts::PR_BUG_ANALYSIS, user_prompt).await?;

        Ok(Self::extract_output(response))
    }

    async fn make_request(system_input: &'static str, user_input: String) -> Result<String> {
        let base_url = config::get("CLAUDE_BASE_URL");
        let api_key = config::get("CLAUDE_API_KEY");
        let model = config::get("CLAUDE_MODEL");

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

    fn extract_output(output: String) -> String {
        let output_regex = Regex::new(r"(?s)<Output>(.*?)<\/Output>").unwrap();

        let Some(matches) = output_regex.captures(&output) else {
            return output;
        };

        if matches.len() == 0 {
            return output;
        }

        matches.get(1).unwrap().as_str().trim().to_string()
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
