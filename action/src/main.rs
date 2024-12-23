use anyhow::Result;
use futures::future::try_join_all;
use regex_lite::Regex;
use shared::{
    ai,
    services::{
        github::{PullRequest, WorkflowRun, WorkflowRuns},
        jira::Issue,
        slack, Git,
    },
    utils::{config, error::AppError},
};
use std::collections::HashSet;

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

    let diff = run
        .repository
        .fetch_diff(old_commit, new_commit, &target_paths)
        .await?;

    let commit_messages = Git::init(&run.repository.full_name)
        .await?
        .get_commit_messages(old_commit, new_commit, &target_paths)?;

    let pull_requests = get_pull_requests(&run, &prev_runs.prev_runs).await?;
    let jira_issues = get_jira_issues(&pull_requests, &commit_messages).await?;
    let summary = ai::ChatGpt::get_release_summary(&diff, &commit_messages).await?;

    slack::post_release_summary(slack::MessageInput {
        app_name: app_name.as_deref(),
        jira_issues,
        prev_run: &prev_runs.last_successful,
        pull_requests,
        run: &run,
        summary,
    })
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
) -> Result<Option<Vec<Issue>>> {
    let jira_integration =
        config::get_optional("JIRA_INTEGRATION_ENABLED").is_some_and(|v| v == "true");

    if !jira_integration {
        return Ok(None);
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

    Ok(Some(issues))
}
