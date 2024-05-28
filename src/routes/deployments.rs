use crate::{
    services::{chat_gpt, Git},
    utils::error::AppError,
};
use axum::Json;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Commit {
    id: String,
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
}

#[derive(Deserialize)]
pub struct Deployment {
    head_commit: Commit,
    repository: Repository,
}

#[derive(Serialize)]
pub struct ResponseData {
    summary: String,
}

pub async fn post_deployment(
    Json(deployment): Json<Deployment>,
) -> Result<(StatusCode, Json<ResponseData>), AppError> {
    let repo_path = deployment.repository.full_name;

    let diff = Git::new(&repo_path)?.get_diff_with_head(&deployment.head_commit.id)?;
    let summary = chat_gpt::get_diff_summary(&diff).await?;

    Ok((StatusCode::OK, Json(ResponseData { summary })))
}
