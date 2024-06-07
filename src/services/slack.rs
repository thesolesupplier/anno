use serde_json::json;

use crate::utils::error::AppError;
use std::env;

pub async fn post_message(message: &str) -> Result<(), AppError> {
    let url = env::var("SLACK_WEBHOOK_URL").expect("SLACK_WEBHOOK_URL should be set");

    reqwest::Client::new()
        .post(url)
        .json(&json!({ "text": message }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}
