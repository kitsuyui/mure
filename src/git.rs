use std::{
    path::Path,
    process::{Command, Output},
    string::FromUtf8Error,
};

use git2::{BranchType, Repository};

use crate::mure_error::Error;

pub trait RepositorySupport {
    fn merged_branches(&self) -> Result<Vec<String>, Error>;
    fn is_clean(&self) -> Result<bool, Error>;
    fn clone(url: &str, into: &Path) -> Result<(), Error>;
    fn has_unsaved(&self) -> Result<bool, Error>;
    fn is_remote_exists(&self) -> Result<bool, Error>;
    fn get_current_branch(&self) -> Result<String, Error>;
    fn pull_fast_forwarded(&self, remote: &str, branch: &str) -> Result<(), Error>;
    fn switch(&self, branch: &str) -> Result<(), Error>;
    fn delete_branch(&self, branch: &str) -> Result<(), Error>;
    fn command(&self, args: &[&str]) -> Result<Output, Error>;
    fn git_command_on_dir(args: &[&str], workdir: &Path) -> Result<Output, Error>;
}

impl RepositorySupport for Repository {
    fn merged_branches(&self) -> Result<Vec<String>, Error> {
        // git for-each-ref --format=%(refname:short) refs/heads/**/* --merged
        let result = self.command(&[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/**/*",
            "--merged",
        ])?;
        let message = String::from_utf8(result.stdout)?;
        Ok(split_lines(message))
    }
    fn is_clean(&self) -> Result<bool, Error> {
        Ok(!self.has_unsaved()?)
    }
    fn clone(url: &str, into: &Path) -> Result<(), Error> {
        let result = Repository::git_command_on_dir(&["clone", url], into)?;
        if !result.status.success() {
            let error = String::from_utf8(result.stderr)?;
            return Err(Error::from_str(&error));
        }
        Ok(())
    }
    fn has_unsaved(&self) -> Result<bool, Error> {
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
    fn is_remote_exists(&self) -> Result<bool, Error> {
        Ok(!self.remotes()?.is_empty())
    }
    fn get_current_branch(&self) -> Result<String, Error> {
        if self.is_empty()? {
            return Err(Error::from_str("repository is empty"));
        }
        let head = self.head()?;

        let Some(name) = head.shorthand() else {
            return Err(Error::from_str("head is not a branch"));
        };
        let branch = self.find_branch(name, BranchType::Local)?;
        let Some(branch_name) = branch.name()? else {
            return Err(Error::from_str("branch name is not found"));
        };
        Ok(branch_name.to_string())
    }
    fn pull_fast_forwarded(&self, remote: &str, branch: &str) -> Result<(), Error> {
        let output = self.command(&["pull", "--ff-only", remote, branch])?;
        if !output.status.success() {
            let message = String::from_utf8(output.stderr)?;
            return Err(Error::from_str(&format!(
                "failed to pull fast forward: {}",
                message
            )));
        }
        Ok(())
    }
    fn switch(&self, branch: &str) -> Result<(), Error> {
        let output = self.command(&["switch", branch])?;
        if !output.status.success() {
            let message = String::from_utf8(output.stderr)?;
            return Err(Error::from_str(&format!(
                "failed to switch to branch {}: {}",
                branch, message
            )));
        }
        Ok(())
    }
    fn delete_branch(&self, branch: &str) -> Result<(), Error> {
        let output = self.command(&["branch", "-d", branch])?;
        if !output.status.success() {
            let message = String::from_utf8(output.stderr)?;
            return Err(Error::from_str(&format!(
                "failed to delete branch {}: {}",
                branch, message
            )));
        }
        Ok(())
    }

    fn git_command_on_dir(args: &[&str], workdir: &Path) -> Result<Output, Error> {
        Ok(Command::new("git")
            .current_dir(workdir)
            .args(args)
            .output()?)
    }

    fn command(&self, args: &[&str]) -> Result<Output, Error> {
        let Some(workdir) = self.workdir() else {
            return Err(Error::from_str("parent dir exist"));
        };
        Self::git_command_on_dir(args, workdir)
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Error {
        Error::from_str(&e.to_string())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Error {
        Error::from_str(&e.to_string())
    }
}

fn split_lines(lines: String) -> Vec<String> {
    lines
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
        let lines = "a\nb\nc\n".to_string();
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
        let Ok(merged_branches) = repo.merged_branches() else {
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
        let result = repo2.pull_fast_forwarded("origin", "main");
        assert!(result.is_err());
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
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().message(),
            "failed to delete branch feature: error: branch 'feature' not found.\n"
        );
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
        let Err(error) = result else {
            unreachable!();
        };
        assert_eq!(error.message(), "fatal: repository '' does not exist\n");
    }
}
