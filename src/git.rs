use git2::{BranchType, Repository};
use std::{
    path::Path,
    process::{Command, Output},
    string::FromUtf8Error,
};

use crate::mure_error;

#[derive(Debug, PartialEq, Eq)]
pub enum PullFastForwardStatus {
    AlreadyUpToDate,
    FastForwarded,
    Abort,
}

#[derive(Debug)]
pub struct RawGitCommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl RawGitCommandOutput {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

impl From<std::process::Output> for RawGitCommandOutput {
    fn from(output: Output) -> Self {
        let status = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8(output.stdout).unwrap_or_default();
        let stderr = String::from_utf8(output.stderr).unwrap_or_default();
        RawGitCommandOutput {
            status,
            stdout,
            stderr,
        }
    }
}

impl TryFrom<RawGitCommandOutput> for GitCommandOutput<()> {
    type Error = Error;

    fn try_from(raw: RawGitCommandOutput) -> Result<Self, Self::Error> {
        match raw.success() {
            true => raw.interpret_to(()),
            false => Err(Error::Raw(raw)),
        }
    }
}

impl RawGitCommandOutput {
    pub fn interpret_to<T>(self, item: T) -> Result<GitCommandOutput<T>, Error> {
        Ok(GitCommandOutput {
            raw: self,
            interpreted_to: item,
        })
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Raw(raw) => write!(f, "{}", raw.stderr),
            Error::FailedToExecute(err) => write!(f, "Failed to execute git command: {}", err),
            Error::CommandNotFound => write!(f, "git command not found"),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Raw(RawGitCommandOutput),
    FailedToExecute(std::io::Error),
    CommandNotFound,
}

#[derive(Debug)]
pub struct GitCommandOutput<T> {
    pub raw: RawGitCommandOutput,
    pub interpreted_to: T,
}

pub trait RepositorySupport {
    fn merged_branches(&self) -> Result<GitCommandOutput<Vec<String>>, Error>;
    fn is_clean(&self) -> Result<bool, mure_error::Error>;
    fn clone(url: &str, into: &Path) -> Result<GitCommandOutput<()>, Error>;
    fn has_unsaved(&self) -> Result<bool, mure_error::Error>;
    fn is_remote_exists(&self) -> Result<bool, mure_error::Error>;
    fn get_current_branch(&self) -> Result<String, mure_error::Error>;
    fn pull_fast_forwarded(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<GitCommandOutput<PullFastForwardStatus>, Error>;
    fn switch(&self, branch: &str) -> Result<GitCommandOutput<()>, Error>;
    fn delete_branch(&self, branch: &str) -> Result<GitCommandOutput<()>, Error>;
    fn command(&self, args: &[&str]) -> Result<RawGitCommandOutput, Error>;
    fn git_command_on_dir(args: &[&str], workdir: &Path) -> Result<RawGitCommandOutput, Error>;
}

impl RepositorySupport for Repository {
    fn merged_branches(&self) -> Result<GitCommandOutput<Vec<String>>, Error> {
        // git for-each-ref --format=%(refname:short) refs/heads/**/* --merged
        let raw = self.command(&[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/**/*",
            "--merged",
        ])?;
        let branches = split_lines(&raw.stdout);
        Ok(GitCommandOutput {
            raw,
            interpreted_to: branches,
        })
    }
    fn is_clean(&self) -> Result<bool, mure_error::Error> {
        Ok(!self.has_unsaved()?)
    }

    fn clone(url: &str, into: &Path) -> Result<GitCommandOutput<()>, Error> {
        Repository::git_command_on_dir(&["clone", url], into)?.try_into()
    }

    fn has_unsaved(&self) -> Result<bool, mure_error::Error> {
        for entry in self.statuses(None)?.iter() {
            match entry.status() {
                git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED => {
                    return Ok(true);
                }
                _ => continue,
            }
        }
        Ok(false)
    }
    fn is_remote_exists(&self) -> Result<bool, mure_error::Error> {
        Ok(!self.remotes()?.is_empty())
    }

    fn get_current_branch(&self) -> Result<String, mure_error::Error> {
        if self.is_empty()? {
            return Err(mure_error::Error::from_str("repository is empty"));
        }
        let head = self.head()?;

        let Some(name) = head.shorthand() else {
            return Err(mure_error::Error::from_str("head is not a branch"));
        };
        let branch = self.find_branch(name, BranchType::Local)?;
        let Some(branch_name) = branch.name()? else {
            return Err(mure_error::Error::from_str("branch name is not found"));
        };
        Ok(branch_name.to_string())
    }

    fn pull_fast_forwarded(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<GitCommandOutput<PullFastForwardStatus>, Error> {
        let raw = self.command(&["pull", "--ff-only", remote, branch])?;
        let status = {
            let message = raw.stdout.to_string();
            if message.contains("Already up to date.") {
                PullFastForwardStatus::AlreadyUpToDate
            } else if message.contains("Fast-forward") {
                PullFastForwardStatus::FastForwarded
            } else {
                PullFastForwardStatus::Abort
            }
        };
        Ok(GitCommandOutput {
            raw,
            interpreted_to: status,
        })
    }

    fn switch(&self, branch: &str) -> Result<GitCommandOutput<()>, Error> {
        self.command(&["switch", branch])?.try_into()
    }

    fn delete_branch(&self, branch: &str) -> Result<GitCommandOutput<()>, Error> {
        self.command(&["branch", "-d", branch])?.try_into()
    }

    fn git_command_on_dir(args: &[&str], workdir: &Path) -> Result<RawGitCommandOutput, Error> {
        let output = Command::new("git").current_dir(workdir).args(args).output();
        match output {
            Ok(out) => Ok(RawGitCommandOutput::from(out)),
            Err(err) => Err(Error::FailedToExecute(err)),
        }
    }

    fn command(&self, args: &[&str]) -> Result<RawGitCommandOutput, Error> {
        let Some(workdir) = self.workdir() else {
            return Err(Error::CommandNotFound);
        };
        Self::git_command_on_dir(args, workdir)
    }
}

impl From<git2::Error> for mure_error::Error {
    fn from(e: git2::Error) -> mure_error::Error {
        mure_error::Error::from_str(&e.to_string())
    }
}

impl From<FromUtf8Error> for mure_error::Error {
    fn from(e: FromUtf8Error) -> mure_error::Error {
        mure_error::Error::from_str(&e.to_string())
    }
}

fn split_lines(lines: &str) -> Vec<String> {
    lines
        .to_string()
        .split('\n')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    use mktemp::Temp;

    #[test]
    fn test_split_lines() {
        let lines = "a\nb\nc\n";
        let expected = vec!["a", "b", "c"];
        assert_eq!(split_lines(lines), expected);
    }

    #[test]
    fn test_merged_branches() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        // git remote add origin
        let example_repo_url = "https://github.com/kitsuyui/kitsuyui.git";
        repo.remote_set_url("origin", example_repo_url)
            .expect("failed to set remote url");

        fixture.create_empty_commit("initial commit").unwrap();

        // create a first branch
        repo.command(&["switch", "-c", "main"])
            .expect("failed to switch to main branch");

        // create a new branch for testing
        let branch_name = "test";
        // git switch -c $branch_name
        repo.command(&["switch", "-c", branch_name])
            .expect("failed to switch to test branch");

        // switch to default branch
        repo.switch("main")
            .expect("failed to switch to main branch");

        // git merge $branch_name
        repo.command(&["merge", branch_name])
            .expect("failed to merge test branch");

        // now test_branch is same as default branch so it should be merged
        let Ok(GitCommandOutput { interpreted_to: merged_branches , ..}) = repo.merged_branches() else {
            unreachable!();
        };
        assert!(merged_branches.contains(&branch_name.to_string()));
    }

    #[test]
    fn test_is_empty() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        // repo is empty when just initialized
        assert!(repo.is_empty().unwrap());

        fixture.create_empty_commit("initial commit").unwrap();

        // repo is not empty after commit
        assert!(!repo.is_empty().unwrap());
    }

    #[test]
    fn test_is_remote_exists() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        // remote is not exists when initialized
        assert!(!repo.is_remote_exists().unwrap());

        // git remote add origin
        let example_repo_url = "https://github.com/kitsuyui/kitsuyui.git";
        repo.remote_set_url("origin", example_repo_url)
            .expect("failed to set remote url");

        // now remote must be set
        assert!(repo
            .is_remote_exists()
            .expect("failed to check remote exists"));
    }

    #[test]
    fn test_has_unsaved_and_is_clean() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        // repo is clean when initialized
        assert!(repo.is_clean().unwrap() && !repo.has_unsaved().unwrap());

        fixture.create_file("1.txt", "hello").unwrap();

        // repo is dirty because of file
        assert!(!repo.is_clean().unwrap() && repo.has_unsaved().unwrap());

        repo.command(&["add", "1.txt"])
            .expect("failed to add 1.txt");

        // staged but not committed file is dirty
        assert!(!repo.is_clean().unwrap() && repo.has_unsaved().unwrap(),);

        repo.command(&["commit", "-m", "add 1.txt"])
            .expect("failed to commit");

        // repo is clean because of committed file
        assert!(repo.is_clean().unwrap() && !repo.has_unsaved().unwrap());

        repo.command(&["switch", "-c", "feature"])
            .expect("failed to switch to feature branch");

        fixture.create_file("2.txt", "hello").unwrap();

        // repo is dirty because of file
        assert!(!repo.is_clean().unwrap() && repo.has_unsaved().unwrap());

        repo.command(&["add", "2.txt"])
            .expect("failed to add 2.txt");

        // staged but not committed file is dirty
        assert!(!repo.is_clean().unwrap() && repo.has_unsaved().unwrap());

        repo.command(&["commit", "-m", "add 2.txt"])
            .expect("failed to commit");

        // repo is clean because of committed file
        assert!(repo.is_clean().unwrap() && !repo.has_unsaved().unwrap());
    }

    #[test]
    fn test_pull_fast_forwarded() {
        let fixture1 = Fixture::create().unwrap();
        let repo1 = &fixture1.repo;

        let fixture2 = Fixture::create().unwrap();
        let repo2 = &fixture2.repo;

        fixture1.create_empty_commit("initial commit").unwrap();
        repo1
            .command(&["switch", "-c", "main"])
            .expect("failed to switch to main branch");

        let remote_path = format!("{}{}", repo1.workdir().unwrap().to_str().unwrap(), ".git");
        repo2
            .command(&["remote", "add", "origin", &remote_path])
            .expect("failed to add remote");
        repo2
            .command(&["checkout", "-b", "main", "origin/main"])
            .expect("failed to fetch");

        fixture1.create_empty_commit("second commit").unwrap();
        repo2.pull_fast_forwarded("origin", "main").unwrap();

        fixture1.create_empty_commit("commit A").unwrap();
        fixture2.create_empty_commit("commit B").unwrap();
        let result = repo2.pull_fast_forwarded("origin", "main").unwrap();
        assert_eq!(result.interpreted_to, PullFastForwardStatus::Abort);
    }

    #[test]
    fn test_get_current_branch() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        let Err(_) = repo.get_current_branch() else {
            unreachable!();
        };

        fixture.create_empty_commit("initial commit").unwrap();
        let Ok(branch_name) = repo.get_current_branch() else {
            unreachable!();
        };
        assert!(branch_name == "main" || branch_name == "master");
    }

    #[test]
    fn test_switch() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        // switch to main branch before first commit will fail
        assert!(repo.switch("main").is_err());
        fixture.create_empty_commit("initial commit").unwrap();

        repo.command(&["switch", "-c", "main"])
            .expect("failed to switch to main branch");

        repo.command(&["switch", "-c", "feature"])
            .expect("failed to switch to main branch");

        repo.switch("main")
            .expect("failed to switch to main branch");
    }

    #[test]
    fn test_delete_branch() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        fixture.create_empty_commit("initial commit").unwrap();
        repo.command(&["switch", "-c", "main"])
            .expect("failed to switch to main branch");

        repo.command(&["switch", "-c", "feature"])
            .expect("failed to switch to feature branch");

        repo.switch("main")
            .expect("failed to switch to main branch");

        let count_before = repo.branches(None).unwrap().count();

        repo.delete_branch("feature")
            .expect("failed to delete feature branch");

        let count_after = repo.branches(None).unwrap().count();

        // feature branch must be deleted
        // note: count_before may be 2 or 3 depending on git config --global init.defaultBranch
        assert_eq!(count_before - count_after, 1);

        // try to delete already deleted branch again
        let result = repo.delete_branch("feature");
        match result {
            Err(err) => {
                let Error::Raw(raw) = err else {
                    unreachable!();
                };
                assert_eq!(raw.stderr, "error: branch 'feature' not found.\n");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_clone() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let repo_url = "https://github.com/kitsuyui/mure";
        let result = <git2::Repository as RepositorySupport>::clone(repo_url, temp_dir.as_path());
        assert!(result.is_ok());

        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let repo_url = "";
        let result = <git2::Repository as RepositorySupport>::clone(repo_url, temp_dir.as_path());

        match result {
            Err(err) => {
                let Error::Raw(raw) = err else {
                    unreachable!();
                };
                assert_eq!(raw.stderr, "fatal: repository '' does not exist\n");
            }
            _ => unreachable!(),
        }
    }
}
