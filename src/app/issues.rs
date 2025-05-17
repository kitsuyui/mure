use std::cmp::Reverse;

use crate::config::Config;
use crate::github;
use crate::github::api::search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository;
use crate::mure_error::Error;

pub fn show_issues_main(config: &Config, queries: &[String]) -> Result<(), Error> {
    let queries = if queries.is_empty() {
        if config.github.is_both_query_and_queries_set() {
            return Err(Error::from_str(
                "Both query and queries are set. Please set only one of them.",
            ));
        }
        config.github.get_queries()
    } else {
        queries.to_vec()
    };
    match show_issues(&queries) {
        Ok(_) => (),
        Err(e) => println!("{e}"),
    }
    Ok(())
}

pub struct RepositorySummary {
    github: GitHubRepoSummary,
}

impl RepositorySummary {
    pub fn new(github: GitHubRepoSummary) -> RepositorySummary {
        RepositorySummary { github }
    }

    fn number_of_pull_requests(&self) -> i64 {
        self.github.number_of_pull_requests
    }

    fn number_of_issues(&self) -> i64 {
        self.github.number_of_issues
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
    #[allow(dead_code)]
    pub name: String,
    pub number_of_issues: i64,
    pub number_of_pull_requests: i64,
    pub default_branch_name: Option<String>,
    pub url: String,
    pub last_release_at: String,
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
            last_release_at: repo
                .latest_release
                .as_ref()
                .map(|release| {
                    if let Some(content) = release.published_at.as_ref() {
                        return content[..10].to_string();
                    }
                    "****-**-**".to_string()
                })
                .unwrap_or("****-**-**".to_string()),
        }
    }
}

pub fn repository_summary(
    repos: &Vec<SearchRepositoryQueryReposEdgesNodeOnRepository>,
) -> Result<Vec<RepositorySummary>, Error> {
    let mut results: Vec<GitHubRepoSummary> = Vec::new();
    for repo in repos {
        results.push(GitHubRepoSummary::new_from_api(repo));
    }

    let mut results: Vec<RepositorySummary> = Vec::new();
    for repo in repos {
        let gh_summary = GitHubRepoSummary::new_from_api(repo);
        let summary = RepositorySummary::new(gh_summary);
        results.push(summary);
    }

    results.sort_by_key(|r| {
        (
            Reverse(r.number_of_pull_requests()),
            Reverse(r.number_of_issues()),
        )
    });

    Ok(results)
}

pub fn show_issues(queries: &Vec<String>) -> Result<(), Error> {
    let Ok(token) = github::token::get_github_token() else {
        return Err(Error::from_str("GH_TOKEN is not set"));
    };
    match github::api::search_all_repositories_by_queries(&token, queries) {
        Err(e) => println!("{e}"),
        Ok(result) => {
            match repository_summary(&result) {
                Ok(results) => {
                    // header
                    println!("Issues\tPRs\tBranch\tLastRelease\tURL");
                    for result in results {
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            result.github.number_of_issues,
                            result.github.number_of_pull_requests,
                            result.default_branch(),
                            result.github.last_release_at,
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
