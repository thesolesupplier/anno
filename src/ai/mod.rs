mod chat_gpt;
mod claude;
mod prompts;

use crate::services::jira::IssueComment;
use anyhow::Result;
pub use chat_gpt::ChatGpt;
pub use claude::Claude;
use regex_lite::Regex;
use std::sync::OnceLock;

static OUTPUT_REGEX: OnceLock<Regex> = OnceLock::new();

pub trait Ai {
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

pub trait ReleaseSummary: Ai {
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

pub trait PrBugAnalysis: Ai {
    async fn get_pr_bug_analysis(diff: &str) -> Result<String> {
        tracing::info!("Fetching AI PR bug analysis");

        let user_prompt = format!("<Diff>{diff}</Diff>");

        Self::prompt(prompts::PR_BUG_ANALYSIS, user_prompt).await
    }
}

pub trait PrAdrAnalysis: Ai {
    async fn get_pr_adr_analysis(
        PrAdrAnalysisInput {
            diff,
            adrs,
            commit_messages,
            pr_body,
        }: PrAdrAnalysisInput<'_>,
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

pub trait IssueTestCasing: Ai {
    async fn get_test_cases(
        issue_description: &str,
        user_comments: &[IssueComment],
    ) -> Result<String> {
        tracing::info!("Fetching AI Jira issue test cases");

        let comments = user_comments.iter().fold(String::new(), |acc, c| {
            acc + "Author" + &c.author.display_name + "\nComment" + &c.body + "\n--"
        });

        let user_prompt = format!(
            "<IssueDescription>{issue_description}</IssueDescription>
            <IssueComments>{comments}</IssueComments>"
        );

        Self::prompt(prompts::JIRA_ISSUE_TEST_CASES, user_prompt).await
    }
}

pub struct PrAdrAnalysisInput<'a> {
    pub diff: &'a str,
    pub adrs: &'a [String],
    pub commit_messages: &'a [String],
    pub pr_body: &'a Option<String>,
}

impl<T> ReleaseSummary for T where T: Ai {}
impl<T> PrAdrAnalysis for T where T: Ai {}
impl<T> PrBugAnalysis for T where T: Ai {}
impl<T> IssueTestCasing for T where T: Ai {}
