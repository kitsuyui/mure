use std::path::PathBuf;

use crate::config::{Config, ConfigSupport};
use crate::github::repo::RepoInfo;
use crate::mure_error::Error;

pub fn list(config: &Config, path: bool, full: bool) -> Result<(), Error> {
    let repos = search_mure_repo(config);
    if repos.is_empty() {
        println!("No repositories found");
        return Ok(());
    }
    for repo in repos {
        match repo {
            Ok(mure_repo) => {
                if full && path {
                    #[allow(clippy::expect_used)]
                    let abpath = mure_repo
                        .absolute_path
                        .to_str()
                        .expect("failed to convert to str");
                    println!("{}", abpath);
                } else if full {
                    println!("{}", mure_repo.repo.name_with_owner());
                } else if path {
                    #[allow(clippy::expect_used)]
                    let relpath = mure_repo
                        .relative_path
                        .to_str()
                        .expect("failed to convert to str");
                    println!("{}", relpath);
                } else {
                    println!("{}", mure_repo.repo.repo);
                }
            }
            Err(e) => {
                println!("{}", e.message());
            }
        }
    }
    Ok(())
}

struct MureRepo {
    relative_path: PathBuf,
    absolute_path: PathBuf,
    repo: RepoInfo,
}

fn search_mure_repo(config: &Config) -> Vec<Result<MureRepo, Error>> {
    let mut repos = vec![];
    match config.base_path().read_dir() {
        Ok(dir) => {
            dir.for_each(|entry| {
                if let Ok(entry) = entry {
                    let metadata = match std::fs::symlink_metadata(entry.path()) {
                        Ok(metadata) => metadata,
                        Err(_) => return,
                    };
                    if !metadata.is_symlink() {
                        return;
                    }
                    match read_symlink_as_mure_repo(&entry.path()) {
                        Ok(mure_repo) => repos.push(Ok(mure_repo)),
                        Err(e) => repos.push(Err(e)),
                    }
                }
            });
        }
        Err(_) => {
            repos.push(Err(Error::from_str("failed to read dir")));
        }
    }
    repos
}

fn read_symlink_as_mure_repo(path: &PathBuf) -> Result<MureRepo, Error> {
    let absolute_path = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return Err(Error::from_str("failed to get absolute path")),
    };
    let Some(owner) = absolute_path.parent() else {
        return Err(Error::from_str("failed to get owner"));
    };
    let Some(domain) = owner.parent() else {
        return Err(Error::from_str("failed to get domain"));
    };
    let repo_name = match absolute_path.file_name() {
        Some(path) => match path.to_str() {
            Some(path) => path.to_string(),
            None => return Err(Error::from_str("failed to get repo name")),
        },
        None => return Err(Error::from_str("failed to get repo name")),
    };
    let repo = match (owner.file_name(), domain.file_name()) {
        (Some(owner), Some(domain)) => RepoInfo {
            owner: owner.to_string_lossy().to_string(),
            domain: domain.to_string_lossy().to_string(),
            repo: repo_name,
        },
        _ => return Err(Error::from_str("failed to get owner or domain")),
    };
    Ok(MureRepo {
        relative_path: path.clone(),
        absolute_path,
        repo,
    })
}

#[cfg(test)]
mod tests {}
