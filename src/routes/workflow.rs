use crate::{
    middleware::validation::GithubEvent,
    services::{chat_gpt, github::WorkflowEvent, slack, Git},
    utils::error::AppError,
};
use hyper::StatusCode;
use regex_lite::Regex;
use std::{env, sync::OnceLock};

pub async fn post(
    GithubEvent(workflow_event): GithubEvent<WorkflowEvent>,
) -> Result<StatusCode, AppError> {
    if !workflow_event.is_successful_release() {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = workflow_event.get_prev_successful_run().await? else {
        return Ok(StatusCode::OK);
    };

    let new_commit = &workflow_event.workflow_run.head_sha;
    let old_commit = &prev_run.head_sha;

    let repo = Git::init(&workflow_event.repository)?;

    let Some(diff) = repo.diff(new_commit, old_commit)? else {
        return Ok(StatusCode::OK);
    };

    let commit_messages = repo.get_commit_messages_between(old_commit, new_commit)?;
    let jira_ticket_links = get_jira_ticket_links(&commit_messages);

    let summary = get_chat_gpt_summary(&diff, &commit_messages).await;

    slack::post_release_message(&summary, jira_ticket_links, &workflow_event, &prev_run).await?;

    Ok(StatusCode::OK)
}

async fn get_chat_gpt_summary(diff: &str, commit_msgs: &[String]) -> String {
    match chat_gpt::summarise_release(diff, commit_msgs).await {
        Ok(summary) => summary,
        Err(err) => format!("*⚠️   An OpenAI error occurred, and I was unable to generate a summary:*\n\n ```{err}```"),
    }
}

static JIRA_TICKET_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_jira_ticket_links(commit_messages: &[String]) -> Vec<String> {
    let jira_base_url = env::var("JIRA_BASE_URL").expect("JIRA_BASE_URL should be set");

    let regex = JIRA_TICKET_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let ticket_links = commit_messages
        .iter()
        .filter_map(|message| {
            regex
                .find(message)
                .map(|ticket| format!("{jira_base_url}/{}", ticket.as_str()))
        })
        .collect();

    ticket_links
}
