use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::services::{chat_gpt, jira::IssueComment};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct TestCases {
    pub cases: Vec<String>,
}

impl TestCases {
    pub async fn new(issue_description: &str, user_comments: &[IssueComment]) -> Result<Self> {
        tracing::info!("Fetching AI Jira issue test cases");

        let comments = user_comments.iter().fold(String::new(), |acc, c| {
            acc + "Author" + &c.author.display_name + "\nComment" + &c.body + "\n--"
        });

        let user_prompt = format!(
            "<IssueDescription>{issue_description}</IssueDescription>
             <IssueComments>{comments}</IssueComments>"
        );

        chat_gpt::make_request(PROMPT, user_prompt, response_schema()).await
    }

    pub fn into_jira_comment_body(self) -> Value {
        let title = json!({
            "type": "paragraph",
            "content": [
                {
                    "type": "text",
                    "text": "Test Cases",
                    "marks": [{ "type": "strong" }]
                }
            ]
        });

        let test_cases: Vec<_> = self
            .cases
            .into_iter()
            .map(|case| {
                json!({
                    "type": "taskItem",
                    "attrs": {
                        "state": "TODO",
                        "localId": Uuid::new_v4().to_string(),
                    },
                    "content": [
                        {
                            "type": "text",
                            "text": format!(" {}", case)
                        }
                    ]
                })
            })
            .collect();

        let content = json!([
          {
            "type": "panel",
            "attrs": {
              "panelType": "success"
            },
            "content": [title]
          },
          {
            "type": "taskList",
            "attrs": {
              "localId": Uuid::new_v4().to_string()
            },
            "content": test_cases
          }
        ]);

        json!({
          "version": 1,
          "type": "doc",
          "content": content
        })
    }
}

fn response_schema() -> Value {
    let test_case = json!({
      "type": "array",
      "description": "An array of cases represented as strings.",
      "items": {
        "type": "string"
      }
    });

    let schema = json!({
      "name": "cases_schema",
      "schema": {
        "type": "object",
        "properties": {
          "cases": test_case
        },
        "required": [
          "cases"
        ],
        "additionalProperties": false
      },
      "strict": true
    });

    json!({
        "type": "json_schema",
        "json_schema": schema
    })
}

const PROMPT: &str = "
    <Instructions>
        Your role is to create test cases in very basic markdown based on the description and comments of a Jira issue.
        Consider user comments as additional information to help identify all scenarios and edge cases.
        Test cases should:
            - Be **clear, concise, and written in a single sentence.**
            - Avoid unnecessary words like 'Verify that', 'Ensure', or similar phrases, as these are implied in a test case.
            - **Avoid redundancy by grouping similar scenarios into a single test case where appropriate.**
            - Cover all distinct scenarios, edge cases, and expected behaviors based on the information provided.
            - Use simple, non-technical language that is easy for all team members to understand.
    </Instructions>
    <Steps>
        1. **Analyze the Jira Issue:** Carefully read the Jira issue description and comments.
        2. **Identify Scenarios:** Determine all distinct scenarios, edge cases, and expected behaviors to test.
        3. **Consolidate and Group:** Group overlapping or similar scenarios into a single test case where possible to avoid redundancy.
        5. **Write Test Cases:** Draft test cases that are clear, concise, and easy to understand, ensuring each distinct scenario is covered in a separate line. Avoid introductory phrases like 'Verify that' or 'Ensure this.'
        6. **Format Test Cases:** Format the test cases in very basic markdown for readability.
    </Steps>
    <Example Output>
        - Search bar displays a dropdown of suggestions when typing a query.
        - No suggestions are displayed if the search query contains only invalid characters.
        - Selecting a suggestion from the dropdown populates the search bar with the selected text.
        - Suggestions update dynamically as the user continues typing.
    </Example Output>
";
