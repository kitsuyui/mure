use crate::config::Config;
use crate::github::repo::RepoInfo;
use crate::{config::ConfigSupport, mure_error::Error};
use std::os::unix::fs;
use std::process::Command;

pub fn clone(config: &Config, repo_url: &str) -> Result<(), Error> {
    let parsed = RepoInfo::parse_url(repo_url);
    if parsed.is_none() {
        return Err(Error::from_str("invalid repo url"));
    }
    let repo_info = parsed.unwrap();
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
