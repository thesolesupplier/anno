use crate::utils::error::AppError;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct WorkflowRuns {
    workflow_runs: Vec<WorkflowRun>,
}

impl WorkflowRuns {
    pub async fn get_previous_successful_run(
        repo_path: &str,
        before_date: &str,
    ) -> Result<WorkflowRun, AppError> {
        let token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");
        let url = format!("https://api.github.com/repos/{repo_path}/actions/runs");

        let mut workflow_runs = reqwest::Client::new()
            .get(url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[
                ("status", "success"),
                ("branch", "master"),
                ("created", &format!("<{before_date}")),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<WorkflowRuns>()
            .await?
            .workflow_runs;

        let previous_run = workflow_runs.remove(0);

        println!("Workflow runs: {:#?}", previous_run);

        Ok(previous_run)
    }
}

#[derive(Deserialize, Debug)]
pub struct WorkflowRun {
    pub name: String,
    pub head_sha: String,
    pub created_at: String,
    pub conclusion: String,
}
