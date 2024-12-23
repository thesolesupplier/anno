use super::{pull_request::PullRequest, workflow::WorkflowTargetPaths, AccessToken};
use anyhow::Result;
use regex_lite::Regex;
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

impl Repository {
    pub fn get_compare_url(&self, old_sha: &str, new_sha: &str) -> String {
        format!(
            "https://github.com/{}/compare/{}...{}",
            self.full_name, old_sha, new_sha
        )
    }

    pub async fn get_pull_requests_for_commit(&self, sha: &str) -> Result<Vec<PullRequest>> {
        tracing::info!("Fetching associated pull requests for {sha}");

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

    pub async fn fetch_diff(
        &self,
        old_sha: &str,
        new_sha: &str,
        target_paths: &Option<WorkflowTargetPaths>,
    ) -> Result<String> {
        tracing::info!("Fetching diff between {old_sha} and {new_sha}");

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

        let filtered_diff = self.filter_diff_by_paths(&diff, target_paths);

        Ok(filtered_diff)
    }

    fn filter_diff_by_paths(
        &self,
        diff: &str,
        target_paths: &Option<WorkflowTargetPaths>,
    ) -> String {
        let re = Regex::new(r"b/([^ ]+)").unwrap();
        let mut is_inside_ignored_file = false;

        diff.lines()
            .filter(|line| {
                if line.starts_with("diff --git") {
                    if let Some(caps) = re.captures(line) {
                        let path = caps[1].to_string();

                        let is_ignored_file = IGNORED_REPO_PATHS.iter().any(|p| path.contains(p));
                        let is_non_target_file = target_paths
                            .as_ref()
                            .map_or(false, |targets| !targets.is_included(&path));

                        is_inside_ignored_file = is_ignored_file || is_non_target_file;
                    }
                }

                !is_inside_ignored_file
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Deserialize)]
pub struct RepoFile {
    pub content: String,
}

#[derive(Deserialize)]
pub struct PullRequestFile {
    pub filename: String,
}

#[derive(Deserialize)]
pub struct Commit {
    pub commit: CommitDetails,
}

#[derive(Deserialize)]
pub struct CommitDetails {
    pub message: String,
}
