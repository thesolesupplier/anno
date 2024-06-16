use crate::{
    middleware::validation::GithubEvent,
    services::{ai, github::WorkflowEvent, jira::Issue, slack, Git},
    utils::error::AppError,
};
use anyhow::Result;
use futures::future::try_join_all;
use hyper::StatusCode;
use regex_lite::Regex;
use std::sync::OnceLock;

pub async fn post(
    GithubEvent(workflow_event): GithubEvent<WorkflowEvent>,
) -> Result<StatusCode, AppError> {
    if !workflow_event.is_successful_release() {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = workflow_event.get_prev_successful_run().await? else {
        return Ok(StatusCode::OK);
    };

    let new_commit = &workflow_event.workflow_run.head_sha;
    let old_commit = &prev_run.head_sha;

    let repo = Git::init(&workflow_event.repository)?;

    let Some(diff) = repo.diff(new_commit, old_commit)? else {
        return Ok(StatusCode::OK);
    };

    let commit_messages = repo.get_commit_messages_between(old_commit, new_commit)?;
    let jira_issues = get_jira_issues(&commit_messages).await?;

    let summary = ai::summarise_release(&diff, &commit_messages).await?;

    slack::post_release_message(&summary, jira_issues, &workflow_event, &prev_run).await?;

    Ok(StatusCode::OK)
}

static JIRA_TICKET_REGEX: OnceLock<Regex> = OnceLock::new();

async fn get_jira_issues(commit_messages: &[String]) -> Result<Vec<Issue>> {
    let regex = JIRA_TICKET_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let jira_requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|message| {
            regex
                .find(message)
                .map(|ticket| Issue::get_by_key(ticket.as_str()))
        })
        .collect();

    let issues = try_join_all(jira_requests).await?.into_iter().collect();

    Ok(issues)
}
