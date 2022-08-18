use crate::mure_error::Error;
use std::process::Command;

pub fn get_default_branch() -> Result<String, Error> {
    let result = Command::new("gh")
        .arg("repo")
        .arg("view")
        .arg("--json")
        .arg("defaultBranchRef")
        .arg("--jq")
        .arg(".defaultBranchRef.name")
        .output()?;

    if !result.status.success() {
        // TODO: better error message
        let error = String::from_utf8(result.stderr).unwrap();
        return Err(Error::from_str(&error));
    }

    let raw_branch_name = String::from_utf8(result.stdout.to_vec());
    match raw_branch_name {
        Ok(branch_name) => Ok(branch_name.trim_end_matches('\n').to_string()),
        Err(e) => Err(Error::from_str(&e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;

    #[test]
    fn test_get_default_branch() {
        assert_eq!(get_default_branch().unwrap(), "main");
    }

    #[assay(
        env = [
          ("PATH", ""),
        ]
      )]
    fn test_gh_is_not_installed() {
        let result = get_default_branch();
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "No such file or directory (os error 2)"
        );
    }

    #[assay(
        env = [
          ("GH_TOKEN", ""),
        ]
      )]
    fn test_gh_token_is_not_set() {
        let result = get_default_branch();
        assert!(result.is_err());
    }
}
