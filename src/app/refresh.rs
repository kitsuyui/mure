use std::path::PathBuf;

use git2::Repository;

use crate::config::Config;
use crate::gh::get_default_branch;
use crate::git::{PullFastForwardStatus, RepositorySupport};
use crate::mure_error::Error;
use crate::verbosity::Verbosity;

use super::list::search_mure_repo;

pub fn refresh_main(
    config: &Config,
    all: bool,
    repository: Option<String>,
    verbosity: Verbosity,
) -> Result<(), Error> {
    if all {
        refresh_all(config, verbosity)?;
    } else {
        let current_dir = std::env::current_dir()?;
        let Some(current_dir) = current_dir.to_str() else {
            return Err(Error::from_str("failed to get current dir"));
        };
        let repo_path = match repository {
            Some(repo) => repo,
            None => current_dir.to_string(),
        };
        match refresh(&repo_path, verbosity) {
            Ok(r) => {
                if let RefreshStatus::Update { message, .. } = r {
                    println!("{message}");
                }
            }
            Err(e) => println!("{e}"),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum RefreshStatus {
    DoNothing(Reason),
    Update {
        switch_to_default: bool,
        message: String,
    },
}

#[derive(Debug)]
pub enum Reason {
    NotGitRepository,
    NoRemote,
}

pub fn refresh_all(config: &Config, verbosity: Verbosity) -> Result<(), Error> {
    let repos = search_mure_repo(config);
    if repos.is_empty() {
        println!("No repositories found");
        return Ok(());
    }
    for repo in repos {
        match repo {
            Ok(mure_repo) => {
                println!("> Refreshing {}", mure_repo.repo.repo);
                let result = refresh(
                    #[allow(clippy::expect_used)]
                    mure_repo
                        .absolute_path
                        .to_str()
                        .expect("failed to convert to str"),
                    verbosity,
                );
                match result {
                    Ok(status) => match status {
                        RefreshStatus::DoNothing(reason) => match reason {
                            Reason::NotGitRepository => {
                                println!("{} is not a git repository", mure_repo.repo.repo)
                            }
                            Reason::NoRemote => {
                                println!("{} has no remote", mure_repo.repo.repo)
                            }
                        },
                        RefreshStatus::Update {
                            switch_to_default,
                            message,
                        } => {
                            if switch_to_default {
                                println!("Switched to {}", mure_repo.repo.repo)
                            }
                            println!("{message}")
                        }
                    },
                    Err(e) => {
                        println!("{}", e.message());
                    }
                }
            }
            Err(e) => {
                println!("{}", e.message());
            }
        }
    }
    Ok(())
}

pub fn refresh(repo_path: &str, verbosity: Verbosity) -> Result<RefreshStatus, Error> {
    let mut messages = vec![];
    if !PathBuf::from(repo_path).join(".git").exists() {
        return Ok(RefreshStatus::DoNothing(Reason::NotGitRepository));
    }

    let repo = Repository::open(repo_path)?;
    if !repo.is_remote_exists()? {
        return Ok(RefreshStatus::DoNothing(Reason::NoRemote));
    }

    let default_branch = get_default_branch(&repo_path.into())?;

    repo.fetch_prune()?;

    // switch to default branch if current branch is clean
    if repo.is_clean()? {
        // git switch $default_branch
        repo.switch(&default_branch)?;
        messages.push(format!("Switched to {default_branch}"));
    }

    // TODO: origin is hardcoded. If you have multiple remotes, you need to specify which one to use.
    let result = repo.pull_fast_forwarded("origin", &default_branch);
    if let Ok(out) = result {
        match out.interpreted_to {
            PullFastForwardStatus::AlreadyUpToDate => match verbosity {
                Verbosity::Quiet => (),
                Verbosity::Normal => {
                    messages.push("Already up to date".to_string());
                }
                Verbosity::Verbose => {
                    messages.push("Already up to date".to_string());
                    messages.push(out.raw.stderr);
                    messages.push(out.raw.stdout);
                }
            },
            PullFastForwardStatus::FastForwarded => match verbosity {
                Verbosity::Quiet => (),
                Verbosity::Normal => {
                    messages.push("Fast-forwarded".to_string());
                }
                Verbosity::Verbose => {
                    messages.push("Fast-forwarded".to_string());
                    messages.push(out.raw.stderr);
                    messages.push(out.raw.stdout);
                }
            },
            _ => (),
        };
    }

    let merged_branches = repo.merged_branches()?.interpreted_to;
    let delete_branches = merged_branches
        .iter()
        .filter(|&branch| !branch.eq(&default_branch))
        .collect::<Vec<_>>();

    for branch in delete_branches {
        repo.delete_branch(branch)?;
        messages.push(format!("Deleted branch {branch}"));
    }

    Ok(RefreshStatus::Update {
        switch_to_default: false,
        message: messages.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    use mktemp::Temp;

    #[test]
    fn test_refresh() {
        let fixture = Fixture::create().unwrap();
        let fixture_origin = Fixture::create().unwrap();

        let origin_path = fixture_origin.repo.path().parent().unwrap();
        fixture_origin
            .create_empty_commit("initial commit")
            .unwrap();
        fixture_origin
            .repo
            .command(&["switch", "-c", "main"])
            .unwrap();
        let result = refresh(origin_path.to_str().unwrap(), Verbosity::Normal);
        match result {
            Ok(RefreshStatus::DoNothing(Reason::NoRemote)) => (),
            _ => unreachable!(),
        }

        fixture
            .repo
            .remote("origin", origin_path.to_str().unwrap())
            .unwrap();
        fixture.repo.command(&["fetch", "origin"]).unwrap();
        fixture.repo.command(&["switch", "main"]).unwrap();
        fixture
            .repo
            .command(&[
                "remote",
                "set-url",
                "origin",
                "https://github.com/kitsuyui/mure.git",
            ])
            .unwrap();
        let path = fixture.repo.path().parent().unwrap();

        let result = refresh(path.to_str().unwrap(), Verbosity::Normal);
        match result {
            Ok(RefreshStatus::Update {
                switch_to_default, ..
            }) => {
                assert!(!switch_to_default);
            }
            Ok(resut) => unreachable!("{:?}", resut),
            Err(e) => unreachable!("{:?}", e),
        }
        drop(fixture_origin);
        drop(fixture);
    }

    #[test]
    fn test_not_git_repository() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");

        let result = refresh(path, Verbosity::Normal).unwrap();
        match result {
            RefreshStatus::DoNothing(Reason::NotGitRepository) => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_no_remote() {
        let fixture = Fixture::create().unwrap();
        let path = fixture.repo.path().parent().unwrap();

        let result = refresh(path.to_str().unwrap(), Verbosity::Normal).unwrap();
        match result {
            RefreshStatus::DoNothing(Reason::NoRemote) => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_refresh_all() {
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

        refresh_all(&config, Verbosity::Verbose).unwrap();
    }
}
