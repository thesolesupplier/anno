use crate::{ai, middleware::validation::JiraEvent};
use hyper::StatusCode;
use serde::Deserialize;
use shared::{services::jira::Issue, utils::error::AppError};

pub async fn test_cases(
    JiraEvent(event): JiraEvent<JiraIssueEvent>,
) -> Result<StatusCode, AppError> {
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

    let test_cases = ai::TestCases::new(issue_description, &issue_comments)
        .await?
        .into_jira_comment_body();

    event.issue.delete_anno_comments().await?;
    event.issue.add_comment(test_cases).await?;

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
        !self.is_from_status("Holding Bay")
            && !self.is_from_status("Review & Estimate")
            && self.is_to_status("In Refinement")
    }

    pub fn is_to_holding_bay_status(&self) -> bool {
        !self.is_from_status("Ready to Dev") && self.is_to_status("Holding Bay")
    }

    fn is_to_status(&self, status: &str) -> bool {
        self.field == "status" && self.to_string.as_ref().is_some_and(|s| s == status)
    }

    fn is_from_status(&self, status: &str) -> bool {
        self.field == "status" && self.from_string.as_ref().is_some_and(|s| s == status)
    }
}
