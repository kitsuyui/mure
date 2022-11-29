use crate::config::Config;
use crate::github::repo::RepoInfo;
use crate::{config::ConfigSupport, mure_error::Error};
use std::fs as std_fs;
use std::os::unix::fs as unix_fs;
use std::process::Command;

pub fn clone(config: &Config, repo_url: &str) -> Result<(), Error> {
    let parsed = RepoInfo::parse_url(repo_url);
    let Some(repo_info) = parsed else {
        return Err(Error::from_str("invalid repo url"));
    };
    let tobe_clone = config.repo_store_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);

    // create dir if not exist (mkdir -p)
    std_fs::create_dir_all(tobe_clone.as_os_str())?;

    let Some(parent) = tobe_clone.parent() else {
        return Err(Error::from_str("invalid repo url (maybe root dir)"));
    };

    let result = Command::new("git")
        .current_dir(parent)
        .arg("clone")
        .arg(repo_url)
        .output()?;

    if !result.status.success() {
        let Ok(error) = String::from_utf8(result.stderr) else {
            return Err(Error::from_str("failed to clone"));
        };
        return Err(Error::from_str(&error));
    }

    let link_to = config.repo_work_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);
    match unix_fs::symlink(tobe_clone, link_to) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::from_str("failed to create symlink")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mktemp::Temp;

    #[test]
    fn test_clone() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let config_file = format!(
            r#"
        [core]
        base_dir = "{}"

        [github]
        username = "kitsuyui"

        [shell]
        cd_shims = "mcd"
    "#,
            temp_dir.as_os_str().to_str().unwrap()
        );
        let config: Config = toml::from_str(&config_file).unwrap();

        match clone(&config, "https://github.com/kitsuyui/mure") {
            Ok(_) => {}
            Err(_) => unreachable!(),
        }
        let config: Config = toml::from_str(&config_file).unwrap();

        let Err(error) = clone(&config, "") else {
            unreachable!();
        };
        assert_eq!(error.to_string(), "invalid repo url");
    }
}
