use crate::middleware::validation::GithubEvent;
use hyper::StatusCode;
use serde::Deserialize;
use shared::{
    ai,
    services::github::{PullRequest, Repository},
    utils::error::AppError,
};

pub async fn bug_analysis(
    GithubEvent(PullRequestEvent {
        pull_request: pr,
        repository: repo,
        action,
    }): GithubEvent<PullRequestEvent>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Processing {} pull request #{}", repo.name, pr.title);

    if pr.user.is_bot() {
        tracing::info!("Is a bot, skipping");
        return Ok(StatusCode::OK);
    }

    if action != "opened" && action != "synchronize" {
        tracing::info!("Is ignored action '{action}', skipping");
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
