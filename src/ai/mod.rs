mod chat_gpt;
mod claude;
mod prompt;

use crate::utils::config;
use anyhow::Result;
use chat_gpt::ChatGpt;
use claude::Claude;
use prompt::RELEASE_SUMMARY_PROMPT;
use regex_lite::Regex;
use std::sync::OnceLock;

pub async fn get_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
    tracing::info!("Fetching AI summary");

    let llm_provider = config::get("LLM_PROVIDER")?;

    match llm_provider.as_str() {
        "anthropic" => Claude::get_summary(diff, commit_messages).await,
        _ => ChatGpt::get_summary(diff, commit_messages).await,
    }
}

trait AiSummary {
    const SYSTEM_PROMPT: &'static str = RELEASE_SUMMARY_PROMPT;

    async fn make_request(input: String) -> Result<String>;

    async fn get_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
        let commit_messages = commit_messages.join("\n");

        let input = format!(
            "<Diff>{diff}</Diff>
            <CommitMessages>{commit_messages}<CommitMessages>"
        );

        let response = Self::make_request(input).await?;
        let summary = Self::extract_output(response);

        Ok(summary)
    }

    fn extract_output(output: String) -> String {
        let output_regex =
            OUTPUT_REGEX.get_or_init(|| Regex::new(r"(?s)<Output>(.*?)<\/Output>").unwrap());

        let Some(matches) = output_regex.captures(&output) else {
            return output;
        };

        if matches.len() == 0 {
            return output;
        }

        matches.get(1).unwrap().as_str().trim().to_string()
    }
}

static OUTPUT_REGEX: OnceLock<Regex> = OnceLock::new();
