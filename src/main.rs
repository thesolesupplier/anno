use anyhow::Result;
use chatgpt::prelude::*;
use dotenv::dotenv;
use git2::{Diff, DiffFormat, DiffLine, Repository};
use std::{env, str};

fn get_origin(line: &DiffLine) -> String {
    match line.origin() {
        '+' | '-' | ' ' => format!("{} ", line.origin()),
        _ => String::new(),
    }
}

fn get_diff_string(diff: &Diff) -> String {
    let mut diff_text = String::new();

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        let path = delta.old_file().path().unwrap().to_str().unwrap();

        if !path.contains("Cargo.lock") {
            let orgin = get_origin(&line);
            let content = str::from_utf8(line.content()).unwrap();

            diff_text.push_str(&format!("{}{}", orgin, content));
        }

        true
    })
    .unwrap();

    diff_text
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let github_username = env::var("GITHUB_USERNAME").expect("GITHUB_USERNAME should be set");
    let github_token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY should be set");

    let repo_path = "constantincerdan/photography-website.git";

    let chat_gpt = ChatGPT::new(openai_api_key)?;

    let url = format!("https://{github_username}:{github_token}@github.com/{repo_path}");

    let repo = match Repository::open("./repo") {
        Ok(repo) => repo,
        Err(_) => Repository::clone(&url, "./repo")?,
    };

    let old_commit = repo.revparse_single("229d67b")?;
    let new_commit = repo.revparse_single("HEAD")?;

    let old_tree = old_commit.peel_to_tree()?;
    let new_tree = new_commit.peel_to_tree()?;

    println!("Old commit: {}", old_commit.id());
    println!("New commit: {}", new_commit.id());

    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;
    let diff_string = get_diff_string(&diff);

    let message = format!(
        "Summarise these diffs in as few sentences as possible, but describe every change made.
         Write it in this style: \"Updated x dependency, deleted y page, updated z page etc.\":
         {diff_string}"
    );

    let response = chat_gpt.send_message(message).await?;

    println!("Response: {}", response.message().content);

    Ok(())
}
