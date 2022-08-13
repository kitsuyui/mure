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
    fn has_unsaved(&self) -> Result<bool, Error> {
        for entry in self.statuses(None)?.iter() {
            match entry.status() {
                git2::Status::CURRENT => continue,
                git2::Status::WT_NEW | git2::Status::WT_MODIFIED | git2::Status::WT_DELETED => {
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }
    fn is_clean(&self) -> Result<bool, Error> {
        Ok(!self.has_unsaved()?)
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
    fn command(&self, args: &[&str]) -> Result<Output, Error> {
        Ok(Command::new("git")
            .current_dir(self.workdir().expect("parent dir exists"))
            .args(args)
            .output()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mktemp::Temp;
    use std::io::Write;

    struct TestRepository {
        repo: Repository,
        _temp_dir: Temp,
    }
    trait RepositoryTestFixture {
        fn create() -> Result<TestRepository, Error>;
        fn set_dummy_profile(&self) -> Result<(), Error>;
        fn create_empty_commit(&self, message: &str) -> Result<(), Error>;
    }

    impl RepositoryTestFixture for TestRepository {
        /// Create temporary repository.
        /// When the test is finished, the temporary directory is removed.
        fn create() -> Result<TestRepository, Error> {
            let temp_dir = Temp::new_dir().expect("failed to create temp dir");
            let path = temp_dir
                .as_path()
                .as_os_str()
                .to_str()
                .expect("failed to get path");
            let repo = Repository::init(path).unwrap();
            let me = TestRepository {
                repo,
                _temp_dir: temp_dir,
            };
            me.set_dummy_profile()?;
            Ok(me)
        }
        /// Set dummy profile.
        fn set_dummy_profile(&self) -> Result<(), Error> {
            let repo = &self.repo;
            // git config user.name "test"
            repo.config()
                .unwrap()
                .set_str("user.name", "tester")
                .unwrap();
            // git config user.email "test@example.com"
            repo.config()
                .unwrap()
                .set_str("user.email", "test@example.com")
                .unwrap();
            Ok(())
        }
        /// Create empty commit.
        fn create_empty_commit(&self, message: &str) -> Result<(), Error> {
            let repo = &self.repo;
            // git commit --allow-empty -m "initial commit"
            let sig = repo.signature().unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap();
            Ok(())
        }
    }

    #[test]
    fn merged_branches() {
        let fixture = TestRepository::create().unwrap();
        let repo = &fixture.repo;

        // git remote add origin
        let example_repo_url = "https://github.com/kitsuyui/kitsuyui.git";
        repo.remote_set_url("origin", example_repo_url)
            .expect("failed to set remote url");

        // create a first branch
        let main_branch = "main";
        repo.command(&["switch", "-c", main_branch])
            .expect("failed to switch to main branch");

        // create a new branch for testing
        let branch_name = "test";
        // git switch -c $branch_name
        repo.command(&["switch", "-c", branch_name])
            .expect("failed to switch to test branch");

        // git commit --allow-empty -m "initial commit"
        fixture.create_empty_commit("initial commit").unwrap();

        // switch to default branch
        repo.command(&["switch", main_branch])
            .expect("failed to switch to main branch");

        // git merge $branch_name
        repo.command(&["merge", branch_name])
            .expect("failed to merge test branch");

        // now test_branch is same as default branch so it should be merged
        match repo.merged_branches() {
            Ok(branches) => {
                println!("{:?}", branches);
                println!("{:?}", vec![main_branch, branch_name]);
                assert!(branches.contains(&branch_name.to_string()));
            }
            Err(e) => {
                unreachable!("failed to get merged branches: {}", e);
            }
        }
    }

    #[test]
    fn test_is_empty() {
        let fixture = TestRepository::create().unwrap();
        let repo = &fixture.repo;

        assert!(repo.is_empty().unwrap(), "repo is empty when initialized");

        // git commit --allow-empty -m "initial commit"
        fixture.create_empty_commit("initial commit").unwrap();

        assert!(!repo.is_empty().unwrap(), "repo is not empty after commit");
    }

    #[test]
    fn test_is_remote_exists() {
        let fixture = TestRepository::create().unwrap();
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
        let fixture = TestRepository::create().unwrap();
        let repo = &fixture.repo;

        assert!(repo.is_clean().unwrap(), "repo is clean when initialized");
        assert!(
            !repo.has_unsaved().unwrap(),
            "repo is clean when initialized"
        );

        // write file
        let filepath = repo.workdir().unwrap().join("something.txt");
        let mut file = std::fs::File::create(filepath).unwrap();
        file.write_all("hello".as_bytes()).unwrap();
        file.sync_all().unwrap();

        assert!(!repo.is_clean().unwrap(), "repo is dirty because of file");
        assert!(repo.has_unsaved().unwrap(), "repo is dirty because of file");

        // git commit -m "initial commit"
        let sig = repo.signature().unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index
                .add_all(&["something.txt"], git2::IndexAddOption::DEFAULT, None)
                .unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        assert!(repo.is_clean().unwrap(), "repo is clean after commit");
        assert!(!repo.has_unsaved().unwrap(), "repo is clean after commit");

        // git checkout -b feature
        repo.command(&["switch", "-c", "feature"])
            .expect("failed to switch to feature branch");

        // write file
        let filepath = repo.workdir().unwrap().join("something2.txt");
        let mut file = std::fs::File::create(filepath).unwrap();
        file.write_all("hello".as_bytes()).unwrap();
        file.sync_all().unwrap();

        assert!(!repo.is_clean().unwrap(), "unstaged file is dirty");
        assert!(repo.has_unsaved().unwrap(), "unstaged file is dirty");

        // git add something2.txt
        repo.command(&["add", "something2.txt"])
            .expect("failed to add something2.txt");

        assert!(repo.is_clean().unwrap(), "repo is clean after commit");
        assert!(!repo.has_unsaved().unwrap(), "repo is clean after commit");
    }

    #[test]
    fn test_get_current_branch() {
        let fixture = TestRepository::create().unwrap();
        let repo = &fixture.repo;

        if let Ok(it) = repo.get_current_branch() {
            panic!("current branch will be empty: {}", it);
        }

        // git commit --allow-empty -m "initial commit"
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
}
