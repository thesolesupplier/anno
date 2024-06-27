use anyhow::{Error, Result};
use regex_lite::Regex;
use std::{env, sync::OnceLock};
mod chat_gpt;
mod claude;
mod prompt;

static CHANGES_REGEX: OnceLock<Regex> = OnceLock::new();

fn extract_summary(ai_response: String) -> Result<String> {
    let summary_regex =
        CHANGES_REGEX.get_or_init(|| Regex::new(r"(?s)<Changes>(.*?)<\/Changes>").unwrap());

    let Some(matches) = summary_regex.captures(&ai_response) else {
        return Ok(ai_response);
    };

    if matches.len() == 0 {
        return Ok(ai_response);
    }

    Ok(matches.get(1).unwrap().as_str().trim().to_string())
}

pub async fn summarise_release(diff: &str, commit_messages: &[String]) -> Result<String> {
    let llm_provider = env::var("LLM_PROVIDER").expect("LLM_PROVIDER should be set");

    let ai_response = match llm_provider.as_str() {
        "openai" => get_chat_gpt_summary(diff, commit_messages).await,
        "anthropic" => get_claude_summary(diff, commit_messages).await,
        _ => {
            tracing::warn!("Unknown LLM provider '{llm_provider}', defaulting to Anthropic.");
            get_claude_summary(diff, commit_messages).await
        }
    }?;

    extract_summary(ai_response)
}

async fn get_claude_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
    claude::summarise_release(diff, commit_messages)
        .await
        .map_err(|err| get_err("Anthropic", err))
}

async fn get_chat_gpt_summary(diff: &str, commit_messages: &[String]) -> Result<String> {
    chat_gpt::summarise_release(diff, commit_messages)
        .await
        .map_err(|err| get_err("OpenAI", err.into()))
}

fn get_err(provider: &str, err: Error) -> Error {
    anyhow::anyhow!("*⚠️   An error occurred while using the {provider} provider:*\n\n ```{err}```")
}
