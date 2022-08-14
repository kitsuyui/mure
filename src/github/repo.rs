use regex::Regex;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct RepoInfo {
    pub domain: String,
    pub owner: String,
    pub repo: String,
}

impl RepoInfo {
    fn new(domain: &str, owner: &str, repo: &str) -> Self {
        RepoInfo {
            domain: domain.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }
    pub fn parse_url(url: &str) -> Option<Self> {
        let patterns = [
            Regex::new(GITHUB_HTTPS_URL).unwrap(),
            Regex::new(GITHUB_GIT_URL).unwrap(),
            Regex::new(GITHUB_SSH_URL).unwrap(),
        ];
        for pattern in patterns.iter() {
            if let Some(repo_info) = RepoInfo::parse_with_regex(pattern, url) {
                return Some(repo_info);
            }
        }
        None
    }
    fn parse_with_regex(pattern: &Regex, url: &str) -> Option<Self> {
        if let Some(caps) = pattern.captures(url) {
            let domain = caps.name("domain")?.as_str();
            let owner = caps.name("owner")?.as_str();
            let repo = caps.name("repo")?.as_str();
            return Some(RepoInfo::new(domain, owner, repo));
        }
        None
    }
}

const GITHUB_HTTPS_URL: &str =
    "^https?://(?P<domain>github\\.com)/(?P<owner>.*?)/(?P<repo>.*?)(/?|(?:\\.git))$";
const GITHUB_GIT_URL: &str =
    "^git@(?P<domain>github\\.com):(?P<owner>.*?)/(?P<repo>.*?)(?:\\.git)?$";
const GITHUB_SSH_URL: &str =
    "^ssh://git@(?P<domain>github\\.com)(?::22)?/(?P<owner>.*?)/(?P<repo>.*?)(?:\\.git)$";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_github_url() {
        let tobe = RepoInfo::new("github.com", "kitsuyui", "mure");
        fn parse(url: &str) -> Option<RepoInfo> {
            RepoInfo::parse_url(url)
        }

        // match
        assert_eq!(parse("https://github.com/kitsuyui/mure").unwrap(), tobe);
        assert_eq!(parse("https://github.com/kitsuyui/mure.git").unwrap(), tobe);
        assert_eq!(parse("git@github.com:kitsuyui/mure.git").unwrap(), tobe);
        assert_eq!(
            parse("ssh://git@github.com:22/kitsuyui/mure.git").unwrap(),
            tobe
        );
        assert_eq!(
            parse("ssh://git@github.com/kitsuyui/mure.git").unwrap(),
            tobe
        );

        // not match
        assert!(parse("https://github.com/").is_none());
        assert!(parse("https://example.com/something/else").is_none());
        assert!(parse("git@example.com:kitsuyui/mure.git").is_none());
        assert!(parse("ssh://git@example.com/kitsuyui/mure.git").is_none());
    }
}
