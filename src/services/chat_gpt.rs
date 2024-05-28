use chatgpt::prelude::*;
use std::env;

pub async fn get_diff_summary(diff: &str) -> Result<String> {
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY should be set");

    let chat_gpt = ChatGPT::new(openai_api_key)?;

    let message = format!(
        "Summarise these diffs in as few sentences as possible.
       Write it in this style: \"Updated x dependency, deleted y page, updated z page etc.\":
       {diff}"
    );

    let response = chat_gpt
        .send_message(message)
        .await?
        .message()
        .content
        .clone();

    Ok(response)
}
