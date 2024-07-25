mod chat_gpt;
mod claude;
mod prompts;

use crate::utils::config;
use anyhow::Result;
use chat_gpt::ChatGpt;
use claude::Claude;
use regex_lite::Regex;
use std::sync::OnceLock;

pub async fn get_release_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
    match config::get("LLM_PROVIDER")?.as_str() {
        "anthropic" => Claude::get_release_summary(diff, commit_messages).await,
        _ => ChatGpt::get_release_summary(diff, commit_messages).await,
    }
}

pub async fn analyse_pr(diff: &str, adrs: &[String]) -> Result<String> {
    match config::get("LLM_PROVIDER")?.as_str() {
        "anthropic" => Claude::analyse_pr(diff, adrs).await,
        _ => ChatGpt::analyse_pr(diff, adrs).await,
    }
}

impl<T> ReleaseSummary for T where T: Ai {}
impl<T> PrAnalysis for T where T: Ai {}

trait ReleaseSummary: Ai {
    async fn get_release_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
        tracing::info!("Fetching AI release summary");

        let commit_messages = commit_messages.join("\n");

        let input = format!(
            "<Diff>{diff}</Diff>
            <CommitMessages>{commit_messages}</CommitMessages>"
        );

        Self::prompt(prompts::RELEASE_SUMMARY, input).await
    }
}

trait PrAnalysis: Ai {
    async fn analyse_pr(diff: &str, adrs: &[String]) -> Result<String> {
        tracing::info!("Fetching AI PR analysis");

        let adrs = adrs.join("\n");

        let input = format!("<Diff>{diff}</Diff><Adrs>{adrs}</Adrs>");

        Self::prompt(prompts::PR_ADR_ANALYSIS, input).await
    }
}

trait Ai {
    async fn prompt(system_prompt: &str, input: String) -> Result<String> {
        let response = Self::make_request(system_prompt, input).await?;

        Ok(Self::extract_output(response))
    }

    async fn make_request(system_prompt: &str, input: String) -> Result<String>;

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
