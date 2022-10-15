use crate::config::Config;
use crate::github::repo::RepoInfo;
use crate::{config::ConfigSupport, mure_error::Error};
use std::fs as std_fs;
use std::os::unix::fs as unix_fs;
use std::process::Command;

pub fn clone(config: &Config, repo_url: &str) -> Result<(), Error> {
    let parsed = RepoInfo::parse_url(repo_url);
    if parsed.is_none() {
        return Err(Error::from_str("invalid repo url"));
    }
    let repo_info = parsed.unwrap();
    let tobe_clone = config.repo_store_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);

    // create dir if not exist (mkdir -p)
    std_fs::create_dir_all(tobe_clone.as_os_str())?;

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
    unix_fs::symlink(tobe_clone, link_to).unwrap();
    Ok(())
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
            Err(err) => panic!("{:?}", err),
        }
    }
}
