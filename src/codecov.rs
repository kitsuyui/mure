use std::{collections::HashMap, path::PathBuf};

use crate::mure_error;

#[derive(Clone)]
pub struct Coverage {
    pub(crate) name: String,
    pub(crate) coverage: Option<f64>,
}

pub struct RepoBranch {
    pub(crate) name: String,
    pub(crate) branch: String,
    pub(crate) commit_id: String,
}

fn get_codecov_token() -> Result<String, mure_error::Error> {
    std::env::var("CODECOV_TOKEN").map_err(|e| {
        mure_error::Error::from_str(&format!(
            "Failed to get CODECOV_TOKEN from environment variable: {:?}",
            e
        ))
    })
}

pub fn get_repository_coverage(
    username: &str,
    repos: &Vec<RepoBranch>,
) -> Result<Vec<Coverage>, mure_error::Error> {
    let token = get_codecov_token()?;
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| mure_error::Error::from_str("Failed to get cache dir"))?
        .join("mure")
        .join("codecov");

    let client = codecov_cache::Client::new(token, PathBuf::from(&cache_dir));
    let codecov_repos = client.get_all_repos(&codecov::owner::Owner::new("github", username))?;
    let mut repo_coverage = Vec::new();
    let codecov_all_repositories = codecov_repos
        .into_iter()
        .map(|repo| (repo.name.to_string(), repo))
        .collect::<HashMap<String, codecov::repos::Repo>>();

    for repo in repos {
        if let Some(repo_info) = codecov_all_repositories.get(&repo.name) {
            if !repo_info.activated {
                continue;
            }
            if !repo_info.active {
                continue;
            }
        }
        let author = codecov::author::Author::new("github", username, &repo.name);
        match client.get_branch_detail_with_commit_id(&author, &repo.branch, &repo.commit_id) {
            Ok(branch_detail) => {
                let coverage = branch_detail.latest_coverage();
                repo_coverage.push(Coverage {
                    name: repo.name.to_string(),
                    coverage: Some(coverage),
                });
            }
            Err(e) => {
                println!("Repo {} Failed to get coverage: {:?}", repo.name, e);
            }
        }
    }
    Ok(repo_coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_repository_coverage() {
        let repos = vec![RepoBranch {
            name: "mure".to_string(),
            branch: "main".to_string(),
            commit_id: "1234567890abcdef".to_string(),
        }];
        let repo_coverage = get_repository_coverage("kitsuyui", &repos).unwrap();
        assert!(!repo_coverage.is_empty());
    }
}
