use std::process::Command;

/// Wrapper of git2 and git commands.
use git2::{BranchType, Repository};

use crate::mure_error::Error;

pub trait RepositoryWrapper {
    fn merged_branches(&self) -> Result<Vec<String>, Error>;
    fn is_clean(&self) -> Result<bool, Error>;
    fn has_unsaved(&self) -> Result<bool, Error>;
    fn is_remote_exists(&self) -> Result<bool, Error>;
    fn get_current_branch(&self) -> Result<String, Error>;
}

impl RepositoryWrapper for Repository {
    fn merged_branches(&self) -> Result<Vec<String>, Error> {
        let mut branches = Vec::new();
        // git for-each-ref --format='%(refname:short)' 'refs/heads/**/*' --merged
        let result = Command::new("git")
            .arg("for-each-ref")
            .arg("--format=%(refname:short)")
            .arg("refs/heads/**/*")
            .arg("--merged")
            .current_dir(self.path().to_str().unwrap())
            .output()?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use mktemp::Temp;
    use std::{io::Write, process::Command};

    #[test]
    fn merged_branches() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");
        let repo = Repository::init(path).unwrap();
        // git remote add origin
        let example_repo_url = "https://github.com/kitsuyui/kitsuyui.git";
        repo.remote_set_url("origin", example_repo_url)
            .expect("failed to set remote url");
        repo.config()
            .unwrap()
            .set_str("user.name", "tester")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();

        // create a first branch
        let main_branch = "main";
        Command::new("git")
            .current_dir(path)
            .arg("switch")
            .arg("-c")
            .arg(main_branch)
            .output()
            .expect("failed to switch branch");

        // create a new branch for testing
        let branch_name = "test_branch";
        // git switch -c $branch_name
        Command::new("git")
            .current_dir(path)
            .arg("switch")
            .arg("-c")
            .arg(branch_name)
            .output()
            .expect("failed to switch branch");

        // git commit --allow-empty -m "initial commit"
        let sig = repo.signature().unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        // switch to default branch
        Command::new("git")
            .current_dir(path)
            .arg("switch")
            .arg(main_branch)
            .output()
            .expect("failed to switch branch");

        // git merge $branch_name
        Command::new("git")
            .current_dir(path)
            .arg("merge")
            .arg(branch_name)
            .output()
            .expect("failed to merge branch");

        // now test_branch is same as default branch so it should be merged
        match repo.merged_branches() {
            Ok(branches) => {
                assert!(branches.contains(&branch_name.to_string()));
            }
            Err(e) => {
                panic!("failed to get merged branches: {}", e);
            }
        }
    }

    #[test]
    fn test_is_empty() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");
        let repo = Repository::init(path).unwrap();

        // git config
        repo.config()
            .unwrap()
            .set_str("user.name", "tester")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();
        assert!(repo.is_empty().unwrap(), "repo is empty when initialized");

        // git commit --allow-empty -m "initial commit"
        let sig = repo.signature().unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        assert!(!repo.is_empty().unwrap(), "repo is not empty after commit");
    }

    #[test]
    fn test_is_remote_exists() {
        // cd $(mktemp -d)
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");

        // git init
        let repo = Repository::init(path).unwrap();

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
    fn test_exists_unsaved() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");
        let repo = Repository::init(path).unwrap();

        // git config
        repo.config()
            .unwrap()
            .set_str("user.name", "tester")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();

        assert!(repo.is_clean().unwrap(), "repo is clean when initialized");
        assert!(
            !repo.has_unsaved().unwrap(),
            "repo is clean when initialized"
        );

        // write file
        let filepath = temp_dir.join("something.txt");
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
        Command::new("git")
            .args(&["switch", "-c", "feature"])
            .current_dir(path)
            .output()
            .unwrap();

        // write file
        let filepath = temp_dir.join("something2.txt");
        let mut file = std::fs::File::create(filepath).unwrap();
        file.write_all("hello".as_bytes()).unwrap();
        file.sync_all().unwrap();

        assert!(!repo.is_clean().unwrap(), "unstaged file is dirty");
        assert!(repo.has_unsaved().unwrap(), "unstaged file is dirty");

        Command::new("git")
            .args(&["add", "something2.txt"])
            .current_dir(path)
            .output()
            .unwrap();

        assert!(repo.is_clean().unwrap(), "repo is clean after commit");
        assert!(!repo.has_unsaved().unwrap(), "repo is clean after commit");
    }

    #[test]
    fn test_get_current_branch() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let path = temp_dir
            .as_path()
            .as_os_str()
            .to_str()
            .expect("failed to get path");
        let repo = Repository::init(path).unwrap();

        // git config
        repo.config()
            .unwrap()
            .set_str("user.name", "tester")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();

        if let Ok(it) = repo.get_current_branch() {
            unreachable!("unreachable! {}", it);
        }

        // git commit --allow-empty -m "initial commit"
        let sig = repo.signature().unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        match repo.get_current_branch() {
            Ok(it) => match it.as_str() {
                "master" => {}
                "main" => {}
                _ => unreachable!("something went wrong! {}", it),
            },
            Err(it) => unreachable!("something went wrong!! {}", it),
        }
    }
}
