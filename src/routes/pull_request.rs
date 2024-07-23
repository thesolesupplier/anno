use crate::{
    ai,
    middleware::validation::GithubEvent,
    services::{
        github::{PullRequest, Repository},
        Git,
    },
    utils::{config, error::AppError},
};
use hyper::StatusCode;
use serde::Deserialize;

pub async fn post(
    GithubEvent(event): GithubEvent<PullRequestEvent>,
) -> Result<StatusCode, AppError> {
    let adr_repo_full_name = config::get("ADR_REPO_FULL_NAME")?;

    tracing::info!(
        "Processing '{}' pull request on '{}'",
        event.pull_request.title,
        event.repository.name,
    );

    if event.action != "opened" {
        return Ok(StatusCode::OK);
    }

    let adr_repo = Git::init(&adr_repo_full_name, None)?;
    let pr_repo = Git::init(
        &event.repository.full_name,
        Some(&event.pull_request.head.r#ref),
    )?;

    let old_commit = &event.pull_request.base.sha;
    let new_commit = &event.pull_request.head.sha;

    let Some(diff) = pr_repo.diff(new_commit, old_commit, None)? else {
        return Ok(StatusCode::OK);
    };

    let adrs = adr_repo.get_contents()?;

    let analysis = ai::analyse_pr(&diff, &adrs).await?;

    tracing::info!("{analysis}");

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub pull_request: PullRequest,
    pub repository: Repository,
}
