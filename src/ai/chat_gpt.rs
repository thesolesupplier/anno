use super::prompt::RELEASE_SUMMARY_PROMPT;
use chatgpt::prelude::*;
use std::env;

pub async fn summarise_release(diff: &str, commit_messages: &[String]) -> Result<String> {
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY should be set");

    let chat_gpt = ChatGPT::new_with_config(
        openai_api_key,
        ModelConfigurationBuilder::default()
            .temperature(0.0)
            .frequency_penalty(2.0)
            .build()
            .unwrap(),
    )?;

    let commit_messages = commit_messages.join("\n");

    let response = chat_gpt
        .send_message(format!(
            "Prompt: {RELEASE_SUMMARY_PROMPT} | Diff: {diff} | Commit Messages: {commit_messages}"
        ))
        .await?
        .message()
        .content
        .clone();

    Ok(response)
}
