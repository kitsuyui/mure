use std::path::{Path, PathBuf};

use crate::config::{Config, ConfigSupport};
use crate::github::repo::RepoInfo;
use crate::mure_error::Error;

pub fn list(config: &Config, path: bool, full: bool) -> Result<(), Error> {
    let repos = search_mure_repo(config);
    if repos.is_empty() {
        println!("No repositories found");
        return Ok(());
    }
    let mut first_error = None;
    for repo in repos {
        match repo {
            Ok(mure_repo) => {
                if full && path {
                    #[allow(clippy::expect_used)]
                    let abpath = mure_repo
                        .absolute_path
                        .to_str()
                        .expect("failed to convert to str");
                    println!("{abpath}");
                } else if full {
                    println!("{}", mure_repo.repo.name_with_owner());
                } else if path {
                    #[allow(clippy::expect_used)]
                    let relpath = mure_repo
                        .relative_path
                        .to_str()
                        .expect("failed to convert to str");
                    println!("{relpath}");
                } else {
                    println!("{}", mure_repo.repo.repo);
                }
            }
            Err(e) => {
                println!("{}", e.message());
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub struct MureRepo {
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
    pub repo: RepoInfo,
}

pub fn search_mure_repo(config: &Config) -> Vec<Result<MureRepo, Error>> {
    let mut repos = vec![];
    collect_mure_repos(&config.base_path(), &config.repos_store_path(), &mut repos);
    if repos.is_empty() && !config.base_path().is_dir() {
        repos.push(Err(Error::from_str("failed to read dir")));
    }
    repos
}

fn collect_mure_repos(
    path: &Path,
    repo_store_path: &Path,
    repos: &mut Vec<Result<MureRepo, Error>>,
) {
    let Ok(dir) = path.read_dir() else {
        return;
    };
    dir.for_each(|entry| {
        let Ok(entry) = entry else {
            return;
        };
        let entry_path = entry.path();
        let metadata = match std::fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,
            Err(_) => return,
        };
        if metadata.is_symlink() {
            match read_symlink_as_mure_repo(&entry_path) {
                Ok(mure_repo) => repos.push(Ok(mure_repo)),
                Err(e) => repos.push(Err(e)),
            }
        } else if metadata.is_dir() && entry_path != repo_store_path {
            collect_mure_repos(&entry_path, repo_store_path, repos);
        }
    });
}

pub fn find_mure_repo(config: &Config, name: &str) -> Result<MureRepo, Error> {
    let mut matches = search_mure_repo(config)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|mure_repo| {
            mure_repo.repo.repo == name
                || mure_repo.repo.name_with_owner() == name
                || mure_repo.repo.fully_qualified_name() == name
        })
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(Error::from_str(
            format!("{} is not a git repository", name).as_str(),
        )),
        1 => Ok(matches.remove(0)),
        _ => Err(Error::from_str(
            format!("multiple repositories match {name}; use owner/repo or domain/owner/repo")
                .as_str(),
        )),
    }
}

fn read_symlink_as_mure_repo(path: &PathBuf) -> Result<MureRepo, Error> {
    let absolute_path = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return Err(Error::from_str("failed to get absolute path")),
    };
    let Some(owner) = absolute_path.parent() else {
        return Err(Error::from_str("failed to get owner"));
    };
    let Some(domain) = owner.parent() else {
        return Err(Error::from_str("failed to get domain"));
    };
    let repo_name = match absolute_path.file_name() {
        Some(path) => match path.to_str() {
            Some(path) => path.to_string(),
            None => return Err(Error::from_str("failed to get repo name")),
        },
        None => return Err(Error::from_str("failed to get repo name")),
    };
    let repo = match (owner.file_name(), domain.file_name()) {
        (Some(owner), Some(domain)) => RepoInfo {
            owner: owner.to_string_lossy().to_string(),
            domain: domain.to_string_lossy().to_string(),
            repo: repo_name,
        },
        _ => return Err(Error::from_str("failed to get owner or domain")),
    };
    Ok(MureRepo {
        relative_path: path.clone(),
        absolute_path,
        repo,
    })
}

#[cfg(test)]
mod tests {
    use mktemp::Temp;

    use crate::verbosity::Verbosity;

    use super::*;

    #[test]
    fn test_search_mure_repo() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");

        let config: Config = toml::from_str(
            format!(
                r#"
            [core]
            base_dir = "{}"

            [github]
            username = "kitsuyui"

            [shell]
            cd_shims = "mucd"
        "#,
                temp_dir.to_str().unwrap()
            )
            .as_str(),
        )
        .unwrap();
        let repos = search_mure_repo(&config);
        assert_eq!(repos.len(), 0);
        crate::app::clone::clone(
            &config,
            "https://github.com/kitsuyui/mure",
            Verbosity::Normal,
        )
        .unwrap();

        let repos = search_mure_repo(&config);
        assert_eq!(repos.len(), 1);

        for repo in repos {
            assert!(repo.is_ok());
            let mure_repo = repo.unwrap();
            assert_eq!(
                mure_repo.repo.fully_qualified_name(),
                "github.com/kitsuyui/mure"
            );
            assert_eq!(mure_repo.repo.name_with_owner(), "kitsuyui/mure");
            assert_eq!(mure_repo.repo.owner, "kitsuyui");
            assert_eq!(mure_repo.repo.domain, "github.com");
            assert_eq!(mure_repo.repo.repo, "mure");
        }
    }

    #[test]
    fn test_find_mure_repo_by_short_and_full_names() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");

        let config: Config = toml::from_str(
            format!(
                r#"
            [core]
            base_dir = "{}"

            [github]
            username = "kitsuyui"

            [shell]
            cd_shims = "mucd"
        "#,
                temp_dir.to_str().unwrap()
            )
            .as_str(),
        )
        .unwrap();
        crate::app::clone::clone(
            &config,
            "https://github.com/kitsuyui/mure",
            Verbosity::Normal,
        )
        .unwrap();

        assert_eq!(
            find_mure_repo(&config, "mure")
                .unwrap()
                .repo
                .fully_qualified_name(),
            "github.com/kitsuyui/mure"
        );
        assert_eq!(
            find_mure_repo(&config, "kitsuyui/mure")
                .unwrap()
                .repo
                .fully_qualified_name(),
            "github.com/kitsuyui/mure"
        );
        assert_eq!(
            find_mure_repo(&config, "github.com/kitsuyui/mure")
                .unwrap()
                .repo
                .fully_qualified_name(),
            "github.com/kitsuyui/mure"
        );
    }

    #[test]
    fn test_find_mure_repo_reports_ambiguous_short_name() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");

        let config: Config = toml::from_str(
            format!(
                r#"
            [core]
            base_dir = "{}"

            [github]
            username = "kitsuyui"

            [shell]
            cd_shims = "mucd"
        "#,
                temp_dir.to_str().unwrap()
            )
            .as_str(),
        )
        .unwrap();
        let first_store = config.repo_store_path("github.com", "alice", "project");
        let second_store = config.repo_store_path("github.com", "bob", "project");
        std::fs::create_dir_all(first_store.parent().unwrap()).unwrap();
        std::fs::create_dir_all(second_store.parent().unwrap()).unwrap();
        git2::Repository::init(&first_store).unwrap();
        git2::Repository::init(&second_store).unwrap();
        let first_work = config.repo_work_path("github.com", "alice", "project");
        let second_work = config.repo_work_path("github.com", "bob", "project");
        std::fs::create_dir_all(first_work.parent().unwrap()).unwrap();
        std::fs::create_dir_all(second_work.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&first_store, first_work).unwrap();
        std::os::unix::fs::symlink(&second_store, second_work).unwrap();

        let Err(err) = find_mure_repo(&config, "project") else {
            panic!("short repo name should be ambiguous");
        };
        assert!(
            err.to_string()
                .contains("multiple repositories match project")
        );
        assert_eq!(
            find_mure_repo(&config, "alice/project")
                .unwrap()
                .repo
                .fully_qualified_name(),
            "github.com/alice/project"
        );
    }

    #[test]
    fn test_app() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");

        let config: Config = toml::from_str(
            format!(
                r#"
            [core]
            base_dir = "{}"

            [github]
            username = "kitsuyui"

            [shell]
            cd_shims = "mucd"
        "#,
                temp_dir.to_str().unwrap()
            )
            .as_str(),
        )
        .unwrap();
        crate::app::clone::clone(
            &config,
            "https://github.com/kitsuyui/mure",
            Verbosity::Normal,
        )
        .unwrap();
        let repos = search_mure_repo(&config);
        assert_eq!(repos.len(), 1);

        // list doesn't panic
        list(&config, false, false).unwrap();
        list(&config, true, false).unwrap();
        list(&config, false, true).unwrap();
        list(&config, true, true).unwrap();
    }
}
