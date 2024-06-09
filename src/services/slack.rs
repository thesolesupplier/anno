use serde_json::json;

use crate::utils::error::AppError;
use std::env;

pub async fn post_message(
    message: &str,
    workflow_url: &str,
    compare_url: String,
) -> Result<(), AppError> {
    let url = env::var("SLACK_WEBHOOK_URL").expect("SLACK_WEBHOOK_URL should be set");

    reqwest::Client::new()
        .put(url)
        .json(&json!({
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": "Bolt Release",
                        "emoji": true
                    }
                },
                {
                    "type": "divider"
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": message
                    }
                },
                {
                    "type": "actions",
                    "elements": [
                        {
                            "type": "button",
                            "text": {
                                "type": "plain_text",
                                "text": "View deployment",
                                "emoji": true
                            },
                            "url": workflow_url
                        },
                        {
                            "type": "button",
                            "text": {
                                "type": "plain_text",
                                "text": "View diff",
                                "emoji": true
                            },
                            "url": compare_url
                        }
                    ]
                }
            ]
        }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}
