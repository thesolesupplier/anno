mod services;

use anyhow::Result;
use dotenv::dotenv;
use services::{chat_gpt, Git};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let repo_path = "constantincerdan/photography-website.git";
    let diff = Git::new(repo_path)?.get_diff_with_head("229d67b")?;
    let summary = chat_gpt::get_diff_summary(&diff).await?;

    println!("Summary: {}", summary);

    Ok(())
}
