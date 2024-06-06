use crate::{
    middleware::validation::GithubEvent,
    services::{chat_gpt, github::Workflow, Git},
    utils::error::AppError,
};
use hyper::StatusCode;

pub async fn post(GithubEvent(workflow): GithubEvent<Workflow>) -> Result<StatusCode, AppError> {
    if !workflow.is_successful_run() {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = workflow.get_prev_successful_run().await? else {
        return Ok(StatusCode::OK);
    };

    let repo = Git::init(&workflow.repository.full_name)?;

    let Some(diff) = repo.diff(&workflow.workflow_run.head_sha, &prev_run.head_sha)? else {
        return Ok(StatusCode::OK);
    };

    let summary = chat_gpt::get_diff_summary(&diff).await?;

    println!("Summary: {summary}");

    Ok(StatusCode::OK)
}
