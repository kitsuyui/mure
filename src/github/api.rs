use crate::mure_error::Error;
use graphql_client::{GraphQLQuery, Response};

#[allow(clippy::upper_case_acronyms)]
type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema/schema.docs.graphql",
    query_path = "graphql/schema/query.graphql",
    response_derives = "Debug,PartialEq,Eq"
)]
pub struct SearchRepositoryQuery;

pub fn search_repository(
    token: String,
    variables: search_repository_query::Variables,
) -> Result<search_repository_query::ResponseData, Error> {
    let request_body = SearchRepositoryQuery::build_query(variables);
    let client = reqwest::blocking::Client::new();
    let url = "https://api.github.com/graphql";
    let bearer = format!("bearer {}", token);
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
