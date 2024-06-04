use crate::{
    services::{
        chat_gpt,
        github::{WorkflowRun, WorkflowRuns},
        Git,
    },
    utils::error::AppError,
};
use axum::Json;
use hyper::StatusCode;
use serde::Deserialize;

pub async fn post_workflow(workflow: Json<Workflow>) -> Result<StatusCode, AppError> {
    if workflow.is_successful_run() {
        let summary = workflow.get_diff_summary().await?;

        println!("Summary: {}", summary);
    }

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct Workflow {
    action: String,
    repository: Repository,
    workflow_run: WorkflowRun,
}

impl Workflow {
    pub fn is_successful_run(&self) -> bool {
        self.action == "completed" && self.workflow_run.conclusion == "success"
    }

    pub async fn get_diff_summary(&self) -> Result<String, AppError> {
        let repo_path = &self.repository.full_name;

        let previous_run =
            WorkflowRuns::get_previous_successful_run(repo_path, &self.workflow_run.created_at)
                .await?;

        let diff =
            Git::init(repo_path)?.diff(&self.workflow_run.head_sha, &previous_run.head_sha)?;

        let summary = chat_gpt::get_diff_summary(&diff).await?;

        Ok(summary)
    }
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
}
