use super::github::Repository;
use anyhow::Result;
use git2::{DiffFormat, DiffLine};
use std::{env, str};

pub struct Git {
    repo: git2::Repository,
}

impl Git {
    pub fn init(repo: &Repository) -> Result<Self> {
        let repos_dir = env::var("REPOS_DIR").expect("REPOS_DIR should be set");
        let user_name = env::var("GITHUB_USERNAME").expect("GITHUB_USERNAME should be set");
        let token = env::var("GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN should be set");

        let repo_url = format!("https://{user_name}:{token}@github.com/{}", repo.full_name);
        let repo_disk_path = format!("{repos_dir}/{}", repo.name.replace('-', "_"));

        let repo = match git2::Repository::open(&repo_disk_path) {
            Ok(repo) => {
                repo.find_remote("origin")?.fetch(&["master"], None, None)?;
                git2::Repository::open(&repo_disk_path)?
            }
            Err(_) => git2::Repository::clone(&repo_url, &repo_disk_path)?,
        };

        Ok(Self { repo })
    }

    pub fn diff(&self, new_commit_hash: &str, old_commit_hash: &str) -> Result<Option<String>> {
        let new_commit = self.repo.revparse_single(new_commit_hash)?;
        let old_commit = self.repo.revparse_single(old_commit_hash)?;

        let new_tree = new_commit.peel_to_tree()?;
        let old_tree = old_commit.peel_to_tree()?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

        if diff.stats()?.files_changed() == 0 {
            return Ok(None);
        }

        let mut diff_text = String::new();

        diff.print(DiffFormat::Patch, |delta, _hunk, line| {
            let path = delta.old_file().path().unwrap().to_str().unwrap();

            if !path.contains("package-lock.json") {
                let change_symbol = get_change_symbol(&line);
                let content = str::from_utf8(line.content()).unwrap();

                diff_text.push_str(&format!("{change_symbol}{content}"));
            }

            true
        })?;

        if diff_text.chars().count() == 0 {
            return Ok(None);
        }

        Ok(Some(diff_text))
    }

    pub fn get_commit_messages_between(&self, commit1: &str, commit2: &str) -> Result<Vec<String>> {
        let mut revwalk = self.repo.revwalk()?;

        revwalk.set_sorting(git2::Sort::TIME)?;

        let commit_range = format!("{commit1}..{commit2}");
        revwalk.push_range(&commit_range)?;

        let mut messages = Vec::new();
        for oid in revwalk {
            let commit = self.repo.find_commit(oid?)?;
            messages.push(commit.message().unwrap_or_default().to_string());
        }

        Ok(messages)
    }
}

fn get_change_symbol(line: &DiffLine) -> String {
    match line.origin() {
        '+' | '-' | ' ' => format!("{} ", line.origin()),
        _ => String::new(),
    }
}
