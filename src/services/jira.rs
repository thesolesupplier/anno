use crate::utils::config;
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Issue {
    pub key: String,
    pub fields: IssueFields,
}

impl Issue {
    pub async fn get_by_key(key: &str) -> Result<Option<Self>> {
        let jira_base_url = config::get("JIRA_BASE_URL")?;
        let jira_api_key = config::get("JIRA_API_KEY")?;

        let response = match reqwest::Client::new()
            .get(format!("{jira_base_url}/rest/api/3/issue/{key}"))
            .header("Accept", "application/json")
            .header("Authorization", format!("Basic {jira_api_key}"))
            .send()
            .await?
            .error_for_status()
        {
            Ok(res) => res,
            Err(err) => {
                if err.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    return Ok(None);
                } else {
                    Err(err)
                }
            }?,
        };

        let issue = response.json::<Self>().await?;

        Ok(Some(issue))
    }

    pub fn get_browse_url(&self) -> String {
        let jira_base_url = config::get("JIRA_BASE_URL").unwrap();
        format!("{jira_base_url}/browse/{}", self.key)
    }
}

#[derive(Deserialize)]
pub struct IssueFields {
    pub summary: String,
}
