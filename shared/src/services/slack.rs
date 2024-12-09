use super::{
    github::{PullRequest, WorkflowRun},
    jira::Issue,
};
use crate::{
    ai::ReleaseNotes,
    utils::{config, error::AppError},
};
use serde_json::{json, Value};

pub struct MessageInput<'a> {
    pub app_name: Option<&'a str>,
    pub jira_issues: Option<Vec<Issue>>,
    pub pull_requests: Vec<PullRequest>,
    pub prev_run: &'a WorkflowRun,
    pub run: &'a WorkflowRun,
    pub summary: ReleaseNotes,
}

pub async fn post_release_message(
    MessageInput {
        app_name,
        jira_issues,
        prev_run,
        pull_requests,
        run: workflow_run,
        summary,
    }: MessageInput<'_>,
) -> Result<(), AppError> {
    let send_slack_msg = config::get("SLACK_MESSAGE_ENABLED") == "true";

    if !send_slack_msg {
        println!("------ SLACK MESSAGE ------");
        println!("{summary:#?}");
        println!("---- END SLACK MESSAGE ----");
        return Ok(());
    }

    tracing::info!("Posting slack message");

    let webhook_url = config::get("SLACK_WEBHOOK_URL");

    let mut message_blocks: Vec<serde_json::Value> =
        Vec::from([get_header_block(app_name, workflow_run)]);

    if !summary.items.is_empty() {
        message_blocks.push(json!({ "type": "divider" }));

        for block in get_summary_block(&summary) {
            message_blocks.push(block);
        }
    }

    if !jira_issues.as_ref().map_or(false, |i| !i.is_empty()) || !pull_requests.is_empty() {
        message_blocks.push(json!({ "type": "divider" }));
    }

    if !pull_requests.is_empty() {
        message_blocks.push(get_pull_requests_block(pull_requests));
    }

    if let Some(jira_issues) = jira_issues {
        if !jira_issues.is_empty() {
            message_blocks.push(get_jira_tickets_block(jira_issues));
        }
    }

    message_blocks.push(get_actions_block(workflow_run, prev_run));
    message_blocks.push(json!({ "type": "divider" }));
    message_blocks.push(get_deployed_by_block(workflow_run));

    reqwest::Client::new()
        .put(webhook_url)
        .json(&json!({"blocks": json!(message_blocks)}))
        .send()
        .await?
        .error_for_status()
        .inspect_err(|e| tracing::error!("Error posting Slack message: {e}"))?;

    Ok(())
}

fn get_header_block(app_name: Option<&str>, run: &WorkflowRun) -> serde_json::Value {
    let app_name = app_name
        .map(|a| a.to_string())
        .unwrap_or_else(|| uppercase_first_letter(&run.repository.name));

    json!({
        "type": "header",
        "text": {
            "type": "plain_text",
            "text": format!("{app_name} release :rocket:",),
            "emoji": true
        }
    })
}

fn get_summary_block(release_notes: &ReleaseNotes) -> Vec<Value> {
    let mut blocks = Vec::new();

    for category in &release_notes.items {
        let items = category
            .items
            .iter()
            .map(|note| format!(r"  •  {note}"))
            .collect::<Vec<_>>()
            .join("\n");

        blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("*{}:*\n{items}", category.title),
            }
        }));
    }

    blocks
}

fn get_pull_requests_block(pull_requests: Vec<PullRequest>) -> serde_json::Value {
    json!({
        "type": "rich_text",
        "elements": [
            {
                "type": "rich_text_section",
                "elements": [
                    {
                        "type": "text",
                        "text": "Pull requests:",
                        "style": {
                            "bold": true
                        }
                    }
                ]
            },
            {
                "type": "rich_text_list",
                "style": "bullet",
                "elements": pull_requests
                .iter()
                .map(|pr| {
                    json!({
                        "type": "rich_text_section",
                        "elements": [
                            {
                                "type": "link",
                                "text": format!("#{} {}", pr.number, pr.title),
                                "url": pr.html_url,
                            }
                        ]
                    })
                })
                .collect::<Vec<_>>()
            }
        ]
    })
}

fn get_jira_tickets_block(jira_issues: Vec<Issue>) -> serde_json::Value {
    json!({
        "type": "rich_text",
        "elements": [
            {
                "type": "rich_text_section",
                "elements": [
                    {
                        "type": "text",
                        "text": "Jira tickets:",
                        "style": {
                            "bold": true
                        }
                    }
                ]
            },
            {
                "type": "rich_text_list",
                "style": "bullet",
                "elements": jira_issues
                .iter()
                .map(|issue| {
                    json!({
                        "type": "rich_text_section",
                        "elements": [
                            {
                                "type": "link",
                                "text": format!("{} {}", issue.key, issue.fields.summary),
                                "url": issue.get_browse_url(),
                            }
                        ]
                    })
                })
                .collect::<Vec<_>>()
            }
        ]
    })
}

fn get_actions_block(run: &WorkflowRun, prev_run: &WorkflowRun) -> serde_json::Value {
    json!({
        "type": "actions",
        "elements": [
            {
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Deployment",
                },
                "url": run.get_run_url()
            },
            {
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Diff",
                },
                "url": run.repository.get_compare_url(&prev_run.head_sha, &run.head_sha)
            },
            {
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Rollback",
                },
                "url": prev_run.get_run_url()
            }
        ]
    })
}

fn get_deployed_by_block(run: &WorkflowRun) -> serde_json::Value {
    json!({
        "type": "context",
        "elements": [
            {
                "type": "mrkdwn",
                "text": format!("*Deployed by:*")
            },
            {
                "type": "image",
                "image_url": run.actor.avatar_url,
                "alt_text": run.actor.login
            },
            {
                "type": "mrkdwn",
                "text": run.actor.login
            },
        ]
    })
}

fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
