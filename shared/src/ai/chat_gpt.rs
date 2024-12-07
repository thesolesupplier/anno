use super::{prompts, schemas};
use crate::{services::jira::IssueComment, utils::config};
use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

pub struct ChatGpt;

impl ChatGpt {
    pub async fn get_release_summary(
        diff: &str,
        commit_messages: &[String],
    ) -> Result<ReleaseNotes> {
        tracing::info!("Fetching AI release summary");

        let commit_messages = commit_messages.join("\n");

        let user_prompt = format!(
            "<Diff>{diff}</Diff>
            <CommitMessages>{commit_messages}</CommitMessages>"
        );

        Self::make_request(
            prompts::RELEASE_SUMMARY,
            user_prompt,
            Some(schemas::release_summary_response()),
        )
        .await
    }

    pub async fn get_test_cases(
        issue_description: &str,
        user_comments: &[IssueComment],
    ) -> Result<TestCases> {
        tracing::info!("Fetching AI Jira issue test cases");

        let comments = user_comments.iter().fold(String::new(), |acc, c| {
            acc + "Author" + &c.author.display_name + "\nComment" + &c.body + "\n--"
        });

        let user_prompt = format!(
            "<IssueDescription>{issue_description}</IssueDescription>
            <IssueComments>{comments}</IssueComments>"
        );

        Self::make_request(
            prompts::JIRA_ISSUE_TEST_CASES,
            user_prompt,
            Some(schemas::test_cases_response()),
        )
        .await
    }

    async fn make_request<T: DeserializeOwned>(
        system_input: &'static str,
        user_input: String,
        response_format: Option<Value>,
    ) -> Result<T> {
        let base_url = config::get("CHAT_GPT_BASE_URL");
        let api_key = config::get("CHAT_GPT_API_KEY");
        let model = config::get_optional("CHAT_GPT_MODEL");

        let response = reqwest::Client::new()
            .post(format!("{base_url}/chat/completions"))
            .header("content-type", "application/json")
            .bearer_auth(api_key)
            .json(&json!({
                "model": model.unwrap_or_else(|| "gpt-4o".to_string()),
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

        let release_notes: T = serde_json::from_str(&response)?;

        Ok(release_notes)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReleaseNotes {
    pub items: Vec<ReleaseNoteCategory>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReleaseNoteCategory {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TestCases {
    pub cases: Vec<String>,
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
