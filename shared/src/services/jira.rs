use crate::utils::config;
use anyhow::Result;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Debug;

#[derive(Deserialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    pub fields: IssueFields,
}

impl Issue {
    pub async fn get_by_key(key: &str) -> Result<Option<Self>> {
        let jira_base_url = config::get("JIRA_BASE_URL");
        let jira_api_key = config::get("JIRA_API_KEY");

        tracing::info!("Fetching Jira issue {key}");

        let response = match reqwest::Client::new()
            .get(format!("{jira_base_url}/rest/api/2/issue/{key}"))
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
                }

                tracing::error!("Error fetching Jira issue: {err}");
                Err(err)
            }?,
        };

        let issue = response.json::<Self>().await?;

        Ok(Some(issue))
    }

    pub fn get_browse_url(&self) -> String {
        let jira_base_url = config::get("JIRA_BASE_URL");
        format!("{jira_base_url}/browse/{}", self.key)
    }

    pub async fn add_comment<T: Serialize + Debug>(&self, comment: T) -> Result<()> {
        let jira_enabled = config::get_optional("JIRA_API_KEY").is_some();

        if !jira_enabled {
            println!("{comment:#?}");
            return Ok(());
        }

        let jira_base_url = config::get("JIRA_BASE_URL");
        let jira_api_key = config::get("JIRA_API_KEY");
        let url = format!("{jira_base_url}/rest/api/3/issue/{}/comment", self.id);

        reqwest::Client::new()
            .post(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Basic {jira_api_key}"))
            .json(&json!({ "body": comment }))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error adding Jira comment: {e}"))?;

        Ok(())
    }

    pub async fn get_user_comments(&self) -> Result<Vec<IssueComment>> {
        let comments = self.get_comments().await?;

        let user_comments: Vec<_> = comments
            .into_iter()
            .filter(|c| !c.is_by_anno_bot())
            .collect();

        Ok(user_comments)
    }

    pub async fn delete_anno_comments(&self) -> Result<()> {
        let jira_enabled = config::get_optional("JIRA_API_KEY").is_some();

        if !jira_enabled {
            return Ok(());
        }

        let bot_comments: Vec<_> = self
            .get_comments()
            .await?
            .into_iter()
            .filter(|c| c.is_by_anno_bot())
            .collect();

        let hide_requests: Vec<_> = bot_comments.iter().map(|c| c.delete()).collect();

        try_join_all(hide_requests)
            .await
            .inspect_err(|e| tracing::error!("Error deleting Jira comments: {e}"))?;

        Ok(())
    }

    async fn get_comments(&self) -> Result<Vec<IssueComment>> {
        let jira_base_url = config::get("JIRA_BASE_URL");
        let jira_api_key = config::get("JIRA_API_KEY");

        let comments = reqwest::Client::new()
            .get(format!(
                "{jira_base_url}/rest/api/2/issue/{}/comment",
                self.id
            ))
            .header("Accept", "application/json")
            .header("Authorization", format!("Basic {jira_api_key}"))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting Jira comments: {e}"))?
            .json::<CommentsResponse>()
            .await?
            .comments;

        Ok(comments)
    }
}

#[derive(Deserialize)]
struct CommentsResponse {
    comments: Vec<IssueComment>,
}

#[derive(Deserialize)]
pub struct IssueComment {
    pub author: CommentAuthor,
    pub body: String,
    #[serde(rename = "self")]
    api_url: String,
}

impl IssueComment {
    pub async fn delete(&self) -> Result<()> {
        let jira_api_key = config::get("JIRA_API_KEY");

        reqwest::Client::new()
            .delete(&self.api_url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Basic {jira_api_key}"))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error deleting Jira comment: {e}"))?;

        Ok(())
    }

    pub fn is_by_anno_bot(&self) -> bool {
        let jira_bot_user_id = config::get("JIRA_BOT_USER_ID");

        self.author.account_id == jira_bot_user_id
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentAuthor {
    pub account_id: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct IssueFields {
    pub summary: String,
    pub description: Option<String>,
}
