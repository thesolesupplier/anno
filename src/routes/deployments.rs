use crate::{
    services::{chat_gpt, Git},
    utils::error::AppError,
};
use axum::Json;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

pub async fn post_deployment(
    Json(deployment): Json<Deployment>,
) -> Result<(StatusCode, Json<ResponseData>), AppError> {
    let summary = deployment.get_diff_summary().await?;

    Ok((StatusCode::OK, Json(ResponseData { summary })))
}

#[derive(Deserialize)]
pub struct Deployment {
    head_commit: Commit,
    repository: Repository,
}

impl Deployment {
    pub async fn get_diff_summary(&self) -> Result<String, AppError> {
        let repo_path = &self.repository.full_name;

        let diff = Git::new(repo_path)?.get_diff_with_head(&self.head_commit.id)?;
        let summary = chat_gpt::get_diff_summary(&diff).await?;

        Ok(summary)
    }
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
}

#[derive(Deserialize)]
struct Commit {
    id: String,
}

#[derive(Serialize)]
pub struct ResponseData {
    summary: String,
}
