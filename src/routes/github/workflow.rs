use crate::{
    ai::{self, ReleaseSummary},
    middleware::validation::GithubEvent,
    services::{
        github::{PullRequest, Repository, WorkflowRun, WorkflowRuns},
        jira::Issue,
        slack, Git,
    },
    utils::error::AppError,
};
use anyhow::Result;
use chrono::{DateTime, Duration, SecondsFormat};
use futures::future::try_join_all;
use hyper::StatusCode;
use regex_lite::Regex;
use serde::Deserialize;
use std::{collections::HashSet, sync::OnceLock};

pub async fn release_summary(
    GithubEvent(WorkflowEvent {
        workflow_run: run,
        repository,
    }): GithubEvent<WorkflowEvent>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Processing {} run {}", run.repository.name, run.name);

    if !run.is_on_master() || !run.is_first_successful_attempt().await? {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = WorkflowRuns::get_prev_successful_run(&run).await? else {
        return Ok(StatusCode::OK);
    };

    let workflow_config = run.get_config().await?;
    let app_name = workflow_config.get_app_name();
    let target_paths = workflow_config.get_target_paths();

    let new_commit = &run.head_sha;
    let old_commit = &prev_run.head_sha;

    let diff = run
        .repository
        .fetch_diff(old_commit, new_commit, &target_paths)
        .await?;

    let commit_messages = if repository.is_too_large_to_clone() {
        let from = &increment_by_one_second(&prev_run.head_commit.timestamp)?;
        let to = &run.head_commit.timestamp;

        run.repository
            .fetch_commit_messages_in_range((from, to), &target_paths)
            .await?
    } else {
        Git::init(&run.repository.full_name)
            .await?
            .get_commit_messages(old_commit, new_commit, &target_paths)?
    };

    let jira_issues = get_jira_issues(&commit_messages).await?;
    let pull_requests = get_pull_requests(&run.repository, &commit_messages).await?;
    let summary = ai::ChatGpt::get_release_summary(&diff, &commit_messages).await?;

    slack::post_release_message(slack::MessageInput {
        app_name,
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
    pub repository: WorkflowEventRepository,
}

#[derive(Deserialize)]
pub struct WorkflowEventRepository {
    size: u64,
}

impl WorkflowEventRepository {
    pub fn is_too_large_to_clone(&self) -> bool {
        self.size > 60_000
    }
}

static JIRA_ISSUE_REGEX: OnceLock<Regex> = OnceLock::new();

async fn get_jira_issues(commit_messages: &[String]) -> Result<Vec<Issue>> {
    let issue_regex = JIRA_ISSUE_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| issue_regex.find(m).map(|i| i.as_str()))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(Issue::get_by_key)
        .collect();

    let mut issues: Vec<_> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    issues.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(issues)
}

static PR_REGEX: OnceLock<Regex> = OnceLock::new();

async fn get_pull_requests<'a>(
    repo: &'a Repository,
    commit_messages: &'a [String],
) -> Result<Vec<PullRequest>> {
    let pr_regex = PR_REGEX.get_or_init(|| Regex::new(r"#(\d+)").unwrap());

    let requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| pr_regex.captures(m).and_then(|c| Some(c.get(1)?.as_str())))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(|id| repo.get_pull_request(id))
        .collect();

    let mut pull_requests: Vec<_> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    pull_requests.sort_by_key(|pr| pr.number);

    Ok(pull_requests)
}

fn increment_by_one_second(date: &str) -> Result<String> {
    let new_datetime = DateTime::parse_from_rfc3339(date)? + Duration::seconds(1);

    Ok(new_datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
}
