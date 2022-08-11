use crate::github;
use crate::github::search_repository_query::{
    ResponseData, SearchRepositoryQueryReposEdgesNode, Variables,
};
use crate::mure_error::Error;

pub struct RepositorySummary {
    // | "\(.issues.totalCount)\t\(.pullRequests.totalCount)\t\(.defaultBranchRef.name)\t\(.url)"'
    pub name: String,
    pub number_of_issues: i64,
    pub number_of_pull_requests: i64,
    pub default_branch_name: String,
    pub url: String,
}

pub fn repository_summary(result: ResponseData) -> Result<Vec<RepositorySummary>, Error> {
    let mut results: Vec<RepositorySummary> = Vec::new();
    if let Some(edge) = result.repos.edges {
        for edge_ in edge {
            let node = edge_.expect("edge is None").node.expect("node is None");
            match node {
                SearchRepositoryQueryReposEdgesNode::Repository(repo) => {
                    results.push(RepositorySummary {
                        name: repo.name.clone(),
                        number_of_issues: repo.issues.total_count,
                        number_of_pull_requests: repo.pull_requests.total_count,
                        default_branch_name: repo.default_branch_ref.unwrap().name.clone(),
                        url: repo.url.clone(),
                    });
                }
                _ => unreachable!("unreachable!"),
            }
        }
    }
    Ok(results)
}

pub fn show_issues() -> Result<(), Error> {
    let query = "user:kitsuyui is:public fork:false archived:false";
    let var = Variables {
        query: query.to_string(),
    };
    let token = std::env::var("GH_TOKEN").expect("GH_TOKEN is not set");
    match github::search_repository(token, var) {
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
                            result.default_branch_name,
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
