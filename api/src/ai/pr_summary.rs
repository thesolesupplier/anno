use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::services::{claude, jira::Issue};

#[derive(Deserialize)]
pub struct PrSummary {
    pub summary: String,
    pub details: Vec<String>,
}

impl PrSummary {
    pub async fn new(diff: &str, commit_messages: &[String], issues: &[Issue]) -> Result<Self> {
        tracing::info!("Generating PR summary");

        let commit_messages = commit_messages.join("\n");
        let issues = issues
            .iter()
            .filter(|i| i.fields.description.is_some())
            .map(|i| {
                format!(
                    "- [{}] {}\n{}",
                    i.key,
                    i.fields.summary,
                    i.fields
                        .description
                        .as_ref()
                        .expect("Description to exist because of filter")
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        let user_prompt = format!(
            "<Diff>{diff}</Diff>
             <CommitMessages>{commit_messages}</CommitMessages>
             <JiraIssues>{issues}</JiraIssues>"
        );

        claude::Request {
            user_prompt,
            system_prompt: SYSTEM_PROMPT,
            tool_schema: response_schema(),
            tool_name: "pr_summary",
            ..Default::default()
        }
        .send()
        .await
    }

    pub fn into_markdown_body(self) -> String {
        let mut body = format!(
            "#### Summary\n{}\n<details><summary>Details</summary><br>\n\n",
            self.summary
        );

        for detail in self.details {
            body.push_str(&format!("- {}\n", detail));
        }

        body.push_str("</details>");

        body
    }
}

fn response_schema() -> Value {
    let details = json!({
        "type": "array",
        "description": "An array of strings representing the technical details of the pull request. Use single backticks to format single line references to code if needed.",
        "items": {
            "type": "string"
        }
    });

    let summary = json!({
      "type": "string",
      "description": "A block of text summarising the pull request."
    });

    json!({
      "name": "pr_summary",
      "input_schema": {
        "type": "object",
        "properties": {
          "summary": summary,
          "details": details,
        },
        "required": [
          "summary",
          "details"
        ],
        "additionalProperties": false
      },
    })
}

const SYSTEM_PROMPT: &str = "
    <Instructions>
        Your task is to summarise a pull request to make it easier for other team members to understand the changes before reviewing.
        You should provide a high-level summary of the changes as well as a separate, detailed list breaking down the most important changes.
        Use the diff, commit messages, and Jira issues (if provided) to summarise the code changes and how they relate to the feature or bug described in the issues.
        Keep your summary short, clear and concise so that it provides a high-level overview of the changes and their impact.
        Use direct language and avoid redundant phrases; the fewer words you use, the clearer your summary will be.
        Avoid including any personal opinions or feedback in your summary, as this is a factual summary of the changes.
        Provide the summary without any introductory or concluding statements.
    </Instructions>
    <Steps>
        - Review the diff to understand the changes made in the pull request.
        - Review the commit messages to understand the context of the changes.
        - Review the Jira issues to understand the feature or bug being addressed.
        - Write a detailed, technical description of the changes made in the pull request.
        - Write a summary that explains the changes made in the pull request and how they relate to the feature or bug.
        - Keep your summary clear and concise, focusing on the high-level changes made in the pull request.
        - Provide the summary without any personal opinions or feedback, as this is a factual summary of the changes.
    </Steps>
";
