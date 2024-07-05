use crate::{
    ai,
    middleware::validation::GithubEvent,
    services::{
        github::{WorkflowRun, WorkflowRuns},
        jira::Issue,
        slack, Git,
    },
    utils::error::AppError,
};
use anyhow::Result;
use axum::extract::Query;
use futures::future::try_join_all;
use hyper::StatusCode;
use regex_lite::Regex;
use serde::Deserialize;
use std::{collections::HashSet, sync::OnceLock};

pub async fn post(
    Query(Config { is_mono_repo }): Query<Config>,
    GithubEvent(WorkflowEvent { workflow_run: run }): GithubEvent<WorkflowEvent>,
) -> Result<StatusCode, AppError> {
    if !run.is_on_master() || !run.is_first_successful_attempt().await? {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = WorkflowRuns::get_prev_successful_run(&run).await? else {
        return Ok(StatusCode::OK);
    };

    let repo = Git::init(&run.repository)?;

    let new_commit = &run.head_sha;
    let old_commit = &prev_run.head_sha;

    let app_name = is_mono_repo
        .unwrap_or(false)
        .then(|| run.get_mono_app_name())
        .flatten();

    let Some(diff) = repo.diff(new_commit, old_commit, app_name)? else {
        return Ok(StatusCode::OK);
    };

    let commit_messages = repo.get_commit_messages(old_commit, new_commit, app_name)?;
    let jira_issues = get_jira_issues(&commit_messages).await?;

    let summary = ai::summarise_release(&diff, &commit_messages).await?;

    slack::post_release_message(slack::MessageInput {
        is_mono_repo,
        app_name,
        summary,
        jira_issues,
        run: &run,
        prev_run: &prev_run,
    })
    .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct WorkflowEvent {
    pub workflow_run: WorkflowRun,
}

static JIRA_TICKET_REGEX: OnceLock<Regex> = OnceLock::new();

async fn get_jira_issues(commit_messages: &[String]) -> Result<Vec<Issue>> {
    let issue_key_regex = JIRA_TICKET_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let jira_requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| issue_key_regex.find(m).map(|i| i.as_str()))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(Issue::get_by_key)
        .collect();

    let issues = try_join_all(jira_requests).await?.into_iter().collect();

    Ok(issues)
}

#[derive(Deserialize)]
pub struct Config {
    pub is_mono_repo: Option<bool>,
}
