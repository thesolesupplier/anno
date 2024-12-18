use super::{repository::Repository, AccessToken, IGNORED_REPO_PATHS};
use crate::utils::{config, error::AppError};
use anyhow::Result;
use base64::prelude::*;
use glob::Pattern;
use regex_lite::Regex;
use serde::Deserialize;

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
                if prev_run.path == run.path
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
        let gh_base_url = config::get("GITHUB_BASE_URL");
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
                ("event", &run.event),
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
    pub event: String,
    path: String,
    created_at: String,
    conclusion: Option<String>,
    html_url: String,
    previous_attempt_url: Option<String>,
}

impl WorkflowRun {
    pub async fn get_by_id(full_repo_name: &String, run_id: &String) -> Result<Self> {
        tracing::info!("Fetching workflow run {run_id}");

        let gh_token = AccessToken::get().await?;
        let url = format!("https://api.github.com/repos/{full_repo_name}/actions/runs/{run_id}");

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

    pub async fn is_first_successful_attempt(&self) -> Result<bool, AppError> {
        if !self.is_successful_attempt() {
            return Ok(false);
        }

        let is_first_successful_attempt = !self.has_prev_successful_attempt().await?;

        Ok(is_first_successful_attempt)
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

    pub async fn get_config(&self) -> Result<WorkflowConfig> {
        let config_file = self.repository.get_file(&self.path).await?;
        let config = WorkflowConfig::from_base64_str(&config_file.content)?;

        Ok(config)
    }
}

#[derive(Deserialize)]
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

    pub fn get_target_paths(&self) -> Option<WorkflowTargetPaths> {
        WorkflowTargetPaths::from_workflow_config(self)
    }

    pub fn get_app_name(&self) -> Option<&str> {
        self.env.as_ref()?.app_name.as_deref()
    }

    pub fn has_summary_enabled(&self) -> bool {
        self.env
            .as_ref()
            .and_then(|e| e.summary_enabled.as_ref().map(|e| e == "true"))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub struct WorkflowTargetPaths {
    pub included: Vec<Pattern>,
    pub excluded: Vec<Pattern>,
}

impl WorkflowTargetPaths {
    pub fn from_workflow_config(config: &WorkflowConfig) -> Option<Self> {
        let push_config = config.on.as_ref()?.push.as_ref()?;
        let paths = push_config.paths.as_deref().unwrap_or_default();
        let ignored_paths = push_config.paths_ignore.as_deref().unwrap_or_default();

        if paths.is_empty() && ignored_paths.is_empty() {
            return None;
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

        Some(Self {
            included: create_patterns(included),
            excluded: create_patterns(excluded),
        })
    }

    pub fn is_included(&self, path: &str) -> bool {
        let is_included = self.included.is_empty() || self.included.iter().any(|p| p.matches(path));
        let is_excluded = self.excluded.iter().any(|p| p.matches(path));

        is_included && !is_excluded
    }

    pub fn get_sanitised_included(&self) -> Vec<String> {
        let special_char_regex = Regex::new(r"[*\[\]?!+]").unwrap();

        self.included
            .iter()
            .map(|p| special_char_regex.replace_all(p.as_str(), "").to_string())
            .collect()
    }
}

#[derive(Deserialize)]
struct WorkflowEnvVariables {
    #[serde(rename = "ANNO_APP_NAME")]
    app_name: Option<String>,
    #[serde(rename = "ANNO_SUMMARY_ENABLED")]
    summary_enabled: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowOnConfig {
    push: Option<WorkflowOnPushConfig>,
}

#[derive(Deserialize)]
struct WorkflowOnPushConfig {
    paths: Option<Vec<String>>,
    #[serde(rename = "paths-ignore")]
    paths_ignore: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct WorkflowRunActor {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
pub struct WorkflowRunCommit {
    pub timestamp: String,
}
