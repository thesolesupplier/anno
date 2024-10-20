use crate::utils::{config, error::AppError, jwt};
use anyhow::Result;
use base64::prelude::*;
use futures::future::try_join_all;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use tokio::sync::OnceCell;

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
    "Cargo.toml",
    "coverage",
    "dist",
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

    pub async fn get_file(&self, path: &str, sha: &str) -> Result<RepoFile> {
        tracing::info!("Fetching file {path} for {sha}");

        let gh_token = AccessToken::get().await?;
        let url = self.contents_url.replace("{+path}", path);

        let response = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[("ref", sha)])
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
        target_paths: &Option<Vec<String>>,
    ) -> Result<String> {
        tracing::info!("Fetching diff beween {old_sha} - {new_sha}");

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

        let mut is_inside_ignored_file = false;
        let filtered_diff = diff
            .lines()
            .filter(|line| {
                if line.contains("diff --git") {
                    let is_ignored_file = IGNORED_REPO_PATHS.iter().any(|p| line.contains(p));
                    let is_non_target_file = target_paths.as_ref().map_or(false, |paths| {
                        paths.iter().all(|p| !line.contains(p.as_str()))
                    });

                    is_inside_ignored_file = is_ignored_file || is_non_target_file;
                }

                !is_inside_ignored_file
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(filtered_diff)
    }

    pub async fn fetch_commit_messages_in_range(
        &self,
        (from, to): (&str, &str),
        target_paths: &Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        tracing::info!("Fetching commits between {from} - {to}");

        // We get all commit messages and return early if no app_name is provided
        // because we know its not a mono-repo and they are all relevant.
        let Some(target_paths) = target_paths else {
            let messages = self
                .list_commits(&[("since", from), ("until", to)])
                .await?
                .into_iter()
                .map(|c| c.commit.message)
                .collect();

            return Ok(messages);
        };

        // If an app_name is provided, first we get all commits that affected
        // files with the app_name in their `/apps` or `/packages` paths.
        let mut messages = HashSet::new();

        for path in target_paths {
            let query = [("since", from), ("until", to), ("path", path)];

            for commit in self.list_commits(&query).await? {
                messages.insert(commit.commit.message);
            }
        }

        // Then we get all commits and filter for PR merges because for some reason
        // the GitHub API excludes these from its response when queried by path.
        let pr_merge_commits: Vec<Commit> = self
            .list_commits(&[("since", from), ("until", to)])
            .await?
            .into_iter()
            .filter(|c| c.commit.message.starts_with("Merge pull request"))
            .collect();

        let pr_regex = Regex::new(r"#(\d+)").unwrap();

        // Finally we check each PR number to see if it affected any files with
        // the app_name in their paths and include the commit message if it did.
        for Commit { commit } in &pr_merge_commits {
            let Some(pr_number) = pr_regex
                .captures(&commit.message)
                .and_then(|c| Some(c.get(1)?.as_str()))
            else {
                continue;
            };

            let mut page = 1;
            loop {
                let files = self.get_pull_request_files(pr_number, page).await?;

                if files.is_empty() {
                    break;
                }

                if files
                    .iter()
                    .any(|f| target_paths.iter().any(|p| f.filename.contains(p)))
                {
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

#[derive(Deserialize, Debug)]
pub struct WorkflowConfig {
    on: Option<WorkflowOnConfig>,
    env: Option<WorkflowEnvVariables>,
}

impl WorkflowConfig {
    pub fn from_base64_str(content: &str) -> Result<Self> {
        let decoded_config = BASE64_STANDARD.decode(content.replace("\n", ""))?;
        let config_content = String::from_utf8(decoded_config)?;
        let config = serde_yaml::from_str(&config_content)?;

        Ok(config)
    }

    pub fn get_target_paths(&self) -> Option<Vec<String>> {
        let paths = self.on.as_ref()?.push.as_ref()?.paths.as_ref()?;

        if paths.is_empty() {
            return None;
        }

        let special_char_regex = Regex::new(r"[*\[\]?!+]").unwrap();

        let sanitised_paths = paths
            .iter()
            .filter(|p| IGNORED_REPO_PATHS.iter().all(|i| !p.contains(i)))
            .map(|p| special_char_regex.replace_all(p, "").to_string())
            .collect();

        Some(sanitised_paths)
    }

    pub fn get_app_name(&self) -> Option<&str> {
        self.env.as_ref()?.anno_app_name.as_deref()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct WorkflowEnvVariables {
    anno_app_name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct WorkflowOnConfig {
    push: Option<WorkflowOnPushConfig>,
}

#[derive(Deserialize, Debug)]
struct WorkflowOnPushConfig {
    paths: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub body: Option<String>,
    pub user: User,
    url: String,
    comments_url: String,
    commits_url: String,
}

impl PullRequest {
    pub async fn get_diff(&self) -> Result<String> {
        tracing::info!("Fetching pull request #{} diff", &self.number);

        let gh_token = AccessToken::get().await?;

        let diff = reqwest::Client::new()
            .get(&self.url)
            .bearer_auth(gh_token)
            .header("Accept", "application/vnd.github.diff")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error fetching PR diff: {e}"))?
            .text()
            .await?;

        let mut is_inside_ignored_file = false;

        let filtered_diff = diff
            .lines()
            .filter(|line| {
                if line.contains("diff --git") {
                    is_inside_ignored_file = IGNORED_REPO_PATHS.iter().any(|p| line.contains(p));
                }

                !is_inside_ignored_file
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(filtered_diff)
    }

    pub async fn get_commit_messages(&self) -> Result<Vec<String>> {
        tracing::info!("Fetching pull request #{} commit messages", &self.number);

        let gh_token = AccessToken::get().await?;

        let mut all_commits: Vec<Commit> = Vec::new();
        let mut page = 1;
        loop {
            let commits: Vec<Commit> = reqwest::Client::new()
                .get(&self.commits_url)
                .bearer_auth(gh_token)
                .header("Accept", "application/json")
                .header("User-Agent", "Anno")
                .query(&[("page", page), ("per_page", 100)])
                .send()
                .await?
                .error_for_status()
                .inspect_err(|e| tracing::error!("Error fetching PR commits: {e}"))?
                .json()
                .await?;

            if commits.is_empty() {
                break;
            }

            all_commits.extend(commits);

            page += 1;
        }

        let all_messages = all_commits.into_iter().map(|c| c.commit.message).collect();

        Ok(all_messages)
    }

    pub async fn add_comment(&self, comment: &str) -> Result<()> {
        tracing::info!("Adding pull request #{} comment", &self.number);

        let pr_comment_enabled = config::get("PR_COMMENT_ENABLED").is_ok_and(|v| v == "true");

        if !pr_comment_enabled {
            println!("------ PR COMMENT ------");
            println!("{comment}");
            println!("------------------------");
            return Ok(());
        }

        let gh_token = AccessToken::get().await?;

        reqwest::Client::new()
            .post(&self.comments_url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .json(&json!({ "body": comment }))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error adding GitHub comment: {e}"))?;

        Ok(())
    }

    pub async fn hide_outdated_comments(&self) -> Result<()> {
        tracing::info!("Hiding outdated pull request {} comments", &self.number);

        let pr_comment_enabled = config::get("PR_COMMENT_ENABLED").is_ok_and(|v| v == "true");

        if !pr_comment_enabled {
            return Ok(());
        }

        let bot_comments: Vec<PullRequestComment> = self
            .get_comments()
            .await?
            .into_iter()
            .filter(|c| c.is_by_anno_bot())
            .collect();

        let hide_requests: Vec<_> = bot_comments.iter().map(|c| c.mark_as_outdated()).collect();

        try_join_all(hide_requests).await?;

        Ok(())
    }

    async fn get_comments(&self) -> Result<Vec<PullRequestComment>> {
        tracing::info!("Getting pull request #{} comments", &self.number);

        let gh_token = AccessToken::get().await?;

        let comments = reqwest::Client::new()
            .get(&self.comments_url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[("per_page", "100")])
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting GitHub comments: {e}"))?
            .json::<Vec<PullRequestComment>>()
            .await?;

        Ok(comments)
    }
}

#[derive(Deserialize)]
pub struct PullRequestComment {
    user: User,
    node_id: String,
}

impl PullRequestComment {
    pub fn is_by_anno_bot(&self) -> bool {
        let bot_user_id = config::get("GITHUB_BOT_USER_ID").unwrap();

        self.user.id.to_string() == bot_user_id
    }

    pub async fn mark_as_outdated(&self) -> Result<()> {
        tracing::info!("Marking comment {} as outdated", &self.node_id);

        let gh_token = AccessToken::get().await?;
        let mutation = format!(
            r#"
            mutation {{
                minimizeComment(input: {{
                    subjectId: "{comment_id}",
                    classifier: OUTDATED
                }}) {{
                    minimizedComment {{
                        isMinimized
                    }}
                }}
            }}"#,
            comment_id = &self.node_id
        );

        reqwest::Client::new()
            .post("https://api.github.com/graphql")
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .json(&json!({ "query": mutation }))
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error hiding GitHub comment: {e}"))?;

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct WorkflowRuns {
    workflow_runs: Vec<WorkflowRun>,
}

impl WorkflowRuns {
    pub async fn get_prev_successful_run(
        run: &WorkflowRun,
    ) -> Result<Option<WorkflowRun>, AppError> {
        tracing::info!("Fetching previous successful run");

        let mut page = 1;
        loop {
            let prev_runs = Self::get_prev_runs(run, page).await?.workflow_runs;

            if prev_runs.is_empty() {
                return Ok(None);
            }

            for prev_run in prev_runs {
                if prev_run.name == run.name
                    && prev_run.head_sha != run.head_sha
                    && prev_run.has_successful_attempt().await?
                {
                    return Ok(Some(prev_run));
                }
            }

            page += 1;
        }
    }

    async fn get_prev_runs(run: &WorkflowRun, page: u8) -> Result<Self, AppError> {
        let gh_base_url = config::get("GITHUB_BASE_URL")?;
        let gh_token = AccessToken::get().await?;

        let url = format!(
            "{}/repos/{}/actions/runs",
            gh_base_url, run.repository.full_name
        );

        let runs = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[
                ("branch", "master"),
                ("event", "push"),
                ("created", &format!("<{}", run.created_at)),
                ("page", &page.to_string()),
            ])
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting previous workflow runs: {e}"))?
            .json::<Self>()
            .await?;

        Ok(runs)
    }
}

#[derive(Deserialize)]
pub struct WorkflowRun {
    pub name: String,
    pub head_sha: String,
    pub repository: Repository,
    pub actor: WorkflowRunActor,
    pub head_commit: WorkflowRunCommit,
    path: String,
    created_at: String,
    conclusion: Option<String>,
    head_branch: String,
    html_url: String,
    previous_attempt_url: Option<String>,
}

impl WorkflowRun {
    pub fn is_on_master(&self) -> bool {
        self.head_branch == "master" || self.head_branch == "main"
    }

    pub fn is_successful_attempt(&self) -> bool {
        self.conclusion.as_ref().is_some_and(|c| c == "success")
    }

    pub async fn has_successful_attempt(&self) -> Result<bool, AppError> {
        Ok(self.is_successful_attempt() || self.get_prev_successful_attempt().await?.is_some())
    }

    pub async fn is_first_successful_attempt(&self) -> Result<bool, AppError> {
        if !self.is_successful_attempt() {
            return Ok(false);
        }

        let is_first_successful_attempt = self.get_prev_successful_attempt().await?.is_none();

        Ok(is_first_successful_attempt)
    }

    async fn get_prev_successful_attempt(&self) -> Result<Option<WorkflowRun>, AppError> {
        let mut possible_prev_attempt = self.get_prev_attempt().await?;

        loop {
            let Some(prev_attempt) = possible_prev_attempt else {
                break;
            };

            if prev_attempt.is_successful_attempt() {
                return Ok(Some(prev_attempt));
            }

            possible_prev_attempt = prev_attempt.get_prev_attempt().await?;
        }

        Ok(None)
    }

    async fn get_prev_attempt(&self) -> Result<Option<WorkflowRun>, AppError> {
        tracing::info!("Fetching previous '{}' workflow attempt", &self.name);

        let gh_token = AccessToken::get().await?;

        let Some(prev_attempt_url) = &self.previous_attempt_url else {
            return Ok(None);
        };

        let workflow_run = reqwest::Client::new()
            .get(prev_attempt_url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting previous workflow attempt: {e}"))?
            .json::<WorkflowRun>()
            .await?;

        Ok(Some(workflow_run))
    }

    pub fn get_run_url(&self) -> &String {
        &self.html_url
    }

    pub async fn get_config(&self) -> Result<WorkflowConfig> {
        let config_file = self.repository.get_file(&self.path, &self.head_sha).await?;
        let config = WorkflowConfig::from_base64_str(&config_file.content)?;

        Ok(config)
    }
}

#[derive(Deserialize)]
pub struct User {
    id: i64,
    r#type: UserType,
}

impl User {
    pub fn is_bot(&self) -> bool {
        matches!(self.r#type, UserType::Bot)
    }
}

#[derive(Deserialize)]
enum UserType {
    User,
    Bot,
}

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
        let gh_base_url = config::get("GITHUB_BASE_URL")?;
        let gh_app_install_id = config::get("GITHUB_APP_INSTALLATION_ID")?;

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

#[derive(Deserialize)]
pub struct WorkflowRunActor {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
struct PullRequestFile {
    filename: String,
}

#[derive(Deserialize)]
pub struct Commit {
    pub commit: CommitDetails,
}

#[derive(Deserialize)]
pub struct CommitDetails {
    pub message: String,
}

#[derive(Deserialize)]
pub struct WorkflowRunCommit {
    pub timestamp: String,
}
