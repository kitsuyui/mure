use std::process::Command;

use crate::{config::ConfigSupport, mure_error::Error};
use regex::Regex;
use std::os::unix::fs;

use crate::config::Config;

const GITHUB_HTTPS_URL: &str =
    "^https?://(?P<domain>github\\.com)/(?P<owner>.*?)/(?P<repo>.*?)(/?|(?:\\.git))$";
const GITHUB_GIT_URL: &str =
    "^git@(?P<domain>github\\.com):(?P<owner>.*?)/(?P<repo>.*?)(?:\\.git)?$";
const GITHUB_SSH_URL: &str =
    "^ssh://git@(?P<domain>github\\.com)(?::22)?/(?P<owner>.*?)/(?P<repo>.*?)(?:\\.git)$";

pub fn clone(config: &Config, repo_url: &str) -> Result<(), Error> {
    if !is_github_repo(repo_url) {
        return Err(Error::from_str("not a github repo"));
    }
    let repo_info = parse_github_url(repo_url).unwrap();

    let tobe_clone = config.repo_store_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);

    if tobe_clone.exists() {
        return Err(Error::from_str("repo already exists"));
    }

    let result = Command::new("git")
        .current_dir(tobe_clone.parent().unwrap())
        .arg("clone")
        .arg(&repo_url)
        .output()?;

    if !result.status.success() {
        let error = String::from_utf8(result.stderr).unwrap();
        return Err(Error::from_str(&error));
    }

    let link_to = config.repo_work_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);
    fs::symlink(tobe_clone, link_to).unwrap();
    Ok(())
}

struct GithubRepo {
    pub domain: String,
    pub owner: String,
    pub repo: String,
}

fn parse_github_url(repo_url: &str) -> Result<GithubRepo, Error> {
    let re1 = Regex::new(GITHUB_HTTPS_URL).unwrap();
    let re2 = Regex::new(GITHUB_GIT_URL).unwrap();
    let re3 = Regex::new(GITHUB_SSH_URL).unwrap();

    if re1.is_match(repo_url) {
        let captures = re1.captures(repo_url).unwrap();
        let domain = captures.name("domain").unwrap().as_str();
        let owner = captures.name("owner").unwrap().as_str();
        let repo = captures.name("repo").unwrap().as_str();
        return Ok(GithubRepo {
            domain: domain.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        });
    } else if re2.is_match(repo_url) {
        let captures = re2.captures(repo_url).unwrap();
        let domain = captures.name("domain").unwrap().as_str();
        let owner = captures.name("owner").unwrap().as_str();
        let repo = captures.name("repo").unwrap().as_str();
        return Ok(GithubRepo {
            domain: domain.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        });
    } else if re3.is_match(repo_url) {
        let captures = re3.captures(repo_url).unwrap();
        let domain = captures.name("domain").unwrap().as_str();
        let owner = captures.name("owner").unwrap().as_str();
        let repo = captures.name("repo").unwrap().as_str();
        return Ok(GithubRepo {
            domain: domain.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        });
    }
    Err(Error::from_str("not a github repo"))
}

fn is_github_repo(repo_url: &str) -> bool {
    parse_github_url(repo_url).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_is_github_repo() {
        // match
        assert!(is_github_repo("https://github.com/kitsuyui/mure"));
        assert!(is_github_repo("https://github.com/kitsuyui/mure.git"));
        assert!(is_github_repo("git@github.com:kitsuyui/mure.git"));
        assert!(is_github_repo("ssh://git@github.com:22/kitsuyui/mure.git"));
        assert!(is_github_repo("ssh://git@github.com/kitsuyui/mure.git"));

        // not match
        assert!(!is_github_repo("https://github.com/"));
        assert!(!is_github_repo("https://example.com/something/else"));
        assert!(!is_github_repo("git@example.com:kitsuyui/mure.git"));
        assert!(!is_github_repo("ssh://git@example.com/kitsuyui/mure.git"));
    }

    #[test]
    fn test_parse_github_url() {
        for url in [
            "https://github.com/kitsuyui/mure",
            "https://github.com/kitsuyui/mure.git",
            "git@github.com:kitsuyui/mure.git",
            "ssh://git@github.com:22/kitsuyui/mure.git",
            "ssh://git@github.com/kitsuyui/mure.git",
        ] {
            match parse_github_url(url) {
                Ok(info) => {
                    assert_eq!(info.domain, "github.com");
                    assert_eq!(info.owner, "kitsuyui");
                    assert_eq!(info.repo, "mure");
                }
                _ => unreachable!("something went wrong"),
            }
        }
    }
}
