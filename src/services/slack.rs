use super::github::{Workflow, WorkflowRun};
use crate::utils::error::AppError;
use serde_json::{json, Value};
use std::env;

pub async fn post_release_message(
    message: &str,
    jira_links: Vec<String>,
    workflow: &Workflow,
    prev_run: &WorkflowRun,
) -> Result<(), AppError> {
    let send_slack_msg = env::var("SLACK_MESSAGE_ENABLED").is_ok_and(|v| v == "true");

    if !send_slack_msg {
        println!("------ SLACK MESSAGE ------");
        println!("{message}");
        println!("------ END SLACK MESSAGE ------");
        return Ok(());
    }

    let url = env::var("SLACK_WEBHOOK_URL").expect("SLACK_WEBHOOK_URL should be set");

    reqwest::Client::new()
        .put(url)
        .json(&json!({
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": format!(
                            "{} release 🚀",
                            uppercase_first_letter(&workflow.repository.name)
                        ),
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
                    "type": "divider"
                },
                {
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
                            "elements": format_jira_links(jira_links)
                        }
                    ]
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
                            "url": workflow.get_run_url()
                        },
                        {
                            "type": "button",
                            "text": {
                                "type": "plain_text",
                                "text": "View diff",
                                "emoji": true
                            },
                            "url": workflow.get_diff_url(&prev_run.head_sha)
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

fn format_jira_links(jira_links: Vec<String>) -> Vec<Value> {
    jira_links
        .iter()
        .map(|link| {
            json!({
                "type": "rich_text_section",
                "elements": [
                    {
                        "type": "link",
                        "text": link,
                        "url": link
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
