mod ai;
mod git;
mod slack;
mod workflows;

use anyhow::Result;
use futures::future::{try_join, try_join_all};
use git::Git;
use regex_lite::Regex;
use shared::{
    services::{github::PullRequest, jira::Issue},
    utils::{config, error::AppError},
};
use std::collections::HashSet;
use workflows::{WorkflowRun, WorkflowRuns};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    config::load();

    let repo = config::get("GITHUB_REPOSITORY");
    let run_id = config::get("GITHUB_RUN_ID");
    let app_name = config::get_optional("APP_NAME");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let run = WorkflowRun::get_by_id(&repo, &run_id).await?;

    if run.has_prev_successful_attempt().await? {
        tracing::info!("Already previously deployed, skipping");
        return Ok(());
    }

    let Some(prev_runs) = WorkflowRuns::get_prev_and_last_successful_runs(&run).await? else {
        tracing::info!("No previous successful run, skipping");
        return Ok(());
    };

    let target_paths = run.get_config().await?.get_target_paths();

    let new_commit = &run.head_sha;
    let old_commit = &prev_runs.last_successful.head_sha;

    let mut diff = run.repository.fetch_diff(old_commit, new_commit).await?;

    if let Some(target_paths) = &target_paths {
        diff = target_paths.filter_diff(&diff);
    }

    let commit_messages = Git::init(&run.repository.full_name)
        .await?
        .get_commit_messages(old_commit, new_commit, &target_paths)?;

    let pull_requests = get_pull_requests(&run, &prev_runs.prev_runs).await?;

    let (jira_issues, summary) = try_join(
        get_jira_issues(&pull_requests, &commit_messages),
        ai::ReleaseSummary::new(&diff, &commit_messages),
    )
    .await?;

    slack::ReleaseSummary {
        app_name: app_name.as_deref(),
        jira_issues,
        prev_run: &prev_runs.last_successful,
        pull_requests,
        run: &run,
        summary,
    }
    .send()
    .await?;

    Ok(())
}

async fn get_pull_requests(
    curr_run: &WorkflowRun,
    prev_runs: &[WorkflowRun],
) -> Result<Vec<PullRequest>> {
    let mut pull_requests = curr_run
        .repository
        .get_pull_requests_for_commit(&curr_run.head_sha)
        .await?;

    for prev_run in prev_runs {
        let prs = curr_run
            .repository
            .get_pull_requests_for_commit(&prev_run.head_sha)
            .await?;

        pull_requests.extend(prs);
    }

    pull_requests.sort_by_key(|pr| pr.number);

    Ok(pull_requests)
}

pub async fn get_jira_issues(
    pull_requests: &[PullRequest],
    commit_messages: &[String],
) -> Result<Vec<Issue>> {
    let jira_enabled = config::get_optional("JIRA_API_KEY").is_some();

    if !jira_enabled {
        return Ok(Vec::new());
    }

    let project_key = config::get("JIRA_PROJECT_KEY");
    let key_regex = Regex::new(&format!(r"{project_key}-\d+")).expect("Valid regex");

    let mut keys = HashSet::new();

    for pr in pull_requests {
        if let Some(key) = key_regex.find(&pr.head.r#ref) {
            keys.insert(key.as_str());
        }

        let Some(body) = &pr.body else {
            continue;
        };

        for key in key_regex.find_iter(body) {
            keys.insert(key.as_str());
        }
    }

    for message in commit_messages {
        if let Some(key) = key_regex.find(message) {
            keys.insert(key.as_str());
        }
    }

    let requests = keys.into_iter().map(Issue::get_by_key).collect::<Vec<_>>();

    let mut issues: Vec<_> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    issues.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(issues)
}
