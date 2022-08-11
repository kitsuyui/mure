use git2::{Error, Repository};

use crate::git::RepositoryWrapper;

pub fn refresh() -> Result<(), Error> {
    // TODO: set path from config or current directory
    let repo = Repository::open(".")?;

    if repo.is_empty()? {
        // nothing to do if repo is empty
        return Ok(());
    }
    if !repo.is_remote_exists()? {
        // nothing to do if repo has no remote
        return Ok(());
    }
    if repo.exists_unsaved()? {
        // nothing to do if repo has unsaved changes
        return Ok(());
    }

    // TODO:

    // get default branch: gh repo view --json defaultBranchRef --jq '.defaultBranchRef.name'
    // let current_branch = repo.get_current_branch()?;
    // if current branch is not default branch, checkout default branch and pull

    Ok(())
}
