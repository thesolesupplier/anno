use anyhow::Result;
use git2::{DiffFormat, DiffLine, Repository};
use std::{env, str};

pub struct Git {
    repo: Repository,
}

impl Git {
    pub fn new(repo_path: &str) -> Result<Self> {
        let user_name = env::var("GITHUB_USERNAME").expect("GITHUB_USERNAME should be set");
        let token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

        let repo_url = format!("https://{user_name}:{token}@github.com/{repo_path}");

        let repo_write_path = env::var("REPO_WRITE_PATH").expect("REPO_WRITE_PATH should be set");

        let repo = match Repository::open(&repo_write_path) {
            Ok(repo) => repo,
            Err(_) => Repository::clone(&repo_url, &repo_write_path)?,
        };

        Ok(Self { repo })
    }

    pub fn get_diff_with_head(&self, commit_hash: &str) -> Result<String> {
        let old_commit = self.repo.revparse_single(commit_hash)?;
        let new_commit = self.repo.revparse_single("HEAD")?;

        let old_tree = old_commit.peel_to_tree()?;
        let new_tree = new_commit.peel_to_tree()?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

        let mut diff_text = String::new();

        diff.print(DiffFormat::Patch, |delta, _hunk, line| {
            let path = delta.old_file().path().unwrap().to_str().unwrap();

            if !path.contains("Cargo.lock") {
                let change_symbol = get_change_symbol(&line);
                let content = str::from_utf8(line.content()).unwrap();

                diff_text.push_str(&format!("{change_symbol}{content}"));
            }

            true
        })?;

        Ok(diff_text)
    }
}

fn get_change_symbol(line: &DiffLine) -> String {
    match line.origin() {
        '+' | '-' | ' ' => format!("{} ", line.origin()),
        _ => String::new(),
    }
}
