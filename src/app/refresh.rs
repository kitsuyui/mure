use std::path::PathBuf;

use git2::Repository;

use crate::gh::get_default_branch;
use crate::git::{PullFastForwardStatus, RepositorySupport};
use crate::mure_error::Error;

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

pub fn refresh(repo_path: &str) -> Result<RefreshStatus, Error> {
    let mut messages = vec![];
    if !PathBuf::from(repo_path).join(".git").exists() {
        return Ok(RefreshStatus::DoNothing(Reason::NotGitRepository));
    }

    let repo = Repository::open(repo_path)?;
    if !repo.is_remote_exists()? {
        return Ok(RefreshStatus::DoNothing(Reason::NoRemote));
    }

    let default_branch = get_default_branch()?;

    // switch to default branch if current branch is clean
    if repo.is_clean()? {
        // git switch $default_branch
        repo.switch(&default_branch)?;
        messages.push(format!("Switched to {}", default_branch));
    }

    // TODO: origin is hardcoded. If you have multiple remotes, you need to specify which one to use.
    let result = repo.pull_fast_forwarded("origin", &default_branch);
    if let Ok(out) = result {
        match out.interpreted_to {
            PullFastForwardStatus::AlreadyUpToDate => {
                // messages.push(out.raw.stderr);
                // messages.push(out.raw.stdout);
                messages.push("Already up to date".to_string());
            }
            PullFastForwardStatus::FastForwarded => {
                // messages.push(out.raw.stderr);
                // messages.push(out.raw.stdout);
                messages.push("Fast-forwarded".to_string());
            }
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
        messages.push(format!("Deleted branch {}", branch));
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
        let result = refresh(origin_path.to_str().unwrap());
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
        let path = fixture.repo.path().parent().unwrap();

        let result = refresh(path.to_str().unwrap());
        match result {
            Ok(RefreshStatus::Update {
                switch_to_default, ..
            }) => {
                assert!(!switch_to_default);
            }
            _ => unreachable!(),
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

        let result = refresh(path).unwrap();
        match result {
            RefreshStatus::DoNothing(Reason::NotGitRepository) => {}
            _ => unreachable!(),
        }
    }
}
