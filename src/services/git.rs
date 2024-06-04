use anyhow::Result;
use git2::{DiffFormat, Repository};
use std::{env, str};

pub struct Git {
    repo: Repository,
}

impl Git {
    pub fn init(repo_path: &str) -> Result<Self> {
        let user_name = env::var("GITHUB_USERNAME").expect("GITHUB_USERNAME should be set");
        let token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

        let repo_url = format!("https://{user_name}:{token}@github.com/{repo_path}");

        let repo_write_path = env::var("REPO_WRITE_PATH").expect("REPO_WRITE_PATH should be set");

        let repo = match Repository::open(&repo_write_path) {
            Ok(repo) => {
                repo.find_remote("origin")?.fetch(&["master"], None, None)?;

                Repository::open(&repo_write_path)?
            }
            Err(_) => Repository::clone(&repo_url, &repo_write_path)?,
        };

        Ok(Self { repo })
    }

    pub fn diff(&self, new_commit_hash: &str, old_commit_hash: &str) -> Result<String> {
        let new_commit = self.repo.revparse_single(new_commit_hash)?;
        let old_commit = self.repo.revparse_single(old_commit_hash)?;

        let new_tree = new_commit.peel_to_tree()?;
        let old_tree = old_commit.peel_to_tree()?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

        // Check if there is any diff
        // println!("Diff: {:?}", diff.stats()?);
        let mut diff_text = String::new();

        diff.print(DiffFormat::Patch, |delta, _hunk, line| {
            let path = delta.old_file().path().unwrap().to_str().unwrap();

            if !path.contains("Cargo.lock") {
                let change_symbol = line.origin();
                let content = str::from_utf8(line.content()).unwrap();

                diff_text.push_str(&format!("{change_symbol}{content}"));
            }

            true
        })?;

        Ok(diff_text)
    }
}
