use crate::mure_error::Error;
use graphql_client::{GraphQLQuery, Response};

#[allow(clippy::upper_case_acronyms)]
type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema/schema.docs.graphql",
    query_path = "graphql/schema/query.graphql",
    response_derives = "Debug,PartialEq,Eq,Clone"
)]
pub struct SearchRepositoryQuery;

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
    let res = client
        .post(url)
        .header("Authorization", bearer)
        .header("User-Agent", "mure")
        .json(&request_body)
        .send()?;
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
