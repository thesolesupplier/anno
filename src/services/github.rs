use crate::utils::{config, error::AppError};
use anyhow::Result;
use serde::Deserialize;

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
        let gh_token = config::get("GITHUB_ACCESS_TOKEN")?;

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
        let prev_runs = Self::get_prev_runs(run).await?.workflow_runs;

        for prev_run in prev_runs {
            if prev_run.name == run.name
                && prev_run.head_sha != run.head_sha
                && prev_run.has_successful_attempt().await?
            {
                return Ok(Some(prev_run));
            }
        }

        Ok(None)
    }

    async fn get_prev_runs(run: &WorkflowRun) -> Result<Self, AppError> {
        let gh_base_url = config::get("GITHUB_BASE_URL")?;
        let gh_token = config::get("GITHUB_ACCESS_TOKEN")?;

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
    pub async fn get_pull_request(&self, id: &str) -> Result<PullRequest> {
        let gh_token = config::get("GITHUB_ACCESS_TOKEN")?;

        let url = self.pulls_url.replace("{/number}", &format!("/{id}"));

        let pr = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()?
            .json::<PullRequest>()
            .await?;

        Ok(pr)
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
}
