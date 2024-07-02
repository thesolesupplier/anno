use crate::utils::config;

use super::github::Repository;
use anyhow::Result;
use git2::{Commit, DiffFormat, DiffLine, Oid};
use std::str;

static IGNORED_DIFF_FILES: [&str; 3] = ["package-lock.json", ".github", ".css"];

pub struct Git {
    repo: git2::Repository,
}

impl Git {
    pub fn init(repo: &Repository) -> Result<Self> {
        let repos_dir = config::get("REPOS_DIR")?;
        let username = config::get("GITHUB_USERNAME")?;
        let token = config::get("GITHUB_ACCESS_TOKEN")?;

        let repo_url = format!("https://{username}:{token}@github.com/{}", repo.full_name);
        let repo_disk_path = format!("{repos_dir}/{}", repo.name.replace('-', "_"));

        let repo = match git2::Repository::open(&repo_disk_path) {
            Ok(repo) => {
                repo.find_remote("origin")?.fetch(&["master"], None, None)?;
                repo
            }
            Err(_) => git2::Repository::clone(&repo_url, &repo_disk_path)?,
        };

        Ok(Self { repo })
    }

    pub fn diff(
        &self,
        new_commit_hash: &str,
        old_commit_hash: &str,
        app_name: Option<&str>,
    ) -> Result<Option<String>> {
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

        diff.print(DiffFormat::Patch, |delta, _, line| {
            let path = delta.old_file().path().unwrap().to_str().unwrap();

            let is_ignored_file = IGNORED_DIFF_FILES.iter().any(|f| path.contains(f));
            let is_in_app_dir = app_name.map_or(true, |n| path.contains(&n.to_lowercase()));

            if !is_ignored_file && is_in_app_dir {
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

    pub fn get_commit_messages(
        &self,
        start_commit: &str,
        end_commit: &str,
        app_name: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let commit_range = &format!("{start_commit}..{end_commit}");
        revwalk.push_range(commit_range)?;

        let mut messages = Vec::new();
        for oid in revwalk {
            if let Some(message) = self.get_commit_message(oid?, app_name)? {
                messages.push(message);
            }
        }

        Ok(messages)
    }

    fn get_commit_message(&self, commit: Oid, app_name: Option<&str>) -> Result<Option<String>> {
        let commit = self.repo.find_commit(commit)?;

        if let Some(app_name) = app_name {
            let affected_files = self.get_affected_files(&commit)?;

            if !affected_files
                .iter()
                .any(|path| path.contains(&app_name.to_lowercase()))
            {
                return Ok(None);
            }
        }

        let message = commit.message().unwrap_or_default().to_string();

        Ok(Some(message))
    }

    fn get_affected_files(&self, commit: &Commit) -> Result<Vec<String>> {
        let tree = commit.tree()?;
        let parent_tree = commit.parent(0)?.tree()?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;

        let mut file_paths = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let file_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().into_owned());

                file_paths.push(file_path.unwrap());

                true
            },
            None,
            None,
            None,
        )?;

        Ok(file_paths)
    }
}

fn get_change_symbol(line: &DiffLine) -> String {
    match line.origin() {
        '+' | '-' | ' ' => format!("{} ", line.origin()),
        _ => String::new(),
    }
}
