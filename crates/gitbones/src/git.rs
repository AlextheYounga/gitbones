use anyhow::{bail, Context, Result};
use git2::Repository;

pub fn open_repo() -> Result<Repository> {
    Repository::open(".").context("Not a git repository")
}


pub fn validate_remote_exists(repo: &Repository, remote_name: &str) -> Result<()> {
    let remotes = repo.remotes().context("Failed to list remotes")?;
    let exists = remotes.iter().any(|r| r == Some(remote_name));
    if !exists {
        bail!(
            "No git remote '{remote_name}' found. \
             Please set one up before running gitbones init:\n  \
             git remote add {remote_name} git@<host>:<repo>.git"
        );
    }
    Ok(())
}
