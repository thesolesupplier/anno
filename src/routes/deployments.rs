use crate::{
    services::{chat_gpt, Git},
    utils::error::AppError,
};

use axum::Json;
use hyper::StatusCode;
use serde::Serialize;

// #[derive(Deserialize, Debug, Clone)]
// struct RequestData {
//     hook_id: u32,
// }

#[derive(Serialize, Debug, Clone)]
pub struct ResponseData {
    summary: String,
}

pub async fn post_deployment(// Json(payload): Json<RequestData>,
) -> Result<(StatusCode, Json<ResponseData>), AppError> {
    let repo_path = "constantincerdan/photography-website.git";

    let diff = Git::new(repo_path)?.get_diff_with_head("229d67b")?;
    let summary = chat_gpt::get_diff_summary(&diff).await?;

    Ok((StatusCode::OK, Json(ResponseData { summary })))
}
