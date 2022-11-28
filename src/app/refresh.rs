use std::path::PathBuf;

use git2::Repository;

use crate::gh::get_default_branch;
use crate::git::RepositorySupport;
use crate::mure_error::Error;

pub enum RefreshStatus {
    DoNothing(Reason),
    Update {
        switch_to_default: bool,
        message: String,
    },
}

pub enum Reason {
    NotGitRepository,
    NoRemote,
    EmptyRepository,
}

pub fn refresh(repo_path: &str) -> Result<RefreshStatus, Error> {
    let mut messages = vec![];
    if !PathBuf::from(repo_path).join(".git").exists() {
        return Ok(RefreshStatus::DoNothing(Reason::NotGitRepository));
    }

    let repo = Repository::open(repo_path)?;

    if repo.is_empty()? {
        return Ok(RefreshStatus::DoNothing(Reason::EmptyRepository));
    }
    if !repo.is_remote_exists()? {
        return Ok(RefreshStatus::DoNothing(Reason::NoRemote));
    }

    let default_branch = get_default_branch()?;
    // git pull --ff-only origin "$default_branch":"$default_branch"

    // TODO: origin is hardcoded. If you have multiple remotes, you need to specify which one to use.
    repo.pull_fast_forwarded("origin", &default_branch)?;
    messages.push(format!(
        "Pulled from origin/{} into {}",
        default_branch, default_branch
    ));

    // switch to default branch if current branch is clean
    if repo.is_clean()? {
        // git switch $default_branch
        let result = repo.switch(&default_branch)?;
        if result.status.success() {
            messages.push(format!("Switched to {}", default_branch));
        } else {
            let message =
                String::from_utf8(result.stdout).map_err(|e| Error::from_str(&e.to_string()))?;
            messages.push(message);
        }
    }

    let merged_branches = repo.merged_branches()?;
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
    use mktemp::Temp;

    #[test]
    fn test_refresh() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");
        Repository::init(path).unwrap();

        let result = refresh(path).unwrap();
        match result {
            RefreshStatus::DoNothing(Reason::EmptyRepository) => {}
            _ => unreachable!(),
        }
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
