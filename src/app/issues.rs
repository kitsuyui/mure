use crate::codecov::{get_repository_coverage, Coverage, RepoBranch};
use crate::config::Config;
use crate::github;
use crate::github::api::search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository;
use crate::mure_error::Error;

pub fn show_issues_main(config: &Config, queries: &Vec<String>) -> Result<(), Error> {
    let queries = if queries.is_empty() {
        if config.github.is_both_query_and_queries_set() {
            return Err(Error::from_str(
                "Both query and queries are set. Please set only one of them.",
            ));
        }
        config.github.get_queries()
    } else {
        queries.clone()
    };
    let username = config.github.username.to_string();
    match show_issues(&username, &queries) {
        Ok(_) => (),
        Err(e) => println!("{e}"),
    }
    Ok(())
}

pub struct RepositorySummary {
    github: GitHubRepoSummary,
    codecov: Option<Coverage>,
}

impl RepositorySummary {
    pub fn new(github: GitHubRepoSummary, codecov: Option<Coverage>) -> RepositorySummary {
        RepositorySummary { github, codecov }
    }

    fn coverage_text(&self) -> String {
        match &self.codecov {
            Some(c) => match c.coverage {
                Some(coverage) => format!("{:.2}%", coverage),
                None => "N/A".to_string(),
            },
            None => "N/A".to_string(),
        }
    }

    fn default_branch(&self) -> String {
        match &self.github.default_branch_name {
            Some(b) => b.to_string(),
            None => "main".to_string(),
        }
    }
}

pub struct GitHubRepoSummary {
    // | "\(.issues.totalCount)\t\(.pullRequests.totalCount)\t\(.defaultBranchRef.name)\t\(.url)"'
    pub name: String,
    pub number_of_issues: i64,
    pub number_of_pull_requests: i64,
    pub default_branch_name: Option<String>,
    pub url: String,
}

impl GitHubRepoSummary {
    pub fn new_from_api(
        repo: &SearchRepositoryQueryReposEdgesNodeOnRepository,
    ) -> GitHubRepoSummary {
        GitHubRepoSummary {
            name: repo.name.clone(),
            number_of_issues: repo.issues.total_count,
            number_of_pull_requests: repo.pull_requests.total_count,
            default_branch_name: repo
                .default_branch_ref
                .as_ref()
                .map(|default_branch_ref| default_branch_ref.name.clone()),
            url: repo.url.clone(),
        }
    }
}

impl RepoBranch {
    pub fn from_api(repo: &SearchRepositoryQueryReposEdgesNodeOnRepository) -> RepoBranch {
        RepoBranch {
            name: repo.name.clone(),
            branch: repo
                .default_branch_ref
                .as_ref()
                .map(|default_branch_ref| default_branch_ref.name.clone())
                .unwrap_or_default(),
        }
    }
}

pub fn repository_summary(
    username: &str,
    repos: &Vec<SearchRepositoryQueryReposEdgesNodeOnRepository>,
) -> Result<Vec<RepositorySummary>, Error> {
    let mut results: Vec<GitHubRepoSummary> = Vec::new();
    for repo in repos {
        results.push(GitHubRepoSummary::new_from_api(repo));
    }

    let branch_repos = &repos.iter().map(RepoBranch::from_api).collect();

    let coverage_summary = get_repository_coverage(username, branch_repos)?;
    let coverage_map = coverage_summary
        .into_iter()
        .map(|c| (c.name.to_string(), c))
        .collect::<std::collections::HashMap<String, Coverage>>();
    let mut results: Vec<RepositorySummary> = Vec::new();
    for repo in repos {
        let gh_summary = GitHubRepoSummary::new_from_api(repo);
        let cov_summary = coverage_map.get(&repo.name).cloned();
        let summary = RepositorySummary::new(gh_summary, cov_summary);
        results.push(summary);
    }
    Ok(results)
}

pub fn show_issues(username: &str, queries: &Vec<String>) -> Result<(), Error> {
    let token = match std::env::var("GH_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return Err(Error::from_str("GH_TOKEN is not set"));
        }
    };

    match github::api::search_all_repositories_by_queries(&token, queries) {
        Err(e) => println!("{e}"),
        Ok(result) => {
            match repository_summary(username, &result) {
                Ok(results) => {
                    // header
                    println!("Issues\tPRs\tBranch\tCoverage\tURL");
                    for result in results {
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            result.github.number_of_issues,
                            result.github.number_of_pull_requests,
                            result.default_branch(),
                            result.coverage_text(),
                            result.github.url,
                        );
                    }
                }
                Err(e) => println!("{e}"),
            }
        }
    };
    Ok(())
}
