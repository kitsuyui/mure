use crate::github;
use crate::github::api::search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository;
use crate::mure_error::Error;

pub struct RepositorySummary {
    // | "\(.issues.totalCount)\t\(.pullRequests.totalCount)\t\(.defaultBranchRef.name)\t\(.url)"'
    pub name: String,
    pub number_of_issues: i64,
    pub number_of_pull_requests: i64,
    pub default_branch_name: Option<String>,
    pub url: String,
}

pub fn repository_summary(
    repos: Vec<SearchRepositoryQueryReposEdgesNodeOnRepository>,
) -> Result<Vec<RepositorySummary>, Error> {
    let mut results: Vec<RepositorySummary> = Vec::new();
    for repo in repos {
        results.push(RepositorySummary {
            name: repo.name.clone(),
            number_of_issues: repo.issues.total_count,
            number_of_pull_requests: repo.pull_requests.total_count,
            default_branch_name: repo
                .default_branch_ref
                .as_ref()
                .map(|default_branch_ref| default_branch_ref.name.clone()),
            url: repo.url.clone(),
        });
    }
    Ok(results)
}

pub fn show_issues(query: &str) -> Result<(), Error> {
    // TODO: more flexible search query
    let token = std::env::var("GH_TOKEN").expect("GH_TOKEN is not set");
    match github::api::search_all_repositories(&token, query) {
        Err(e) => println!("{}", e),
        Ok(result) => {
            match repository_summary(result) {
                Ok(results) => {
                    // header
                    println!("Issues\tPRs\tBranch\tURL");
                    for result in results {
                        println!(
                            "{}\t{}\t{}\t{}",
                            result.number_of_issues,
                            result.number_of_pull_requests,
                            result.default_branch_name.unwrap_or_else(|| "".to_string()),
                            result.url
                        );
                    }
                }
                Err(e) => println!("{}", e),
            }
        }
    };
    Ok(())
}
