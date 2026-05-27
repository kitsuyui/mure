use crate::config::Config;
use crate::git::RepositorySupport;
use crate::github::repo::RepoInfo;
use crate::verbosity::Verbosity;
use crate::{config::ConfigSupport, mure_error::Error};
use std::fs as std_fs;
use std::os::unix::fs as unix_fs;

pub fn clone(config: &Config, repo_url: &str, verbosity: Verbosity) -> Result<(), Error> {
    let parsed = RepoInfo::parse_url(repo_url);
    let Some(repo_info) = parsed else {
        return Err(Error::from_str("invalid repo url"));
    };
    let tobe_clone = config.repo_store_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);

    // create dir if not exist (mkdir -p)
    std_fs::create_dir_all(tobe_clone.as_os_str())?;

    let parent = tobe_clone
        .parent()
        .ok_or_else(|| Error::from_str("invalid repo url (maybe root dir)"))?
        .to_path_buf();

    let result =
        <git2::Repository as RepositorySupport>::clone(repo_url, &parent).inspect_err(|_| {
            let _ = std_fs::remove_dir_all(&tobe_clone);
        })?;
    match verbosity {
        Verbosity::Quiet => (),
        Verbosity::Normal => {
            eprintln!("{}", result.raw.stderr);
        }
        Verbosity::Verbose => {
            eprintln!("{}", result.raw.stderr);
            println!("{}", result.raw.stdout);
        }
    }

    let link_to = config.repo_work_path(&repo_info.domain, &repo_info.owner, &repo_info.repo);
    let Some(link_parent) = link_to.parent() else {
        let _ = std_fs::remove_dir_all(&tobe_clone);
        return Err(Error::from_str("invalid repo url (maybe root dir)"));
    };
    std_fs::create_dir_all(link_parent).map_err(|e| {
        let _ = std_fs::remove_dir_all(&tobe_clone);
        Error::from(e)
    })?;
    match unix_fs::symlink(&tobe_clone, &link_to) {
        Ok(_) => Ok(()),
        Err(e) => {
            let _ = std_fs::remove_dir_all(&tobe_clone);
            Err(Error::from_str(&format!("failed to create symlink: {e}")))
        }
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
        cd_shims = "mucd"
    "#,
            temp_dir.as_os_str().to_str().unwrap()
        );
        let config: Config = toml::from_str(&config_file).unwrap();

        match clone(
            &config,
            "https://github.com/kitsuyui/mure",
            Verbosity::Normal,
        ) {
            Ok(_) => {}
            Err(_) => unreachable!(),
        }
        assert!(
            config
                .repo_work_path("github.com", "kitsuyui", "mure")
                .is_symlink()
        );
        let config: Config = toml::from_str(&config_file).unwrap();

        let Err(error) = clone(&config, "", Verbosity::Normal) else {
            unreachable!();
        };
        assert_eq!(error.to_string(), "invalid repo url");
    }
}
