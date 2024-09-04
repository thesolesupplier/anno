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

pub async fn get_pr_adr_analysis(input: PrAnalysisInput<'_>) -> Result<String> {
    match config::get("LLM_PROVIDER")?.as_str() {
        "anthropic" => Claude::get_pr_adr_analysis(input).await,
        _ => ChatGpt::get_pr_adr_analysis(input).await,
    }
}

pub async fn get_pr_bug_analysis(diff: &str) -> Result<String> {
    match config::get("LLM_PROVIDER")?.as_str() {
        "anthropic" => Claude::get_pr_bug_analysis(diff).await,
        _ => ChatGpt::get_pr_bug_analysis(diff).await,
    }
}

impl<T> ReleaseSummary for T where T: Ai {}
impl<T> PrAdrAnalysis for T where T: Ai {}
impl<T> PrBugAnalysis for T where T: Ai {}

trait ReleaseSummary: Ai {
    async fn get_release_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
        tracing::info!("Fetching AI release summary");

        let commit_messages = commit_messages.join("\n");

        let user_prompt = format!(
            "<Diff>{diff}</Diff>
            <CommitMessages>{commit_messages}</CommitMessages>"
        );

        Self::prompt(prompts::RELEASE_SUMMARY, user_prompt).await
    }
}

trait PrBugAnalysis: Ai {
    async fn get_pr_bug_analysis(diff: &str) -> Result<String> {
        tracing::info!("Fetching AI PR bug analysis");

        let user_prompt = format!("<Diff>{diff}</Diff>");

        Self::prompt(prompts::PR_BUG_ANALYSIS, user_prompt).await
    }
}

trait PrAdrAnalysis: Ai {
    async fn get_pr_adr_analysis(
        PrAnalysisInput {
            diff,
            adrs,
            commit_messages,
            pr_body,
        }: PrAnalysisInput<'_>,
    ) -> Result<String> {
        tracing::info!("Fetching AI PR ADR analysis");

        let adrs = adrs.join("\n");
        let commit_messages = commit_messages.join("\n");

        let mut user_prompt = format!(
            "<Diff>{diff}</Diff>
            <Adrs>{adrs}</Adrs>
            <CommitMessages>{commit_messages}</CommitMessages>"
        );

        if let Some(pr_body) = pr_body {
            user_prompt.push_str(&format!("<PrDescription>{pr_body}</PrDescription>"));
        }

        Self::prompt(prompts::PR_ADR_ANALYSIS, user_prompt).await
    }
}

pub struct PrAnalysisInput<'a> {
    pub diff: &'a str,
    pub adrs: &'a [String],
    pub commit_messages: &'a [String],
    pub pr_body: &'a Option<String>,
}

trait Ai {
    async fn prompt(system_prompt: &str, user_prompt: String) -> Result<String> {
        let response = Self::make_request(system_prompt, user_prompt).await?;

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
