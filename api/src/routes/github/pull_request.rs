use crate::{ai, middleware::validation::GithubEvent};
use anyhow::Result;
use futures::future::try_join_all;
use hyper::StatusCode;
use regex_lite::Regex;
use serde::Deserialize;
use shared::{
    services::{
        github::{PullRequest, Repository},
        jira::Issue,
    },
    utils::{config, error::AppError},
};
use std::collections::HashSet;

pub async fn review(
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
        tracing::info!("Is ignored '{action}' action, skipping");
        return Ok(StatusCode::OK);
    }

    let diff = pr.get_diff().await?;
    let commit_messages = pr.get_commit_messages().await?;
    let review = ai::PrReview::new(&diff, &commit_messages).await?;

    if action == "opened" {
        let issues = get_jira_issues(&pr).await?;
        let summary = ai::PrSummary::new(&diff, &commit_messages, &issues).await?;
        let pr_body = get_pr_body(summary, &pr, &issues);

        pr.set_body(pr_body).await?;
        pr.add_comment(&review.feedback).await?;

        return Ok(StatusCode::OK);
    }

    let anno_comments = pr.get_anno_comments().await?;
    let is_prev_positive = anno_comments.first().map_or(false, |c| c.is_positive());

    if review.is_positive() && is_prev_positive {
        return Ok(StatusCode::OK);
    }

    pr.clear_prev_comments(&anno_comments).await?;
    pr.add_comment(&review.feedback).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub pull_request: PullRequest,
    pub repository: Repository,
}

pub async fn get_jira_issues(pr: &PullRequest) -> Result<Vec<Issue>> {
    let jira_enabled = config::get_optional("JIRA_API_KEY").is_some();

    if !jira_enabled {
        return Ok(Vec::new());
    }

    let project_key = config::get("JIRA_PROJECT_KEY");
    let key_regex = Regex::new(&format!(r"{project_key}-\d+")).expect("Valid regex");

    let mut keys = HashSet::new();

    if let Some(key) = key_regex.find(&pr.head.r#ref) {
        keys.insert(key.as_str());
    }

    if let Some(body) = &pr.body {
        for key in key_regex.find_iter(body) {
            keys.insert(key.as_str());
        }
    };

    let requests = keys.into_iter().map(Issue::get_by_key).collect::<Vec<_>>();

    let mut issues: Vec<_> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    issues.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(issues)
}

pub fn get_pr_body(summary: ai::PrSummary, pr: &PullRequest, issues: &[Issue]) -> String {
    let mut body = String::new();

    if let Some(existing_body) = &pr.body {
        body = format!("{}<hr>{}\n", existing_body, body);
    }

    if !issues.is_empty() {
        body.push_str("#### Tickets\n");

        for issue in issues {
            body.push_str(&format!(
                "- [{} - {}]({})\n",
                issue.key,
                issue.fields.summary,
                issue.get_browse_url()
            ));
        }
    }

    body.push_str(&summary.into_markdown_body());

    body
}
