use crate::{
    middleware::validation::GithubEvent,
    services::{chat_gpt, github::Workflow, slack, Git},
    utils::error::AppError,
};
use hyper::StatusCode;
use regex_lite::Regex;
use std::{env, sync::OnceLock};

static JIRA_TICKET_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_jira_ticket_links(commit_messages: &[String]) -> Vec<String> {
    let jira_base_url = env::var("JIRA_BASE_URL").expect("JIRA_BASE_URL should be set");

    let regex = JIRA_TICKET_REGEX.get_or_init(|| Regex::new(r"TFW-\d+").unwrap());

    let ticket_links = commit_messages
        .iter()
        .filter_map(|msg| {
            regex.find(msg).map(|t| {
                let ticket = t.as_str();
                format!("{jira_base_url}/{ticket}")
            })
        })
        .collect();

    ticket_links
}

pub async fn post(GithubEvent(workflow): GithubEvent<Workflow>) -> Result<StatusCode, AppError> {
    let send_slack_msg = env::var("SLACK_MESSAGE_ENABLED").is_ok_and(|e| e == "true");

    if !workflow.is_pipeline_run() || !workflow.is_successful_run() {
        return Ok(StatusCode::OK);
    }

    let Some(prev_run) = workflow.get_prev_successful_run().await? else {
        return Ok(StatusCode::OK);
    };

    let new_commit = &workflow.workflow_run.head_sha;
    let old_commit = &prev_run.head_sha;

    let repo = Git::init(&workflow)?;

    let Some(diff) = repo.diff(new_commit, old_commit)? else {
        return Ok(StatusCode::OK);
    };

    let commit_msgs = repo.get_commit_messages_between(old_commit, new_commit)?;
    let jira_ticket_links = get_jira_ticket_links(&commit_msgs);

    let summary = chat_gpt::summarise_release(&diff, &commit_msgs).await?;

    if send_slack_msg {
        slack::post_release_message(&summary, jira_ticket_links, &workflow, &prev_run).await?;
    } else {
        println!("------ SUMMARY ------");
        println!("{summary}");
        println!("------ END SUMMARY ------");
    }

    Ok(StatusCode::OK)
}
