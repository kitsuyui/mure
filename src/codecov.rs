use std::collections::HashMap;

use serde::Deserialize;

use crate::mure_error;

#[derive(Deserialize, Debug)]
struct RepoInfo {
    name: String,
    // private: bool,
    // updatestamp: Option<String>,
    // author: Author,
    // language: Option<String>,
    // branch: String,
    // active: bool,
    // activated: bool,
}

// #[derive(Deserialize, Debug)]
// struct Author {
//     service: String,
//     username: String,
//     name: String,
// }

#[derive(Deserialize, Debug)]
struct RepoList {
    // count: i64,
    next: Option<String>,
    // previous: Option<String>,
    results: Vec<RepoInfo>,
    // total_pages: i64,
}

#[derive(Deserialize, Debug)]
struct RepoBranchDetail {
    // name: String,
    // updatestamp: Option<String>,
    head_commit: HeadCommit,
}

#[derive(Deserialize, Debug)]
struct HeadCommit {
    // commitid: String,
    // message: String,
    // timestamp: String,
    // ci_passed: bool,
    // author: Author,
    // branch: String,
    totals: TotalsCommon,
    // state: String,
    // report: Report,
}

#[derive(Deserialize, Debug)]
struct TotalsCommon {
    // files: i64,
    // lines: i64,
    // hits: i64,
    // misses: i64,
    // partials: i64,
    coverage: f64,
    // branches: i64,
    // methods: i64,
    // sessions: i64,
    // complexity: i64,
    // complexity_total: i64,
    // complexity_ratio: f64,
}

fn get_repository_detail(
    username: &str,
    repo_name: &str,
    branch_name: &str,
) -> Result<RepoBranchDetail, mure_error::Error> {
    let token = get_codecov_token()?;
    let url = format!(
        "https://codecov.io/api/v2/github/{username}/repos/{repo_name}/branches/{branch_name}"
    );
    let client = reqwest::blocking::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("bearer {}", token)).map_err(|e| {
            mure_error::Error::from_str(&format!(
                "Failed to create header value from token: {:?}",
                e
            ))
        })?,
    );
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| {
            mure_error::Error::from_str(&format!("Failed to send request to codecov: {:?}", e))
        })?
        .json::<RepoBranchDetail>()?;
    Ok(resp)
}

fn get_codecov_token() -> Result<String, mure_error::Error> {
    std::env::var("CODECOV_TOKEN").map_err(|e| {
        mure_error::Error::from_str(&format!(
            "Failed to get CODECOV_TOKEN from environment variable: {:?}",
            e
        ))
    })
}

fn get_all_repository_list(username: &str) -> Result<Vec<RepoInfo>, mure_error::Error> {
    let mut repo_list = Vec::new();
    let mut page = 1;
    loop {
        let resp = get_repository_list(username, 500, page)?;
        repo_list.extend(resp.results);
        if resp.next.is_none() {
            break;
        }
        page += 1;
    }
    Ok(repo_list)
}

fn get_repository_list(
    username: &str,
    page_size: i64,
    page: i64,
) -> Result<RepoList, mure_error::Error> {
    let token = get_codecov_token()?;
    // TODO: filter private repo ?
    let url = format!(
        "https://codecov.io/api/v2/github/{username}/repos?page_size={page_size}&active=true&page={page}"
    );
    let client = reqwest::blocking::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("bearer {}", token)).map_err(|e| {
            mure_error::Error::from_str(&format!(
                "Failed to create header value from token: {:?}",
                e
            ))
        })?,
    );
    let repo_list = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| mure_error::Error::from_str(&format!("{:?}", e)))?
        .json::<RepoList>()?;

    Ok(repo_list)
}

pub struct RepoBranch {
    pub(crate) name: String,
    pub(crate) branch: String,
}

#[derive(Clone)]
pub struct Coverage {
    pub(crate) name: String,
    pub(crate) coverage: Option<f64>,
}

pub fn get_repository_coverage(
    username: &str,
    repos: &Vec<RepoBranch>,
) -> Result<Vec<Coverage>, mure_error::Error> {
    let mut repo_coverage = Vec::new();
    let all_repositories = get_all_repository_list(username)?;
    let all_repositories = all_repositories
        .into_iter()
        .map(|repo| (repo.name.to_string(), repo))
        .collect::<HashMap<String, RepoInfo>>();

    for repo in repos {
        if !all_repositories.contains_key(&repo.name) {
            continue;
        }

        match get_repository_detail(username, &repo.name, &repo.branch) {
            Ok(repo_detail) => {
                repo_coverage.push(Coverage {
                    name: repo.name.to_string(),
                    coverage: Some(repo_detail.head_commit.totals.coverage),
                });
            }
            Err(e) => {
                repo_coverage.push(Coverage {
                    name: repo.name.to_string(),
                    coverage: None,
                });
                println!("Failed to get coverage: {:?}", e);
            }
        }
    }
    Ok(repo_coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_repository_list() {
        let repo_list = get_all_repository_list("kitsuyui").unwrap();
        assert!(!repo_list.is_empty());
    }

    #[test]
    fn test_get_repository_detail() {
        let x = get_repository_detail("kitsuyui", "mure", "main").unwrap();
        assert!(x.head_commit.totals.coverage > 0.0);
    }

    #[test]
    fn test_get_repository_coverage() {
        let repos = vec![RepoBranch {
            name: "mure".to_string(),
            branch: "main".to_string(),
        }];
        let repo_coverage = get_repository_coverage("kitsuyui", &repos).unwrap();
        assert!(!repo_coverage.is_empty());
    }
}
