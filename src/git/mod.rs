/// Wrapper of git2 and git commands.
use git2::{BranchType, Error, Repository};

pub trait RepositoryWrapper {
    fn is_clean(&self) -> Result<bool, Error>;
    fn exists_unsaved(&self) -> Result<bool, Error>;
    fn is_remote_exists(&self) -> Result<bool, Error>;
    fn get_current_branch(&self) -> Result<String, Error>;
}

impl RepositoryWrapper for Repository {
    fn exists_unsaved(&self) -> Result<bool, Error> {
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
        Ok(!self.exists_unsaved()?)
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
    use std::io::Write;

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

        assert!(
            !repo.exists_unsaved().unwrap(),
            "repo is clean when initialized"
        );

        // write file
        let filepath = temp_dir.join("something.txt");
        let mut file = std::fs::File::create(filepath).unwrap();
        file.write_all("hello".as_bytes()).unwrap();
        file.sync_all().unwrap();

        assert!(
            repo.exists_unsaved().unwrap(),
            "repo is dirty because of file"
        );

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

        assert!(
            !repo.exists_unsaved().unwrap(),
            "repo is clean after commit"
        );
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
