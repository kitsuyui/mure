use crate::mure_error::Error;
use std::process::Command;

pub fn get_default_branch() -> Result<String, Error> {
    let result = match Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "defaultBranchRef",
            "-t",
            "{{.defaultBranchRef.name}}",
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => return Err(Error::GHCommandError(e.to_string())),
    };

    if !result.status.success() {
        let Ok(message) = String::from_utf8(result.stderr) else {
            return Err(Error::from_str("failed to get default branch"));
        };
        return Err(Error::from_str(&message));
    }

    let Ok(message) = String::from_utf8(result.stdout) else {
        return Err(Error::from_str("failed to get default branch"));
    };
    Ok(message)
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
