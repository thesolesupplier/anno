use crate::utils::error::AppError;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct WorkflowEvent {
    pub repository: Repository,
    pub workflow_run: WorkflowRun,
}

impl WorkflowEvent {
    pub async fn is_first_successful_release(&self) -> Result<bool, AppError> {
        if !self.workflow_run.is_pipeline_run() || !self.workflow_run.is_successful_run() {
            return Ok(false);
        }

        let is_first_successful_attempt = self
            .workflow_run
            .get_prev_successful_attempt()
            .await?
            .is_none();

        Ok(is_first_successful_attempt)
    }

    pub async fn get_prev_successful_release(&self) -> Result<Option<WorkflowRun>, AppError> {
        let gh_base_url = env::var("GITHUB_BASE_URL").expect("GITHUB_BASE_URL should be set");
        let gh_token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

        let url = format!(
            "{}/repos/{}/actions/runs",
            gh_base_url, self.repository.full_name
        );

        let mut workflow_runs = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[
                ("status", "success"),
                ("branch", "master"),
                ("event", "push"),
                ("created", &format!("<{}", self.workflow_run.created_at)),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<WorkflowRuns>()
            .await?
            .workflow_runs;

        if workflow_runs.is_empty() {
            return Ok(None);
        }

        let previous_run = workflow_runs.remove(0);

        Ok(Some(previous_run))
    }

    pub fn get_diff_url(&self, prev_sha: &str) -> String {
        format!(
            "https://github.com/{}/compare/{}...{}",
            self.repository.full_name, prev_sha, self.workflow_run.head_sha
        )
    }

    pub fn get_run_url(&self) -> &String {
        &self.workflow_run.html_url
    }
}

#[derive(Deserialize)]
pub struct WorkflowRun {
    pub name: String,
    pub head_sha: String,
    pub created_at: String,
    pub conclusion: Option<String>,
    pub html_url: String,
    previous_attempt_url: Option<String>,
}

impl WorkflowRun {
    pub fn is_pipeline_run(&self) -> bool {
        self.name == "Pipeline"
    }

    pub fn is_successful_run(&self) -> bool {
        self.conclusion.as_ref().is_some_and(|c| c == "success")
    }

    pub async fn get_prev_successful_attempt(&self) -> Result<Option<WorkflowRun>, AppError> {
        let mut possible_prev_run = self.get_prev_attempt().await?;

        loop {
            let Some(prev_run) = possible_prev_run else {
                break;
            };

            if prev_run.is_successful_run() {
                return Ok(Some(prev_run));
            }

            possible_prev_run = prev_run.get_prev_attempt().await?;
        }

        Ok(None)
    }

    pub async fn get_prev_attempt(&self) -> Result<Option<WorkflowRun>, AppError> {
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
}

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct WorkflowRuns {
    workflow_runs: Vec<WorkflowRun>,
}
