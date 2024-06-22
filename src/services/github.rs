use crate::utils::error::AppError;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct WorkflowRun {
    pub head_sha: String,
    pub repository: Repository,
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

    pub async fn is_first_successful_attempt(&self) -> Result<bool, AppError> {
        if !self.is_successful_attempt() {
            return Ok(false);
        }

        let is_first_successful_attempt = self.get_prev_successful_attempt().await?.is_none();

        Ok(is_first_successful_attempt)
    }

    fn is_successful_attempt(&self) -> bool {
        self.conclusion.as_ref().is_some_and(|c| c == "success")
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
        let gh_token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

        let Some(prev_attempt_url) = &self.previous_attempt_url else {
            return Ok(None);
        };

        println!("{prev_attempt_url}");

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
pub struct WorkflowRuns {
    workflow_runs: Vec<WorkflowRun>,
}

impl WorkflowRuns {
    pub async fn get_prev_successful_run(
        run: &WorkflowRun,
    ) -> Result<Option<WorkflowRun>, AppError> {
        let runs = Self::get_prev_runs(run).await?.workflow_runs;

        for run in runs {
            if run.is_successful_attempt() || run.get_prev_successful_attempt().await?.is_some() {
                return Ok(Some(run));
            }
        }

        Ok(None)
    }

    async fn get_prev_runs(run: &WorkflowRun) -> Result<Self, AppError> {
        let gh_base_url = env::var("GITHUB_BASE_URL").expect("GITHUB_BASE_URL should be set");
        let gh_token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

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

        println!("Num of runs: {:?}", runs.workflow_runs.len());

        Ok(runs)
    }
}

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub name: String,
}

impl Repository {
    pub fn get_compare_url(&self, before_sha: &str, after_sha: &str) -> String {
        format!(
            "https://github.com/{}/compare/{}...{}",
            self.full_name, before_sha, after_sha
        )
    }
}
