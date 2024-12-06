use super::{pull_request::PullRequest, workflow::WorkflowTargetPaths, AccessToken};
use anyhow::Result;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub name: String,
    pulls_url: String,
    compare_url: String,
    commits_url: String,
    contents_url: String,
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
                } else {
                    Err(err)
                }
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

    pub async fn fetch_commit_messages_in_range(
        &self,
        (from, to): (&str, &str),
        target_paths: &Option<WorkflowTargetPaths>,
    ) -> Result<Vec<String>> {
        tracing::info!("Fetching commits between {from} - {to}");

        // If no target_paths is provided we get all commit messages and return
        // them because we know it's not a mono-repo and they are all relevant.
        let Some(target_paths) = target_paths else {
            let messages = self
                .list_commits(&[("since", from), ("until", to)])
                .await?
                .into_iter()
                .map(|c| c.commit.message)
                .collect();

            return Ok(messages);
        };

        // If target_paths is provided, first we get all commits that affected
        // files with any of the target_paths in their paths.
        let mut messages = HashSet::new();

        for path in &target_paths.get_sanitised_included() {
            for commit in self
                .list_commits(&[("since", from), ("until", to), ("path", path)])
                .await?
            {
                messages.insert(commit.commit.message);
            }
        }

        // Then we get all commits and filter for PR merges because the GitHub API
        // excludes these from its response when querying by path (for some reason).
        let pr_merge_commits: Vec<Commit> = self
            .list_commits(&[("since", from), ("until", to)])
            .await?
            .into_iter()
            .filter(|c| c.commit.message.starts_with("Merge pull request"))
            .collect();

        let pr_number_regex = Regex::new(r"#(\d+)").unwrap();

        // Finally we check each PR number to see if it affected any files with
        // the target_paths in their paths and include the commit message if it did.
        for Commit { commit } in &pr_merge_commits {
            let Some(pr_number) = pr_number_regex
                .captures(&commit.message)
                .and_then(|c| c.get(1).map(|f| f.as_str()))
            else {
                continue;
            };

            let mut page = 1;
            loop {
                let files = self.get_pull_request_files(pr_number, page).await?;

                if files.is_empty() {
                    break;
                }

                let has_affected_target_files =
                    files.iter().any(|f| target_paths.is_included(&f.filename));

                if has_affected_target_files {
                    messages.insert(commit.message.clone());
                    break;
                }

                page += 1;
            }
        }

        Ok(messages.into_iter().collect())
    }

    async fn list_commits<T: Serialize + ?Sized>(&self, query: &T) -> Result<Vec<Commit>> {
        let gh_token = AccessToken::get().await?;
        let url = self.commits_url.replace("{/sha}", "");

        let mut all_commits: Vec<Commit> = Vec::new();
        let mut page = 1;
        loop {
            tracing::info!("Listing page {page} of commits");

            let commits: Vec<Commit> = reqwest::Client::new()
                .get(&url)
                .bearer_auth(gh_token)
                .header("Accept", "application/json")
                .header("User-Agent", "Anno")
                .query(query)
                .query(&[("page", page), ("per_page", 100)])
                .send()
                .await?
                .error_for_status()
                .inspect_err(|e| tracing::error!("Error listing commits: {e}"))?
                .json()
                .await?;

            if commits.is_empty() {
                break;
            }

            all_commits.extend(commits);

            page += 1;
        }

        Ok(all_commits)
    }

    async fn get_pull_request_files(&self, id: &str, page: u8) -> Result<Vec<PullRequestFile>> {
        tracing::info!("Listing page {page} of pull request #{id} files");

        let gh_token = AccessToken::get().await?;
        let url = self.pulls_url.replace("{/number}", &format!("/{id}/files"));

        let files: Vec<PullRequestFile> = reqwest::Client::new()
            .get(&url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[("page", page), ("per_page", 100)])
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error fetching PR files: {e}"))?
            .json()
            .await?;

        Ok(files)
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
