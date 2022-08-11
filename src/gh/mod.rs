use crate::mure_error::Error;
use std::process::Command;

pub fn get_default_branch() -> Result<String, Error> {
    // TODO: if gh is not installed, return error
    let result = Command::new("gh")
        .arg("repo")
        .arg("view")
        .arg("--json")
        .arg("defaultBranchRef")
        .arg("--jq")
        .arg(".defaultBranchRef.name")
        .output()?;
    if !result.status.success() {
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
    #[test]
    fn test_get_default_branch() {
        match get_default_branch() {
            Ok(default_branch) => {
                assert_eq!(default_branch, "main");
            }
            _ => unreachable!("unreachable!"),
        }
    }
}
