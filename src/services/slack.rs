use serde_json::json;

use super::github::{Workflow, WorkflowRun};
use crate::utils::error::AppError;
use std::env;

pub async fn post_release_message(
    message: &str,
    workflow: &Workflow,
    prev_run: &WorkflowRun,
) -> Result<(), AppError> {
    let url = env::var("SLACK_WEBHOOK_URL").expect("SLACK_WEBHOOK_URL should be set");

    let run_url = workflow.get_run_url();
    let compare_url = workflow.get_diff_url(&prev_run.head_sha);
    let header = format!(
        "{} release 🚀",
        uppercase_first_letter(&workflow.repository.name)
    );

    reqwest::Client::new()
        .put(url)
        .json(&json!({
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": header,
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
                            "url": run_url
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

fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
