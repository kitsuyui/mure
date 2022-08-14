use std::process::{Command, Output};

/// Wrapper of git2 and git commands.
use git2::{BranchType, Repository};

use crate::mure_error::Error;

pub trait RepositorySupport {
    fn merged_branches(&self) -> Result<Vec<String>, Error>;
    fn is_clean(&self) -> Result<bool, Error>;
    fn has_unsaved(&self) -> Result<bool, Error>;
    fn is_remote_exists(&self) -> Result<bool, Error>;
    fn get_current_branch(&self) -> Result<String, Error>;
    fn pull_fast_forwarded(&self, remote: &str, branch: &str) -> Result<Output, Error>;
    fn switch(&self, branch: &str) -> Result<Output, Error>;
    fn delete_branch(&self, branch: &str) -> Result<Output, Error>;
    fn command(&self, args: &[&str]) -> Result<Output, Error>;
}

impl RepositorySupport for Repository {
    fn merged_branches(&self) -> Result<Vec<String>, Error> {
        let mut branches = Vec::new();
        // git for-each-ref --format=%(refname:short) refs/heads/**/* --merged
        let result = self.command(&[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/**/*",
            "--merged",
        ])?;
        let stdout = String::from_utf8(result.stdout).unwrap();
        for line in stdout.split('\n') {
            if !line.is_empty() {
                branches.push(line.to_string());
            }
        }
        Ok(branches)
    }
    fn is_clean(&self) -> Result<bool, Error> {
        Ok(!self.has_unsaved()?)
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
        let head_name = head.shorthand().unwrap();
        match self.find_branch(head_name, BranchType::Local)?.name()? {
            Some(branch_name) => Ok(branch_name.to_owned()),
            None => unreachable!("unreachable!"),
        }
    }
    fn pull_fast_forwarded(&self, remote: &str, branch: &str) -> Result<Output, Error> {
        let output = self.command(&["pull", "--ff-only", remote, branch, branch])?;
        if !output.status.success() {
            return Err(Error::from_str(&format!(
                "failed to pull fast forward: {}",
                String::from_utf8(output.stderr).unwrap()
            )));
        }
        Ok(output)
    }
    fn switch(&self, branch: &str) -> Result<Output, Error> {
        let output = self.command(&["switch", branch])?;
        if !output.status.success() {
            return Err(Error::from_str(&format!(
                "failed to switch to branch {}: {}",
                branch,
                String::from_utf8(output.stderr).unwrap()
            )));
        }
        Ok(output)
    }
    fn delete_branch(&self, branch: &str) -> Result<Output, Error> {
        let output = self.command(&["branch", "-d", branch])?;
        if !output.status.success() {
            return Err(Error::from_str(&format!(
                "failed to delete branch {}: {}",
                branch,
                String::from_utf8(output.stderr).unwrap()
            )));
        }
        Ok(output)
    }
    fn command(&self, args: &[&str]) -> Result<Output, Error> {
        Ok(Command::new("git")
            .current_dir(self.workdir().expect("parent dir exists"))
            .args(args)
            .output()?)
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Error {
        Error::from_str(&e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mktemp::Temp;
    use std::io::Write;

    struct Fixture {
        repo: Repository,
        _temp_dir: Temp,
    }

    impl Fixture {
        /// Create temporary repository.
        /// When the test is finished, the temporary directory is removed.
        fn create() -> Result<Fixture, Error> {
            let temp_dir = Temp::new_dir().expect("failed to create temp dir");
            let path = temp_dir
                .as_path()
                .as_os_str()
                .to_str()
                .expect("failed to get path");
            let repo = Repository::init(path)?;
            let me = Fixture {
                repo,
                _temp_dir: temp_dir,
            };
            me.set_dummy_profile()?;
            Ok(me)
        }
        /// Set dummy profile.
        fn set_dummy_profile(&self) -> Result<(), Error> {
            // git config user.name "test"
            // git config user.email "test@example.com"
            let repo = &self.repo;
            repo.config()?.set_str("user.name", "tester")?;
            repo.config()?.set_str("user.email", "test@example.com")?;
            Ok(())
        }
        /// Create empty commit.
        fn create_empty_commit(&self, message: &str) -> Result<(), Error> {
            self.repo
                .command(&["commit", "--allow-empty", "-m", message])?;
            Ok(())
        }

        fn create_file(&self, filename: &str, content: &str) -> Result<(), Error> {
            let filepath = self.repo.workdir().unwrap().join(filename);
            let mut file = std::fs::File::create(filepath)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            Ok(())
        }
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
        match repo.merged_branches() {
            Ok(branches) => {
                assert!(branches.contains(&branch_name.to_string()));
            }
            Err(e) => {
                unreachable!("failed to get merged branches: {}", e);
            }
        }
    }

    #[test]
    fn test_is_empty() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        assert!(repo.is_empty().unwrap(), "repo is empty when initialized");

        fixture.create_empty_commit("initial commit").unwrap();

        assert!(!repo.is_empty().unwrap(), "repo is not empty after commit");
    }

    #[test]
    fn test_is_remote_exists() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        assert!(
            !repo.is_remote_exists().unwrap(),
            "remote is not exists when initialized"
        );

        // git remote add origin
        let example_repo_url = "https://github.com/kitsuyui/kitsuyui.git";
        repo.remote_set_url("origin", example_repo_url)
            .expect("failed to set remote url");

        assert!(
            repo.is_remote_exists()
                .expect("failed to check remote exists"),
            "now remote must be set"
        );
    }

    #[test]
    fn test_has_unsaved_and_is_clean() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        assert!(
            repo.is_clean().unwrap() && !repo.has_unsaved().unwrap(),
            "repo is clean when initialized"
        );

        fixture.create_file("1.txt", "hello").unwrap();

        assert!(
            !repo.is_clean().unwrap() && repo.has_unsaved().unwrap(),
            "repo is dirty because of file"
        );

        repo.command(&["add", "1.txt"])
            .expect("failed to add 1.txt");

        assert!(
            !repo.is_clean().unwrap() && repo.has_unsaved().unwrap(),
            "staged but not committed file is dirty"
        );

        repo.command(&["commit", "-m", "add 1.txt"])
            .expect("failed to commit");

        assert!(
            repo.is_clean().unwrap() && !repo.has_unsaved().unwrap(),
            "repo is clean after commit"
        );

        repo.command(&["switch", "-c", "feature"])
            .expect("failed to switch to feature branch");

        fixture.create_file("2.txt", "hello").unwrap();

        assert!(
            !repo.is_clean().unwrap() && repo.has_unsaved().unwrap(),
            "unstaged file is dirty"
        );

        repo.command(&["add", "2.txt"])
            .expect("failed to add 2.txt");

        assert!(
            !repo.is_clean().unwrap() && repo.has_unsaved().unwrap(),
            "staged but not committed file is dirty"
        );

        repo.command(&["commit", "-m", "add 2.txt"])
            .expect("failed to commit");

        assert!(
            repo.is_clean().unwrap() && !repo.has_unsaved().unwrap(),
            "repo is clean after commit"
        );
    }

    #[test]
    fn test_get_current_branch() {
        let fixture = Fixture::create().unwrap();
        let repo = &fixture.repo;

        if let Ok(it) = repo.get_current_branch() {
            panic!("current branch will be empty: {}", it);
        }

        fixture.create_empty_commit("initial commit").unwrap();

        match repo.get_current_branch() {
            Ok(it) => match it.as_str() {
                "master" => {}
                "main" => {}
                _ => panic!("something went wrong! {}", it),
            },
            Err(it) => panic!("something went wrong!! {}", it),
        }
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
        assert_eq!(
            count_before - count_after,
            1,
            "feature branch must be deleted"
        );
    }
}
