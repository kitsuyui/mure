use crate::mure_error::Error;

pub fn get_github_token() -> Result<String, Error> {
    match std::env::var("GH_TOKEN") {
        Ok(token) if token.len() > 0 => Ok(token),
        _ => Err(Error::from_str("GH_TOKEN is not set")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;

    #[assay(
        env = [
          ("GH_TOKEN", ""),
        ]
      )]
    fn test_get_github_token_err() {
        let result = get_github_token();
        assert!(result.is_err());
    }

    #[assay(
        env = [
          ("GH_TOKEN", "test"),
        ]
    )]
    fn test_get_github_token_success() {
        let result = get_github_token();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }
}
