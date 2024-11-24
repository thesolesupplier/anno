use crate::services::{
    github::{PullRequest, Repository},
    jira::Issue,
};
use anyhow::Result;
use futures::future::try_join_all;
use regex_lite::Regex;
use std::{collections::HashSet, sync::OnceLock};

static JIRA_ISSUE_REGEX: OnceLock<Regex> = OnceLock::new();

pub async fn get_jira_issues(commit_messages: &[String]) -> Result<Vec<Issue>> {
    let issue_regex = JIRA_ISSUE_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| issue_regex.find(m).map(|i| i.as_str()))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(Issue::get_by_key)
        .collect();

    let mut issues: Vec<_> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    issues.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(issues)
}

static PR_REGEX: OnceLock<Regex> = OnceLock::new();

pub async fn get_pull_requests<'a>(
    repo: &'a Repository,
    commit_messages: &'a [String],
) -> Result<Vec<PullRequest>> {
    let pr_regex = PR_REGEX.get_or_init(|| Regex::new(r"#(\d+)").unwrap());

    let requests: Vec<_> = commit_messages
        .iter()
        .filter_map(|m| pr_regex.captures(m).and_then(|c| Some(c.get(1)?.as_str())))
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(|id| repo.get_pull_request(id))
        .collect();

    let mut pull_requests: Vec<_> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    pull_requests.sort_by_key(|pr| pr.number);

    Ok(pull_requests)
}
