use crate::utils::error::AppError;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct Workflow {
    pub action: String,
    pub repository: Repository,
    pub workflow_run: WorkflowRun,
}

impl Workflow {
    pub fn is_successful_run(&self) -> bool {
        self.action == "completed" && self.workflow_run.conclusion == "success"
    }

    pub async fn get_prev_successful_run(&self) -> Result<Option<WorkflowRun>, AppError> {
        let token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

        let url = format!(
            "https://api.github.com/repos/{}/actions/runs",
            self.repository.full_name
        );

        let mut workflow_runs = reqwest::Client::new()
            .get(url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[
                ("status", "success"),
                ("branch", "master"),
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
}

#[derive(Deserialize, Debug)]
pub struct WorkflowRun {
    pub name: String,
    pub head_sha: String,
    pub created_at: String,
    pub conclusion: String,
}

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Deserialize)]
pub struct WorkflowRuns {
    workflow_runs: Vec<WorkflowRun>,
}
