use crate::middleware::validation::GithubEvent;
use anyhow::Result;
use chrono::{DateTime, Duration, SecondsFormat};
use hyper::StatusCode;
use serde::Deserialize;
use shared::{
    ai::{self, ReleaseSummary},
    services::{
        github::{WorkflowRun, WorkflowRuns},
        slack, Git,
    },
    utils::{commits, config, error::AppError},
};

pub async fn release_summary(
    GithubEvent(WorkflowEvent {
        workflow_run: run,
        repository,
    }): GithubEvent<WorkflowEvent>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Processing {} run {}", run.repository.name, run.name);

    let jira_integration =
        config::get_optional("JIRA_INTEGRATION_ENABLED").map_or(false, |v| v == "true");

    if !run.is_first_successful_attempt().await? {
        tracing::info!("Unsuccessful attempt, skipping");
        return Ok(StatusCode::OK);
    }

    let workflow_config = run.get_config().await?;

    if !workflow_config.has_summary_enabled() {
        tracing::info!("Summary not enabled, skipping");
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = WorkflowRuns::get_prev_successful_run(&run).await? else {
        tracing::info!("No previous successful run, skipping");
        return Ok(StatusCode::OK);
    };

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

    let jira_issues = if jira_integration {
        Some(commits::get_jira_issues(&commit_messages).await?)
    } else {
        None
    };

    let pull_requests = commits::get_pull_requests(&run.repository, &commit_messages).await?;
    let summary = ai::ChatGpt::get_release_summary(&diff, &commit_messages).await?;

    slack::post_release_message(slack::MessageInput {
        app_name: workflow_config.get_app_name(),
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

fn increment_by_one_second(date: &str) -> Result<String> {
    let new_datetime = DateTime::parse_from_rfc3339(date)? + Duration::seconds(1);

    Ok(new_datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
}
