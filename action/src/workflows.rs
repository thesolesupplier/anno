use anyhow::Result;
use base64::prelude::*;
use glob::Pattern;
use regex_lite::Regex;
use serde::Deserialize;
use shared::{
    services::github::{AccessToken, IGNORED_REPO_PATHS, repository::Repository},
    utils::{config, error::AppError},
};

#[derive(Deserialize)]
pub struct WorkflowRuns {
    workflow_runs: Vec<WorkflowRun>,
}

impl WorkflowRuns {
    pub async fn get_prev_runs_with_last_success_for_branch(
        run: &WorkflowRun,
    ) -> Result<Option<PrevRuns>, AppError> {
        tracing::info!(
            "Fetching previous and last successful workflow runs for {} branch",
            run.head_branch
        );

        let mut all_prev_runs: Vec<WorkflowRun> = Vec::new();
        let mut page = 1;
        loop {
            let prev_runs = Self::get_prev_runs(run, true, page).await?.workflow_runs;

            if prev_runs.is_empty() {
                return Ok(None);
            }

            for prev_run in prev_runs {
                if prev_run.path != run.path {
                    continue;
                }

                if prev_run.has_successful_attempt().await? {
                    return Ok(Some(PrevRuns {
                        last_successful: prev_run,
                        prev_runs: all_prev_runs,
                    }));
                }

                all_prev_runs.push(prev_run);
            }

            page += 1;
        }
    }

    pub async fn get_prev_successful_run(
        run: &WorkflowRun,
    ) -> Result<Option<WorkflowRun>, AppError> {
        tracing::info!("Fetching last successful run for workflow");

        let mut page = 1;
        loop {
            let prev_runs = Self::get_prev_runs(run, false, page).await?.workflow_runs;

            if prev_runs.is_empty() {
                return Ok(None);
            }

            for prev_run in prev_runs {
                if prev_run.path != run.path {
                    continue;
                }

                if prev_run.has_successful_attempt().await? {
                    return Ok(Some(prev_run));
                }
            }

            page += 1;
        }
    }

    async fn get_prev_runs(
        run: &WorkflowRun,
        for_run_branch: bool,
        page: u8,
    ) -> Result<Self, AppError> {
        let gh_base_url = config::get("GITHUB_BASE_URL");
        let gh_token = AccessToken::get().await?;

        let url = format!(
            "{}/repos/{}/actions/runs",
            gh_base_url, run.repository.full_name
        );

        let mut request = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .query(&[
                ("created", &format!("<{}", run.created_at)),
                ("page", &page.to_string()),
            ]);

        if for_run_branch {
            request = request.query(&[("branch", &run.head_branch)]);
        }

        let runs = request
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
pub struct WorkflowRepo {
    url: String,
    full_name: String,
}

pub struct PrevRuns {
    pub last_successful: WorkflowRun,
    pub prev_runs: Vec<WorkflowRun>,
}

#[derive(Deserialize)]
pub struct WorkflowRun {
    pub head_sha: String,
    pub head_branch: String,
    pub repository: WorkflowRepo,
    pub actor: WorkflowRunActor,
    pub path: String,
    created_at: String,
    conclusion: Option<String>,
    html_url: String,
    previous_attempt_url: Option<String>,
}

impl WorkflowRun {
    pub async fn get_by_id(repo_name: &String, run_id: &String) -> Result<Self> {
        tracing::info!("Fetching workflow run {run_id}");

        let gh_token = AccessToken::get().await?;
        let url = format!("https://api.github.com/repos/{repo_name}/actions/runs/{run_id}");

        let workflow_run = reqwest::Client::new()
            .get(url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting workflow run: {e}"))?
            .json::<Self>()
            .await?;

        Ok(workflow_run)
    }

    pub fn is_successful_attempt(&self) -> bool {
        self.conclusion.as_ref().is_some_and(|c| c == "success")
    }

    pub async fn has_successful_attempt(&self) -> Result<bool, AppError> {
        Ok(self.is_successful_attempt() || self.get_prev_successful_attempt().await?.is_some())
    }

    pub async fn has_prev_successful_attempt(&self) -> Result<bool, AppError> {
        Ok(self.get_prev_successful_attempt().await?.is_some())
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
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting previous workflow attempt: {e}"))?
            .json::<WorkflowRun>()
            .await?;

        Ok(Some(workflow_run))
    }

    pub fn get_run_url(&self) -> &String {
        &self.html_url
    }

    pub async fn get_repo(&self) -> Result<Repository> {
        tracing::info!("Fetching workflow repository");

        let gh_token = AccessToken::get().await?;

        let repo = reqwest::Client::new()
            .get(&self.repository.url)
            .bearer_auth(gh_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Anno")
            .send()
            .await?
            .error_for_status()
            .inspect_err(|e| tracing::error!("Error getting workflow run: {e}"))?
            .json::<Repository>()
            .await?;

        Ok(repo)
    }
}

#[derive(Deserialize)]
pub struct WorkflowConfig {
    on: Option<WorkflowOnConfig>,
}

impl WorkflowConfig {
    pub fn from_base64_str(content: &str) -> Result<Self> {
        let decoded_config = BASE64_STANDARD.decode(content.replace('\n', ""))?;
        let config_content = String::from_utf8(decoded_config)?;
        let config = serde_yaml::from_str(&config_content)?;

        Ok(config)
    }

    pub fn push_config(&self) -> Option<&WorkflowOnPushConfig> {
        self.on.as_ref()?.push.as_ref()
    }

    pub fn get_target_paths(&self) -> WorkflowTargetPaths {
        WorkflowTargetPaths::from_workflow_config(self)
    }
}

#[derive(Debug, Default)]
pub struct WorkflowTargetPaths {
    pub included: Vec<Pattern>,
    pub excluded: Vec<Pattern>,
}

impl WorkflowTargetPaths {
    pub fn from_workflow_config(config: &WorkflowConfig) -> Self {
        let Some(push_config) = config.push_config() else {
            return Self::default();
        };

        let paths = push_config.paths.as_deref().unwrap_or_default();
        let ignored_paths = push_config.paths_ignore.as_deref().unwrap_or_default();

        if paths.is_empty() && ignored_paths.is_empty() {
            return Self::default();
        }

        let (included, mut excluded) = paths
            .iter()
            .filter(|p| IGNORED_REPO_PATHS.iter().all(|i| !p.contains(i)))
            .partition::<Vec<_>, _>(|p| !p.starts_with('!'));

        for path in ignored_paths {
            excluded.push(path);
        }

        let create_patterns = |pths: Vec<&String>| {
            pths.iter()
                .map(|p| Pattern::new(p.strip_prefix('!').unwrap_or(p)).unwrap())
                .collect::<Vec<_>>()
        };

        Self {
            included: create_patterns(included),
            excluded: create_patterns(excluded),
        }
    }

    pub fn filter_diff(&self, diff: &str) -> String {
        let re = Regex::new(r"b/([^ ]+)").unwrap();

        let mut is_inside_ignored_file = false;
        diff.lines()
            .filter(|line| {
                if line.starts_with("diff --git") {
                    if let Some(caps) = re.captures(line) {
                        let path = caps[1].to_string();

                        let is_ignored_file = IGNORED_REPO_PATHS.iter().any(|p| path.contains(p));
                        let is_non_target_file = !self.is_included(&path);

                        is_inside_ignored_file = is_ignored_file || is_non_target_file;
                    }
                }

                !is_inside_ignored_file
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_included(&self, path: &str) -> bool {
        let is_included = self.included.is_empty() || self.included.iter().any(|p| p.matches(path));
        let is_excluded = self.excluded.iter().any(|p| p.matches(path));

        is_included && !is_excluded
    }
}

#[derive(Deserialize)]
struct WorkflowOnConfig {
    push: Option<WorkflowOnPushConfig>,
}

#[derive(Deserialize)]
pub struct WorkflowOnPushConfig {
    paths: Option<Vec<String>>,
    #[serde(rename = "paths-ignore")]
    paths_ignore: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct WorkflowRunActor {
    pub login: String,
    pub avatar_url: String,
}
