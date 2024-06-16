use anyhow::Result;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct Issue {
    pub key: String,
    pub fields: IssueFields,
}

impl Issue {
    pub async fn get_by_key(key: &str) -> Result<Self> {
        let jira_base_url = env::var("JIRA_BASE_URL").expect("JIRA_BASE_URL should be set");
        let jira_api_key = env::var("JIRA_API_KEY").expect("JIRA_API_KEY should be set");

        let ticket = reqwest::Client::new()
            .get(format!("{jira_base_url}/rest/api/3/issue/{key}"))
            .header("Accept", "application/json")
            .header("Authorization", format!("Basic {jira_api_key}"))
            .send()
            .await?
            .error_for_status()?
            .json::<Self>()
            .await?;

        Ok(ticket)
    }

    pub fn get_browse_url(&self) -> String {
        let jira_base_url = env::var("JIRA_BASE_URL").expect("JIRA_BASE_URL should be set");
        format!("{jira_base_url}/browse/{}", self.key)
    }
}

#[derive(Deserialize)]
pub struct IssueFields {
    pub summary: String,
}
