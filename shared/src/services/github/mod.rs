pub mod pull_request;
pub mod repository;
pub mod workflow;
pub use pull_request::PullRequest;
pub use repository::Repository;
pub use workflow::{WorkflowRun, WorkflowRuns, WorkflowTargetPaths};

use crate::utils::{config, jwt};
use anyhow::Result;
use serde::Deserialize;
use tokio::sync::OnceCell;

const IGNORED_REPO_PATHS: [&str; 9] = [
    ".github",
    "build",
    "Cargo.lock",
    "coverage",
    "dist",
    "target",
    "node_modules",
    "package-lock.json",
    "yarn.lock",
];

#[derive(Deserialize)]
pub struct AccessToken {
    token: String,
}

pub static GITHUB_ACCESS_TOKEN: OnceCell<String> = OnceCell::const_new();

impl AccessToken {
    pub async fn get() -> Result<&'static String> {
        GITHUB_ACCESS_TOKEN.get_or_try_init(Self::fetch).await
    }

    async fn fetch() -> Result<String> {
        // Use token set by Github in an action context if available
        if let Some(github_token) = config::get_optional("GITHUB_TOKEN") {
            return Ok(github_token);
        }

        let gh_base_url = config::get("GITHUB_BASE_URL");
        let gh_app_install_id = config::get("GITHUB_APP_INSTALLATION_ID");

        let jwt_token = jwt::create_github_token();
        let url = format!("{gh_base_url}/app/installations/{gh_app_install_id}/access_tokens");

        let access_token = reqwest::Client::new()
            .post(url)
            .bearer_auth(jwt_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error fetching GitHub access token: {e}"))?
            .json::<Self>()
            .await?
            .token;

        Ok(access_token)
    }
}
