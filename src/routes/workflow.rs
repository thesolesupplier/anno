use crate::{
    services::{chat_gpt, github::Workflow, Git},
    utils::error::AppError,
};
use axum::Json;
use hyper::StatusCode;

pub async fn post_workflow(workflow: Json<Workflow>) -> Result<StatusCode, AppError> {
    if workflow.is_successful_run() {
        let prev_run = workflow.get_prev_successful_run().await?;

        let repo = Git::init(&workflow.repository.full_name)?;
        let diff = repo.diff(&workflow.workflow_run.head_sha, &prev_run.head_sha)?;

        let summary = chat_gpt::get_diff_summary(&diff).await?;

        println!("Summary: {summary}");
    }

    Ok(StatusCode::OK)
}
