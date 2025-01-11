use super::{pull_request::PullRequest, AccessToken};
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub name: String,
    pulls_url: String,
    compare_url: String,
    contents_url: String,
    commits_url: String,
}

impl Repository {
    pub fn get_compare_url(&self, old_sha: &str, new_sha: &str) -> String {
        format!(
            "https://github.com/{}/compare/{}...{}",
            self.full_name, old_sha, new_sha
        )
    }

    pub async fn get_pull_requests_for_commit(&self, sha: &str) -> Result<Vec<PullRequest>> {
        tracing::info!("Fetching associated pull requests for commit {sha}");

        let gh_token = AccessToken::get().await?;
        let url = self.commits_url.replace("{/sha}", &format!("/{sha}/pulls"));

        let response = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting associated PRs: {e}"))?
            .json::<Vec<PullRequest>>()
            .await?;

        Ok(response)
    }

    pub async fn get_pull_request(&self, id: &str) -> Result<Option<PullRequest>> {
        tracing::info!("Fetching pull request #{id}");

        let gh_token = AccessToken::get().await?;
        let url = self.pulls_url.replace("{/number}", &format!("/{id}"));

        let response = match reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
        {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Error getting PR: {err}");

                if err.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    return Ok(None);
                }

                Err(err)
            }?,
        };

        let pull_request: PullRequest = response.json().await?;

        Ok(Some(pull_request))
    }

    pub async fn get_file(&self, path: &str) -> Result<RepoFile> {
        tracing::info!("Fetching file {path}");

        let gh_token = AccessToken::get().await?;
        let url = self.contents_url.replace("{+path}", path);

        let response = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting repo file: {e}"))?
            .json::<RepoFile>()
            .await?;

        Ok(response)
    }

    pub async fn fetch_diff(&self, old_sha: &str, new_sha: &str) -> Result<String> {
        tracing::info!("Fetching diff between commits {old_sha} and {new_sha}");

        let gh_token = AccessToken::get().await?;
        let url = self
            .compare_url
            .replace("{base}...{head}", &format!("{old_sha}...{new_sha}"));

        let diff = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/vnd.github.diff")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error fetching repo diff: {e}"))?
            .text()
            .await?;

        Ok(diff)
    }
}

#[derive(Deserialize)]
pub struct RepoFile {
    pub content: String,
}

#[derive(Deserialize)]
pub struct Commit {
    pub commit: CommitDetails,
}

#[derive(Deserialize)]
pub struct CommitDetails {
    pub message: String,
}
