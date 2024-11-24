use anyhow::Result;
use common::{
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

    let repo = config::get("GITHUB_REPOSITORY").unwrap();
    let run_id = config::get("GITHUB_RUN_ID").unwrap();
    let app_name = config::get("APP_NAME").ok();

    let run = WorkflowRun::get_by_id(&repo, &run_id).await?;

    if !run.is_first_successful_attempt().await? {
        tracing::info!("Unsuccessful attempt, skipping");
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

    let jira_issues = commits::get_jira_issues(&commit_messages).await?;
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
