use crate::utils::{config, error::AppError, jwt};
use anyhow::Result;
use futures::future::try_join_all;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::OnceCell;

pub static GITHUB_ACCESS_TOKEN: OnceCell<String> = OnceCell::const_new();

#[derive(Deserialize)]
pub struct WorkflowRun {
    pub name: String,
    pub head_sha: String,
    pub repository: Repository,
    pub actor: Actor,
    created_at: String,
    conclusion: Option<String>,
    head_branch: String,
    html_url: String,
    previous_attempt_url: Option<String>,
}

impl WorkflowRun {
    pub fn is_on_master(&self) -> bool {
        self.head_branch == "master"
    }

    pub fn get_mono_app_name(&self) -> Option<&str> {
        self.name.split_whitespace().next()
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
            .error_for_status()?
            .json::<WorkflowRun>()
            .await?;

        Ok(Some(workflow_run))
    }

    pub fn get_run_url(&self) -> &String {
        &self.html_url
    }
}

#[derive(Deserialize)]
pub struct Actor {
    pub login: String,
    pub avatar_url: String,
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
            .error_for_status()?
            .json::<Self>()
            .await?;

        Ok(runs)
    }
}

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub name: String,
    pulls_url: String,
}

impl Repository {
    pub async fn get_pull_request(&self, id: &str) -> Result<Option<PullRequest>> {
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

    pub fn get_compare_url(&self, old_sha: &str, new_sha: &str) -> String {
        format!(
            "https://github.com/{}/compare/{}...{}",
            self.full_name, old_sha, new_sha
        )
    }
}

#[derive(Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub head: Commit,
    pub base: Commit,
    pub body: Option<String>,
    url: String,
    comments_url: String,
}

impl PullRequest {
    pub async fn fetch_diff(&self) -> Result<String> {
        let gh_token = AccessToken::get().await?;

        let diff = reqwest::Client::new()
            .get(&self.url)
            .bearer_auth(gh_token)
            .header("Accept", "application/vnd.github.diff")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(diff)
    }

    pub async fn add_comment(&self, comment: &str) -> Result<()> {
        let gh_token = AccessToken::get().await?;

        reqwest::Client::new()
            .post(&self.comments_url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .json(&json!({ "body": comment }))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn hide_outdated_comments(&self) -> Result<()> {
        let bot_comments: Vec<Comment> = self
            .get_comments()
            .await?
            .into_iter()
            .filter(|c| c.is_by_anno_bot())
            .collect();

        let hide_requests: Vec<_> = bot_comments.iter().map(|c| c.mark_as_outdated()).collect();

        try_join_all(hide_requests).await?;

        Ok(())
    }

    async fn get_comments(&self) -> Result<Vec<Comment>> {
        let gh_token = AccessToken::get().await?;

        let comments = reqwest::Client::new()
            .get(&self.comments_url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[("per_page", "100")])
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<Comment>>()
            .await?;

        Ok(comments)
    }
}

#[derive(Deserialize)]
pub struct Commit {
    pub r#ref: String,
    pub sha: String,
}

#[derive(Deserialize)]
pub struct Comment {
    user: User,
    node_id: String,
}

impl Comment {
    pub fn is_by_anno_bot(&self) -> bool {
        let bot_user_id = config::get("GITHUB_BOT_USER_ID").expect("GITHUB_BOT_USER_ID to be set");

        self.user.id.to_string() == bot_user_id
    }

    pub async fn mark_as_outdated(&self) -> Result<()> {
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
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct User {
    id: i64,
}

#[derive(Deserialize)]
pub struct AccessToken {
    token: String,
}

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
            .error_for_status()?
            .json::<Self>()
            .await?
            .token;

        Ok(access_token)
    }
}
