use crate::{services::github::AccessToken, utils::config};
use anyhow::Result;
use git2::{ObjectType, TreeEntry, TreeWalkMode, TreeWalkResult};
use std::str;

pub struct Git {
    repo: git2::Repository,
}

impl Git {
    pub async fn init(full_name: &str) -> Result<Self> {
        tracing::info!("Initialising {full_name} repository");

        let repos_dir = config::get("REPOS_DIR")?;
        let gh_token = AccessToken::get().await?;

        let repo_name = full_name.split('/').last().unwrap_or(full_name);
        let repo_url = format!("https://x-access-token:{gh_token}@github.com/{}", full_name);
        let repo_disk_path = format!("{repos_dir}/{}", repo_name.replace('-', "_"));

        let repo = match git2::Repository::open(&repo_disk_path) {
            Ok(repo) => {
                tracing::info!("Repository already cloned, pulling latest changes");
                repo.find_remote("origin")?.fetch(&["master"], None, None)?;
                repo
            }
            Err(_) => {
                tracing::info!("Cloning repository");
                git2::Repository::clone(&repo_url, &repo_disk_path)?
            }
        };

        Ok(Self { repo })
    }

    pub fn get_contents(&self) -> Result<Vec<String>> {
        let read_entry_contents = |entry: &TreeEntry| -> Result<String> {
            let object_id = entry.to_object(&self.repo)?.id();
            let blob = self.repo.find_blob(object_id)?;
            let contents = str::from_utf8(blob.content())?.to_string();

            Ok(contents)
        };

        let mut contents: Vec<String> = Vec::new();

        self.repo
            .head()?
            .peel_to_commit()?
            .tree()?
            .walk(TreeWalkMode::PreOrder, |_, entry| {
                if entry.kind() != Some(ObjectType::Blob) {
                    return TreeWalkResult::Ok;
                }

                let Ok(content) = read_entry_contents(entry) else {
                    return TreeWalkResult::Ok;
                };

                contents.push(content);

                TreeWalkResult::Ok
            })?;

        Ok(contents)
    }
}
