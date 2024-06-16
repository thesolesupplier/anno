use anyhow::{Error, Result};
use std::env;
mod chat_gpt;
mod claude;
mod prompt;

pub async fn summarise_release(diff: &str, commit_messages: &[String]) -> Result<String> {
    let llm_provider = env::var("LLM_PROVIDER").expect("LLM_PROVIDER should be set");

    match llm_provider.as_str() {
        "openai" => get_chat_gpt_summary(diff, commit_messages).await,
        "anthropic" | _ => {
            if llm_provider != "anthropic" {
                tracing::warn!("Unknown LLM provider '{llm_provider}', defaulting to Anthropic.");
            }

            get_claude_summary(diff, commit_messages).await
        }
    }
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
