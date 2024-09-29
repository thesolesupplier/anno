use crate::{
    ai::{ChatGpt, IssueTestCasing},
    middleware::validation::JiraEvent,
    services::jira::Issue,
    utils::error::AppError,
};
use hyper::StatusCode;
use serde::Deserialize;

pub async fn status(JiraEvent(event): JiraEvent<JiraIssueEvent>) -> Result<StatusCode, AppError> {
    if !event.is_move_to_review_status() {
        return Ok(StatusCode::OK);
    }

    let issue_description = &event.issue.fields.description;
    let issue_comments = event.issue.get_user_comments().await?;

    let test_cases = ChatGpt::get_test_cases(issue_description, &issue_comments).await?;

    event.issue.delete_outdated_comments().await?;
    event.issue.add_comment(&test_cases).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct JiraIssueEvent {
    pub issue: Issue,
    pub changelog: JiraChangeLog,
}

impl JiraIssueEvent {
    pub fn is_move_to_review_status(&self) -> bool {
        self.changelog.items.iter().any(|i| i.is_review_status())
    }
}

#[derive(Deserialize)]
pub struct JiraChangeLog {
    pub items: Vec<JiraChangeLogItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraChangeLogItem {
    field: String,
    to_string: Option<String>,
}

impl JiraChangeLogItem {
    pub fn is_review_status(&self) -> bool {
        self.field == "status"
            && self
                .to_string
                .as_ref()
                .map(|s| s == "Review & Estimate")
                .unwrap_or(false)
    }
}
