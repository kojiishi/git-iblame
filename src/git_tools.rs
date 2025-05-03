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

    #[cfg(test)]
    fn from_repository(repository: Repository) -> anyhow::Result<Self> {
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

    pub fn show(&self, commit_id: Oid) -> anyhow::Result<()> {
        let mut command = std::process::Command::new("git");
        command.current_dir(self.root_path());
        command.arg("show").arg(commit_id.to_string());
        let mut child = command.spawn()?;
        child.wait()?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    use super::*;

    #[cfg(test)]
    pub(crate) struct TempRepository {
        pub git: GitTools,
        _temp_dir: tempfile::TempDir,
    }

    #[cfg(test)]
    impl TempRepository {
        pub fn new() -> anyhow::Result<Self> {
            let dir = tempfile::TempDir::new()?;
            let repository = Repository::init(dir.path())?;
            let mut config = repository.config()?;
            config.set_str("user.name", "Test User")?;
            config.set_str("user.email", "test@test.com")?;
            Ok(Self {
                git: GitTools::from_repository(repository)?,
                _temp_dir: dir,
            })
        }

        pub fn repository(&self) -> &Repository {
            self.git.repository()
        }

        pub fn root_path(&self) -> &Path {
            self.git.root_path()
        }

        pub fn commit_file(&self, path: &Path, content: &str) -> anyhow::Result<()> {
            let file_path = self.root_path().join(path);
            std::fs::create_dir_all(file_path.parent().unwrap())?;
            std::fs::write(&file_path, content)?;

            let mut index = self.repository().index()?;
            index.add_path(path)?;
            index.write()?;

            let mut signature = self.git.repository.signature()?;
            let mut signature2 = self.git.repository.signature()?;
            let tree_id = index.write_tree()?;
            let tree = self.git.repository.find_tree(tree_id)?;
            let commit_id = self.git.repository.commit(
                Some("HEAD"),
                &mut signature,
                &mut signature2,
                "Add file",
                &tree,
                &[],
            )?;
            assert_eq!(
                commit_id,
                self.git.repository.head()?.peel_to_commit()?.id()
            );
            Ok(())
        }
    }

    #[test]
    fn content_as_string() -> anyhow::Result<()> {
        let git = TempRepository::new()?;
        let path = PathBuf::from("test.txt");
        let content = "Hello, world!";
        git.commit_file(&path, content)?;
        assert_eq!(git.git.content_as_string(Oid::zero(), &path)?, content);
        Ok(())
    }
}
