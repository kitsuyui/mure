use crate::mure_error::Error;
use graphql_client::{GraphQLQuery, Response};

#[allow(clippy::upper_case_acronyms)]
type URI = String;

#[allow(clippy::upper_case_acronyms)]
type DateTime = String;

#[allow(clippy::upper_case_acronyms)]
type GitObjectID = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema/schema.docs.graphql",
    query_path = "graphql/schema/query.graphql",
    response_derives = "Debug,PartialEq,Eq,Clone"
)]
pub struct SearchRepositoryQuery;

pub fn search_all_repositories_by_queries(
    token: &str,
    queries: &Vec<String>,
) -> Result<Vec<search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository>, Error> {
    let mut results =
        vec![] as Vec<search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository>;
    for query in queries {
        let mut repos = search_all_repositories(token, query)?;
        results.append(&mut repos);
    }
    Ok(results)
}

pub fn search_all_repositories(
    token: &str,
    query: &str,
) -> Result<Vec<search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository>, Error> {
    let mut results =
        vec![] as Vec<search_repository_query::SearchRepositoryQueryReposEdgesNodeOnRepository>;

    let mut cursor = None as Option<String>;
    let mut count = 0;
    loop {
        let variables = search_repository_query::Variables {
            query: query.to_string(),
            first: 100,
            cursor,
        };
        let response = search_repositories(token, variables);
        match response {
            Ok(response) => {
                let page_info = response.repos.page_info;
                let edges = response.repos.edges;
                if let Some(edge) = edges {
                    for edge_ in edge {
                        let Some(edge) = edge_ else {
                            continue;
                        };
                        let Some(node) = edge.node else {
                            continue;
                        };
                        if let search_repository_query::SearchRepositoryQueryReposEdgesNode::Repository(repo) =
                            node
                        {
                            results.push(repo);
                        }
                    }
                }
                if page_info.has_next_page {
                    cursor = page_info.end_cursor;
                } else {
                    break;
                }
            }
            Err(err) => {
                return Err(err);
            }
        }
        count += 1;
        if count > 100 {
            // Avoid infinite loop to prevent reaching github api limit.
            break;
        }
    }
    Ok(results)
}

fn search_repositories(
    token: &str,
    variables: search_repository_query::Variables,
) -> Result<search_repository_query::ResponseData, Error> {
    let request_body = SearchRepositoryQuery::build_query(variables);
    let client = reqwest::blocking::Client::new();
    let url = "https://api.github.com/graphql";
    let bearer = format!("bearer {token}");

    // Set timeout to 10 seconds.
    // I don't know the best value for timeout. But 10 seconds is the upper limit of REST API.
    // GraphQL API has a rate limit but it is complicated to calculate in the code.
    // https://docs.github.com/en/rest/using-the-rest-api/troubleshooting-the-rest-api?apiVersion=2022-11-28#timeouts
    // https://docs.github.com/en/graphql/overview/rate-limits-and-node-limits-for-the-graphql-api
    let timeout = std::time::Duration::from_secs(10);

    let res = client
        .post(url)
        .header("Authorization", bearer)
        .header("User-Agent", "mure")
        .timeout(timeout)
        .json(&request_body)
        .send()?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text()?;
        return Err(Error::from_str(&format!(
            "Failed to search repositories: status: {status}, text: {text}"
        )));
    }
    let response_body: Response<search_repository_query::ResponseData> = res.json()?;
    match response_body.data {
        Some(data) => Ok(data),
        None => Err(Error::from_str("No data found")),
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::from_str(&e.to_string())
    }
}
