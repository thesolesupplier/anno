use chatgpt::prelude::*;
use std::env;

pub async fn get_diff_summary(diff: &str) -> Result<String> {
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY should be set");

    let chat_gpt = ChatGPT::new(openai_api_key)?;

    let message = format!(
        "
         You are a software developer turned product owner.
         Your responsbility is to communicate the changes in this diff to the team.
         The team is non-tech and you need to make it easy for them to understand.
         Rather than describing the diff literally, you need to summarise it.
         Describe the change as a feature in its entirety rather than the individual
         lines or files that were changed.
         Your summary should be concise and no longer than a single sentence.
         Again, your entire summary should only be a single sentence long, no headings or bullet points
         ----
         Diff: {diff}
        "
    );

    let response = chat_gpt
        .send_message(message)
        .await?
        .message()
        .content
        .clone();

    Ok(response)
}
