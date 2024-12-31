use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::services::chat_gpt;

#[derive(Deserialize, Serialize, Debug)]
pub struct ReleaseSummary {
    pub items: Vec<SummaryCategory>,
}

impl ReleaseSummary {
    pub async fn new(diff: &str, commit_messages: &[String]) -> Result<Self> {
        tracing::info!("Generating release summary");

        let commit_messages = commit_messages.join("\n");

        let user_prompt = format!(
            "<Diff>{diff}</Diff>
             <CommitMessages>{commit_messages}</CommitMessages>"
        );

        chat_gpt::Request {
            user_prompt,
            system_prompt: SYSTEM_PROMPT,
            response_schema: response_schema(),
            ..Default::default()
        }
        .send()
        .await
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SummaryCategory {
    pub title: String,
    pub items: Vec<String>,
}

fn response_schema() -> Value {
    let category_title = json!({
        "type": "string",
        "description": "The title of the JSON object."
    });

    let category_items = json!({
        "type": "array",
        "description": "An array of strings.",
        "items": {
            "type": "string"
        }
    });

    let category = json!({
        "type": "object",
        "properties": {
            "title": category_title,
            "items": category_items
        },
        "required": [
            "title",
            "items"
        ],
        "additionalProperties": false
    });

    let categories = json!({
        "type": "array",
        "description": "An array of JSON objects where each object has a title and an items array.",
        "items": category
    });

    let schema = json!({
        "name": "json_objects_array",
        "schema": {
            "type": "object",
            "properties": {
                "items": categories
            },
            "required": [
                "items"
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

const SYSTEM_PROMPT: &str = "
    <Instructions>
        Your role is to analyze a git code diff and related commit messages to identify and summarize the features that have been released.
        Avoid describing each individual code change. Instead, focus on understanding the broader context of the changes and what features they translate into.
        Keep your description of each feature concise and non-technical, so that a non-technical team member can understand the change in simple terms.
        Avoid listing every commit message or code change. Instead, group the changes into categories like New features, Improvements, Bug fixes, Dependency changes and Feature flags.
        Avoid describing how a feature will impact a user or experience, just describe what the feature is and what it does.
        Avoid expanding acronyms, for example PLP, PDP or USP, to their full meanings because the users understand those.
        Specify any feature flags connected to these features, if any, along with the environments they are enabled in.
        List any dependency additions, updates or removals that were made in the package configuration files only.
    </Instructions>
    <Steps>
        Analyze the Diff: Examine the git code diff to understand the changes in the codebase.
        Analyze Commit Messages: Review the commit messages to gain context and further insights into the changes.
        Identify User-Facing Features: Determine which changes correspond to new features, enhancements, or bug fixes that would be noticeable to the end-users.
        Summarize in Non-Technical Terms: Write a summary of these features in a way that a non-technical team can understand, but no longer than a sentence.
        List Dependency Changes: Identify any dependency changes made in the package configuration files (e.g., new libraries, updated versions) and list them.
        Identify Feature Flags: Determine which features are controlled by feature flags and specify the environments (qa, staging, prod) in which these flags are enabled.
        Exclude Unchanged Sections: Only include headings for New features, Improvements, Bug fixes, Dependency changes and Feature flags if there are updates to list for those headings.
    </Steps>
    <ExampleOutPut1>
        <Output>
            New features:
            • Search results can now be filtered by date and relevance.
            • New avatar customisation options have been added to user profiles.
            Improvements:
            • Refactored the marketing service to improve readability.
            • Added more breakpoints to the Image component.
            Bug fixes:
            • Fixed an issue where the data service was not guarding against unexpected parsing errors.
            • Implemented a workaround to address the caching bug in the user authentication flow.
            Dependency changes:
            • Updated Library `XYZ` to version `1.3.0`.
            • Added library `ABC` version `2.1.0`.
            Feature flags
            • `AVATAR_CUSTOMISATION` is enabled in `qa` and `staging` environments.
        </Output>
    </ExampleOutPut1>
    <ExampleOutPut2>
            New features
            • Added support for tracking URLs in Discord messages for new product discoveries.
            Feature flags
            • `DISCORD_TRACKED_URLS` is enabled in the `prod` environment.
    </ExampleOutPut2>
    <ExampleOutPut3>
            Bug fixes
            • Fixed an issue where the Twitter hyperlink was not displaying properly.
    </ExampleOutPut3>
    Please perform this analysis on the provided git code diff and commits and deliver a summary as described above based on that diff.
    The output should be placed in <Output> tags as demonstrated in the examples above.
    Avoid including headings for New features, Improvements, Bug fixes, Dependency changes or Feature flags if there are no updates to list for those headings.
";
