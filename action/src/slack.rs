use super::workflows::WorkflowRun;
use crate::ai;
use serde_json::{json, Value};
use shared::{
    services::{github::PullRequest, jira::Issue},
    utils::{config, error::AppError},
};

pub struct ReleaseSummary<'a> {
    pub app_name: Option<&'a str>,
    pub jira_issues: Vec<Issue>,
    pub diff_url: String,
    pub prev_run_url: Option<&'a String>,
    pub pull_requests: Vec<PullRequest>,
    pub run: &'a WorkflowRun,
    pub summary: ai::ReleaseSummary,
}

impl ReleaseSummary<'_> {
    pub async fn send(&self) -> Result<(), AppError> {
        let send_slack_msg = config::get("SLACK_MESSAGE_ENABLED") == "true";

        if !send_slack_msg {
            println!("{:#?}", self.summary);
            return Ok(());
        }

        tracing::info!("Posting release summary to Slack");

        let mut message_blocks = Vec::from([self.get_header_block(), json!({ "type": "divider" })]);

        message_blocks.extend(self.get_summary_block());

        if !self.jira_issues.is_empty() || !self.pull_requests.is_empty() {
            message_blocks.push(json!({ "type": "divider" }));
        }

        if !self.pull_requests.is_empty() {
            message_blocks.push(self.get_pull_requests_block());
        }

        if !self.jira_issues.is_empty() {
            message_blocks.push(self.get_jira_tickets_block());
        }

        message_blocks.push(self.get_actions_block());
        message_blocks.push(json!({ "type": "divider" }));
        message_blocks.push(self.get_deployed_by_block());

        reqwest::Client::new()
            .put(config::get("SLACK_WEBHOOK_URL"))
            .json(&json!({"blocks": json!(message_blocks)}))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error posting Slack message: {e}"))?;

        Ok(())
    }

    fn get_header_block(&self) -> Value {
        let app_name = self.app_name.unwrap_or(&self.run.repository.name);

        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": format!("{app_name} release :rocket:"),
                "emoji": true
            }
        })
    }

    fn get_summary_block(&self) -> Vec<Value> {
        let mut blocks = Vec::new();

        for category in &self.summary.items {
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
                    "text": format!("*{}*\n{items}", category.title),
                }
            }));
        }

        blocks
    }

    fn get_pull_requests_block(&self) -> Value {
        json!({
            "type": "rich_text",
            "elements": [
                {
                    "type": "rich_text_section",
                    "elements": [
                        {
                            "type": "text",
                            "text": "Pull requests",
                            "style": {
                                "bold": true
                            }
                        }
                    ]
                },
                {
                    "type": "rich_text_list",
                    "style": "bullet",
                    "elements": self.pull_requests
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

    fn get_jira_tickets_block(&self) -> Value {
        json!({
            "type": "rich_text",
            "elements": [
                {
                    "type": "rich_text_section",
                    "elements": [
                        {
                            "type": "text",
                            "text": "Jira tickets",
                            "style": {
                                "bold": true
                            }
                        }
                    ]
                },
                {
                    "type": "rich_text_list",
                    "style": "bullet",
                    "elements": self.jira_issues
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

    fn get_actions_block(&self) -> Value {
        let mut elements = Vec::from([
            json!({
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Deployment",
                },
                "url": self.run.get_run_url()
            }),
            json!({
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Diff",
                },
                "url": self.diff_url
            }),
        ]);

        if let Some(prev_run_url) = self.prev_run_url {
            elements.push(json!({
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Rollback",
                },
                "url": prev_run_url
            }));
        }

        json!({
            "type": "actions",
            "elements": elements
        })
    }

    fn get_deployed_by_block(&self) -> Value {
        json!({
            "type": "context",
            "elements": [
                {
                    "type": "mrkdwn",
                    "text": format!("*Deployed by:*")
                },
                {
                    "type": "image",
                    "image_url": self.run.actor.avatar_url,
                    "alt_text": self.run.actor.login
                },
                {
                    "type": "mrkdwn",
                    "text": self.run.actor.login
                },
            ]
        })
    }
}
