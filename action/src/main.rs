use anyhow::Result;
use shared::{
    ai::{self, ReleaseSummary},
    services::{
        github::{WorkflowRun, WorkflowRuns},
        slack, Git,
    },
    utils::{commits, config, error::AppError},
};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    config::load();

    let repo = config::get("GITHUB_REPOSITORY");
    let run_id = config::get("GITHUB_RUN_ID");
    let app_name = config::get_optional("APP_NAME");
    let jira_integration =
        config::get_optional("JIRA_INTEGRATION_ENABLED").map_or(false, |v| v == "true");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let run = WorkflowRun::get_by_id(&repo, &run_id).await?;

    if run.has_prev_successful_attempt().await? {
        tracing::info!("Already previously deployed, skipping");
        return Ok(());
    }

    let Some(prev_run) = WorkflowRuns::get_prev_successful_run(&run).await? else {
        tracing::info!("No previous successful run, skipping");
        return Ok(());
    };

    let target_paths = run.get_config().await?.get_target_paths();

    let new_commit = &run.head_sha;
    let old_commit = &prev_run.head_sha;

    let diff = run
        .repository
        .fetch_diff(old_commit, new_commit, &target_paths)
        .await?;

    let commit_messages = Git::init(&run.repository.full_name)
        .await?
        .get_commit_messages(old_commit, new_commit, &target_paths)?;

    let jira_issues = if jira_integration {
        Some(commits::get_jira_issues(&commit_messages).await?)
    } else {
        None
    };

    let pull_requests = commits::get_pull_requests(&run.repository, &commit_messages).await?;
    let summary = ai::ChatGpt::get_release_summary(&diff, &commit_messages).await?;

    slack::post_release_message(slack::MessageInput {
        app_name: app_name.as_deref(),
        jira_issues,
        prev_run: &prev_run,
        pull_requests,
        run: &run,
        summary,
    })
    .await?;

    Ok(())
}
