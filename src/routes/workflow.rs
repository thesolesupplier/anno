use crate::{
    ai,
    middleware::validation::GithubEvent,
    services::{
        github::{PullRequest, Repository, WorkflowRun, WorkflowRuns},
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
    tracing::info!("Processing '{}' run", run.repository.name);

    if !run.is_on_master() || !run.is_first_successful_attempt().await? {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = WorkflowRuns::get_prev_successful_run(&run).await? else {
        return Ok(StatusCode::OK);
    };

    let repo = Git::init(&run.repository.full_name, None).await?;

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
    let pull_requests = get_pull_requests(&run.repository, &commit_messages).await?;

    let summary = ai::get_release_summary(&diff, &commit_messages).await?;

    slack::post_release_message(slack::MessageInput {
        app_name,
        is_mono_repo,
        jira_issues,
        prev_run: &prev_run,
        pull_requests,
        run: &run,
        summary,
    })
    .await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct WorkflowEvent {
    pub workflow_run: WorkflowRun,
}

#[derive(Deserialize)]
pub struct Config {
    pub is_mono_repo: Option<bool>,
}

static JIRA_ISSUE_REGEX: OnceLock<Regex> = OnceLock::new();

async fn get_jira_issues(commit_messages: &[String]) -> Result<Vec<Issue>> {
    tracing::info!("Fetching JIRA issues");

    let issue_regex = JIRA_ISSUE_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| issue_regex.find(m).map(|i| i.as_str()))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(Issue::get_by_key)
        .collect();

    let issues = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    Ok(issues)
}

static PR_REGEX: OnceLock<Regex> = OnceLock::new();

async fn get_pull_requests<'a>(
    repo: &'a Repository,
    commit_messages: &'a [String],
) -> Result<Vec<PullRequest>> {
    tracing::info!("Fetching Pull Requests");

    let pr_regex = PR_REGEX.get_or_init(|| Regex::new(r"#(\d+)").unwrap());

    let requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| pr_regex.captures(m).and_then(|c| Some(c.get(1)?.as_str())))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(|id| repo.get_pull_request(id))
        .collect();

    let pull_requests = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    Ok(pull_requests)
}
