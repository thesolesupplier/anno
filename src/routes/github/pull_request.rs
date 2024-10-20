use crate::{
    ai::{self, PrAdrAnalysis, PrBugAnalysis},
    middleware::validation::GithubEvent,
    services::{
        github::{PullRequest, Repository},
        Git,
    },
    utils::{config, error::AppError},
};
use hyper::StatusCode;
use serde::Deserialize;

pub async fn adr_analysis(
    GithubEvent(PullRequestEvent {
        pull_request: pr,
        repository: repo,
        action,
    }): GithubEvent<PullRequestEvent>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Processing {} pull request #{}", repo.name, pr.title);

    let adr_repo_full_name = config::get("ADR_REPO_FULL_NAME")?;

    if pr.user.is_bot() {
        return Ok(StatusCode::OK);
    }

    if action != "opened" && action != "synchronize" {
        return Ok(StatusCode::OK);
    }

    let diff = pr.get_diff().await?;
    let commit_messages = pr.get_commit_messages().await?;

    let adrs = Git::init(&adr_repo_full_name).await?.get_contents()?;

    let analysis = ai::Claude::get_pr_adr_analysis(ai::PrAdrAnalysisInput {
        diff: &diff,
        adrs: &adrs,
        commit_messages: &commit_messages,
        pr_body: &pr.body,
    })
    .await?;

    pr.add_comment(&analysis).await?;

    Ok(StatusCode::OK)
}

pub async fn bug_analysis(
    GithubEvent(PullRequestEvent {
        pull_request: pr,
        repository: repo,
        action,
    }): GithubEvent<PullRequestEvent>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Processing {} pull request #{}", repo.name, pr.title);

    if pr.user.is_bot() {
        return Ok(StatusCode::OK);
    }

    if action != "opened" && action != "synchronize" {
        return Ok(StatusCode::OK);
    }

    let diff = pr.get_diff().await?;
    let commit_messages = pr.get_commit_messages().await?;

    let analysis = ai::Claude::get_pr_bug_analysis(&diff, &commit_messages).await?;

    if action == "synchronize" {
        pr.hide_outdated_comments().await?;
    }

    pr.add_comment(&analysis).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub pull_request: PullRequest,
    pub repository: Repository,
}
