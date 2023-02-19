use crate::config::Config;
use crate::github;
use crate::github::api::search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository;
use crate::mure_error::Error;

pub fn show_issues_main(config: &Config, query: Option<String>) -> Result<(), Error> {
    let default_query = format!(
        "user:{} is:public fork:false archived:false",
        &config.github.username
    );
    let query = query.unwrap_or_else(|| default_query.to_string());
    match show_issues(&query) {
        Ok(_) => (),
        Err(e) => println!("{e}"),
    }
    Ok(())
}

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
    let token = match std::env::var("GH_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return Err(Error::from_str("GH_TOKEN is not set"));
        }
    };

    match github::api::search_all_repositories(&token, query) {
        Err(e) => println!("{e}"),
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
                            result.default_branch_name.unwrap_or_default(),
                            result.url
                        );
                    }
                }
                Err(e) => println!("{e}"),
            }
        }
    };
    Ok(())
}
