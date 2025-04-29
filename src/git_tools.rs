use std::path::{Path, PathBuf};

use anyhow::*;
use git2::{Oid, Repository, RepositoryOpenFlags};

pub struct GitTools {
    repository: Repository,
    root_path: PathBuf,
}

impl GitTools {
    /// Construct from the `Path` to a file in the repository.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let repository = Repository::open_ext(
            &path,
            RepositoryOpenFlags::empty(),
            &[] as &[&std::ffi::OsStr],
        )?;

        let git_path = repository.path().canonicalize()?;
        let root_path = git_path.parent().unwrap();

        Ok(Self {
            repository: repository,
            root_path: root_path.to_path_buf(),
        })
    }

    /// Get `git2::Repository`.
    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    /// Get the canonicalized root directory.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Get the content of a `path` at the tree of the `commit_id` as a string.
    /// If `commit_id` is zero, the `head` is used.
    pub fn content_as_string(&self, commit_id: Oid, path: &Path) -> anyhow::Result<String> {
        let commit = if commit_id.is_zero() {
            self.repository.head()?.peel_to_commit()?
        } else {
            self.repository.find_commit(commit_id)?
        };
        let tree = commit.tree()?;
        let entry = tree.get_path(path)?;
        let object = entry.to_object(&self.repository)?;
        // https://github.com/rust-lang/git2-rs/issues/1156
        let blob = object.into_blob().unwrap();
        Ok(std::str::from_utf8(blob.content())?.to_string())
    }

    /// Get the commit id of one older commit of `commit_id`.
    pub fn older_commit_id(&self, commit_id: Oid) -> Result<Option<Oid>, git2::Error> {
        let mut revwalk = self.repository.revwalk()?;
        revwalk.push(commit_id)?;
        let first_id = revwalk.next().unwrap()?;
        assert_eq!(commit_id, first_id);
        revwalk.next().transpose()
    }
}
