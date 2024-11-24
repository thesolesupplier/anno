use crate::middleware::validation::JiraEvent;
use common::{
    ai::{ChatGpt, IssueTestCasing},
    services::jira::Issue,
    utils::error::AppError,
};
use hyper::StatusCode;
use serde::Deserialize;

pub async fn status(JiraEvent(event): JiraEvent<JiraIssueEvent>) -> Result<StatusCode, AppError> {
    if !event.should_trigger_test_cases() {
        return Ok(StatusCode::OK);
    }

    let Some(issue_description) = &event.issue.fields.description else {
        return Ok(StatusCode::OK);
    };

    if issue_description.is_empty() {
        return Ok(StatusCode::OK);
    }

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
    pub fn should_trigger_test_cases(&self) -> bool {
        self.changelog
            .items
            .iter()
            .any(|i| i.is_to_refinement_status() || i.is_to_holding_bay_status())
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
    from_string: Option<String>,
}

impl JiraChangeLogItem {
    pub fn is_to_refinement_status(&self) -> bool {
        (self.is_from_status("Next")
            || self.is_from_status("Ready to Scope")
            || self.is_from_status("Ready to Refine"))
            && self.is_to_status("In Refinement")
    }

    pub fn is_to_holding_bay_status(&self) -> bool {
        !self.is_from_status("Ready to Dev") && self.is_to_status("Holding Bay")
    }

    fn is_to_status(&self, status: &str) -> bool {
        self.field == "status"
            && self
                .to_string
                .as_ref()
                .map(|s| s == status)
                .unwrap_or(false)
    }

    fn is_from_status(&self, status: &str) -> bool {
        self.field == "status"
            && self
                .from_string
                .as_ref()
                .map(|s| s == status)
                .unwrap_or(false)
    }
}
