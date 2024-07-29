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
    GithubEvent(PullRequestEvent {
        pull_request: pr,
        repository: repo,
        action,
    }): GithubEvent<PullRequestEvent>,
) -> Result<StatusCode, AppError> {
    let adr_repo_full_name = config::get("ADR_REPO_FULL_NAME")?;

    tracing::info!("Processing '{}' pull request on '{}'", pr.title, repo.name);

    if action != "opened" {
        return Ok(StatusCode::OK);
    }

    let pr_repo = Git::init(&repo.full_name, Some(&pr.head.r#ref)).await?;
    let old_commit = &pr.base.sha;
    let new_commit = &pr.head.sha;

    let commit_messages = pr_repo.get_commit_messages(old_commit, new_commit, None)?;
    let Some(diff) = pr_repo.diff(new_commit, old_commit, None)? else {
        return Ok(StatusCode::OK);
    };

    let adr_repo = Git::init(&adr_repo_full_name, None).await?;
    let adrs = adr_repo.get_contents()?;

    let analysis = ai::analyse_pr(ai::PrAnalysisInput {
        diff: &diff,
        adrs: &adrs,
        commit_messages: &commit_messages,
        pr_body: &pr.body,
    })
    .await?;

    pr.add_comment(&analysis).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub pull_request: PullRequest,
    pub repository: Repository,
}
