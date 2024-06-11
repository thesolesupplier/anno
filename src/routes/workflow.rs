use crate::{
    middleware::validation::GithubEvent,
    services::{chat_gpt, github::Workflow, slack, Git},
    utils::error::AppError,
};
use hyper::StatusCode;
use std::env;

pub async fn post(GithubEvent(workflow): GithubEvent<Workflow>) -> Result<StatusCode, AppError> {
    let send_slack_msg = env::var("SLACK_MESSAGE_ENABLED").is_ok_and(|e| e == "true");

    if !workflow.is_pipeline_run() || !workflow.is_successful_run() {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = workflow.get_prev_successful_run().await? else {
        return Ok(StatusCode::OK);
    };

    let new_commit = &workflow.workflow_run.head_sha;
    let old_commit = &prev_run.head_sha;

    let repo = Git::init(&workflow)?;

    let Some(diff) = repo.diff(new_commit, old_commit)? else {
        return Ok(StatusCode::OK);
    };

    let commit_msgs = repo.get_commit_messages_between(old_commit, new_commit)?;

    let summary = chat_gpt::summarise_release(&diff, commit_msgs).await?;

    if send_slack_msg {
        slack::post_release_message(&summary, &workflow, &prev_run).await?;
    } else {
        println!("------ SUMMARY ------");
        println!("{summary}");
        println!("------ END SUMMARY ------");
    }

    Ok(StatusCode::OK)
}
