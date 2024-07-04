use super::{github::WorkflowRun, jira::Issue};
use crate::utils::{config, error::AppError};
use serde_json::{json, Value};

pub struct MessageInput<'a> {
    pub app_name: Option<&'a str>,
    pub jira_issues: Vec<Issue>,
    pub is_mono_repo: Option<bool>,
    pub prev_run: &'a WorkflowRun,
    pub run: &'a WorkflowRun,
    pub summary: String,
}

pub async fn post_release_message(
    MessageInput {
        app_name,
        jira_issues,
        is_mono_repo,
        prev_run,
        run: workflow_run,
        summary,
    }: MessageInput<'_>,
) -> Result<(), AppError> {
    let send_slack_msg = config::get("SLACK_MESSAGE_ENABLED").is_ok_and(|v| v == "true");

    if !send_slack_msg {
        println!("------ SLACK MESSAGE ------");
        println!("{summary}");
        println!("------ END SLACK MESSAGE ------");
        return Ok(());
    }

    let url = config::get("SLACK_WEBHOOK_URL")?;

    let app_name = app_name
        .map(|a| a.to_string())
        .unwrap_or_else(|| uppercase_first_letter(&workflow_run.repository.name));

    let mut message_blocks: Vec<serde_json::Value> = Vec::from([json!({
        "type": "header",
        "text": {
            "type": "plain_text",
            "text": format!("{app_name} release :rocket:",),
            "emoji": true
        }
    })]);

    if is_mono_repo.unwrap_or(false) {
        message_blocks.push(json!({
            "type": "context",
            "elements": [
                {
                    "type": "mrkdwn",
                    "text": format!("*Repo*: {}", workflow_run.repository.name)
                }
            ]
        }));
    }

    message_blocks.push(json!({ "type": "divider" }));
    message_blocks.push(json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": summary
        }
    }));

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

    message_blocks.push(json!({ "type": "divider" }));
    message_blocks.push(json!({
        "type": "context",
        "elements": [
            {
                "type": "mrkdwn",
                "text": "*Deployed by:*"
            },
            {
                "type": "image",
                "image_url": workflow_run.actor.avatar_url,
                "alt_text": "cute cat"
            },
            {
                "type": "mrkdwn",
                "text": workflow_run.actor.login
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
