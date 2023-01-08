use std::io::Write;

use git2::Repository;
use mktemp::Temp;

use crate::{
    git::{GitCommandOutput, RepositorySupport},
    mure_error::Error,
};

#[cfg(test)]
pub struct Fixture {
    pub repo: Repository,
    _temp_dir: Temp,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
impl Fixture {
    /// Create temporary repository.
    /// When the test is finished, the temporary directory is removed.
    pub fn create() -> Result<Fixture, Error> {
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
    pub fn set_dummy_profile(&self) -> Result<(), Error> {
        // git config user.name "test"
        // git config user.email "test@example.com"
        let repo = &self.repo;
        repo.config()?.set_str("user.name", "tester")?;
        repo.config()?.set_str("user.email", "test@example.com")?;
        Ok(())
    }
    /// Create empty commit.
    pub fn create_empty_commit(
        &self,
        message: &str,
    ) -> Result<GitCommandOutput<()>, crate::git::Error> {
        self.repo
            .command(&["commit", "--allow-empty", "-m", message])?
            .interpret_to(())
    }

    pub fn create_file(&self, filename: &str, content: &str) -> Result<(), Error> {
        let Some(workdir) = self.repo.workdir() else {
                return Err(Error::from_str("workdir not found"));
            };
        let filepath = workdir.join(filename);
        let mut file = std::fs::File::create(filepath)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}
