use super::{github::WorkflowRun, jira::Issue};
use crate::utils::error::AppError;
use serde_json::{json, Value};
use std::env;

pub struct MessageInput<'a> {
    pub message: String,
    pub jira_issues: Vec<Issue>,
    pub run: &'a WorkflowRun,
    pub prev_run: &'a WorkflowRun,
    pub app_name: Option<&'a str>,
}

pub async fn post_release_message(
    MessageInput {
        message,
        jira_issues,
        run: workflow_run,
        prev_run,
        app_name,
    }: MessageInput<'_>,
) -> Result<(), AppError> {
    let send_slack_msg = env::var("SLACK_MESSAGE_ENABLED").is_ok_and(|v| v == "true");

    if !send_slack_msg {
        println!("------ SLACK MESSAGE ------");
        println!("{message}");
        println!("------ END SLACK MESSAGE ------");
        return Ok(());
    }

    let url = env::var("SLACK_WEBHOOK_URL").expect("SLACK_WEBHOOK_URL should be set");

    let app_name = app_name
        .map(|a| a.to_string())
        .unwrap_or_else(|| uppercase_first_letter(&workflow_run.repository.name));

    let mut message_blocks: Vec<serde_json::Value> = Vec::from([
        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": format!("{app_name} release 🚀",),
                "emoji": true
            }
        }),
        json!({ "type": "divider" }),
        json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": message
            }
        }),
    ]);

    if !jira_issues.is_empty() {
        message_blocks.push(json!({ "type": "divider" }));
        message_blocks.push(json!({
            "type": "rich_text",
            "elements": [
                {
                    "type": "rich_text_section",
                    "elements": [
                        {
                            "type": "text",
                            "text": "Jira tickets:\n",
                            "style": {
                                "bold": true
                            }
                        }
                    ]
                },
                {
                    "type": "rich_text_list",
                    "style": "bullet",
                    "elements": format_jira_links(jira_issues)
                }
            ]
        }));
    }

    message_blocks.push(json!({
        "type": "actions",
        "elements": [
            {
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "View deployment",
                },
                "url": workflow_run.get_run_url()
            },
            {
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "View diff",
                },
                "url": workflow_run.repository.get_compare_url(&prev_run.head_sha, &workflow_run.head_sha)
            }
        ]
    }));

    reqwest::Client::new()
        .put(url)
        .json(&json!({"blocks": json!(message_blocks)}))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

fn format_jira_links(jira_issues: Vec<Issue>) -> Vec<Value> {
    jira_issues
        .iter()
        .map(|issue| {
            json!({
                "type": "rich_text_section",
                "elements": [
                    {
                        "type": "link",
                        "text": format!("{} {}", issue.key, issue.fields.summary),
                        "url": issue.get_browse_url(),
                        "style": {
                            "bold": true
                        }
                    }
                ]
            })
        })
        .collect::<Vec<_>>()
}

fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
