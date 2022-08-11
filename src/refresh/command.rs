use git2::Repository;

use crate::gh::get_default_branch;
use crate::git::RepositoryWrapper;
use crate::mure_error::Error;
use std::process::Command;

pub enum RefreshStatus {
    DoNothing(Reason),
    Update {
        switch_to_default: bool,
        message: String,
    },
}

pub enum Reason {
    NoRemote,
    EmptyRepository,
}

pub fn refresh(repo_path: &str) -> Result<RefreshStatus, Error> {
    let repo = Repository::open(repo_path)?;

    if repo.is_empty()? {
        return Ok(RefreshStatus::DoNothing(Reason::EmptyRepository));
    }
    if !repo.is_remote_exists()? {
        return Ok(RefreshStatus::DoNothing(Reason::NoRemote));
    }

    let default_branch = get_default_branch()?;
    // git pull --ff-only origin "$default_branch":"$default_branch"
    Command::new("git")
        .arg("pull")
        .arg("--ff-only")
        .arg("origin")
        .arg(&default_branch)
        .arg(&default_branch)
        .output()?;

    // switch to default branch if current branch is clean
    if repo.is_clean()? {
        // git switch $default_branch
        let result = Command::new("git")
            .current_dir(repo_path)
            .arg("switch")
            .arg(&default_branch)
            .output()?;

        return Ok(RefreshStatus::Update {
            switch_to_default: true,
            message: String::from_utf8(result.stdout).unwrap(),
        });
    }
    Ok(RefreshStatus::Update {
        switch_to_default: false,
        message: String::from(""),
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
}
