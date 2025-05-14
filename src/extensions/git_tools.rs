use std::path::{Path, PathBuf};

use anyhow::*;
use log::*;

pub struct GitTools {
    repository: git2::Repository,
    workdir_path: PathBuf,
}

impl GitTools {
    /// Construct from the `Path` to a file in the repository.
    /// The `path` can be a path to a subdirectory inside the working directory
    /// of the repository.
    /// See <https://libgit2.org/docs/reference/main/repository/git_repository_open_ext.html>.
    pub fn from_file_path(path: &Path) -> anyhow::Result<Self> {
        let repository = git2::Repository::open_ext(
            path,
            git2::RepositoryOpenFlags::empty(),
            &[] as &[&std::ffi::OsStr],
        )?;
        Self::from_repository(repository)
    }

    /// The 'path' argument must point to either a git repository folder, or an
    /// existing work dir.
    /// See <https://libgit2.org/docs/reference/main/repository/git_repository_open.html>.
    pub fn from_repository_path(repository_path: &Path) -> anyhow::Result<Self> {
        let repository = git2::Repository::open(repository_path)?;
        Self::from_repository(repository)
    }

    fn from_repository(repository: git2::Repository) -> anyhow::Result<Self> {
        let git_path = repository.path().canonicalize()?;
        let workdir_path = git_path.parent().unwrap();
        Ok(Self {
            repository,
            workdir_path: workdir_path.to_path_buf(),
        })
    }

    /// Get `git2::Repository`.
    pub fn repository(&self) -> &git2::Repository {
        &self.repository
    }

    /// Get the repository path; i.e., the `.git` directory.
    pub fn repository_path(&self) -> &Path {
        self.repository.path()
    }

    /// Get the canonicalized root directory of the worktree .
    pub(crate) fn workdir_path(&self) -> &Path {
        &self.workdir_path
    }

    pub fn path_in_workdir(&self, path: &Path) -> anyhow::Result<PathBuf> {
        let path = path.canonicalize()?;
        let path = path.strip_prefix(self.workdir_path())?;
        Ok(Self::to_posix_path(path))
    }

    #[cfg(target_os = "windows")]
    fn to_posix_path(path: &Path) -> PathBuf {
        assert!(path.is_relative());
        let path_str = path.to_string_lossy().replace('\\', "/");
        PathBuf::from(path_str)
    }

    #[cfg(not(target_os = "windows"))]
    fn to_posix_path(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    pub fn head_commit_id(&self) -> anyhow::Result<git2::Oid> {
        let head = self.repository.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id())
    }

    /// Get the content of a `path` at the tree of the `commit_id` as a string.
    /// If `commit_id` is zero, the `head` is used.
    pub fn content_as_string(&self, commit_id: git2::Oid, path: &Path) -> anyhow::Result<String> {
        debug!("content_as_string: {commit_id} {path:?}");
        let commit = if commit_id.is_zero() {
            self.repository.head()?.peel_to_commit()?
        } else {
            self.repository.find_commit(commit_id)?
        };
        let tree = commit.tree()?;
        trace!("content_as_string: tree={}", tree.id());
        let entry = tree.get_path(path)?;
        let object = entry.to_object(&self.repository)?;
        // https://github.com/rust-lang/git2-rs/issues/1156
        let blob = object.into_blob().unwrap();
        Ok(std::str::from_utf8(blob.content())?.to_string())
    }

    pub fn show(&self, commit_id: git2::Oid, paths: &[&Path]) -> anyhow::Result<()> {
        debug!("git-show: {commit_id} {paths:?}");
        let mut command = self.create_show_command(commit_id);
        if !paths.is_empty() {
            command.arg("--");
            for path in paths {
                command.arg(path);
            }
        }
        let mut child = command.spawn()?;
        child.wait()?;
        Ok(())
    }

    pub fn create_show_command(&self, commit_id: git2::Oid) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        command
            .current_dir(self.repository_path())
            .arg("show")
            .arg(commit_id.to_string());
        command
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
            let repository = git2::Repository::init(dir.path())?;
            let mut config = repository.config()?;
            config.set_str("user.name", "Test User")?;
            config.set_str("user.email", "test@test.com")?;
            Ok(Self {
                git: GitTools::from_repository(repository)?,
                _temp_dir: dir,
            })
        }

        pub fn repository(&self) -> &git2::Repository {
            self.git.repository()
        }

        pub fn worktree_path(&self) -> &Path {
            self.git.workdir_path()
        }

        pub fn to_file_path(&self, path: &Path) -> PathBuf {
            assert!(path.is_relative());
            self.worktree_path().join(path)
        }

        pub fn add_file_content(&self, path: &Path, content: &str) -> anyhow::Result<()> {
            let file_path = self.to_file_path(path);
            std::fs::create_dir_all(file_path.parent().unwrap())?;
            std::fs::write(&file_path, content)?;

            let mut index = self.repository().index()?;
            index.add_path(path)?;
            index.write()?;
            Ok(())
        }

        pub fn rename_file(&self, old_path: &Path, new_path: &Path) -> anyhow::Result<()> {
            let old_file_path = self.to_file_path(old_path);
            let new_file_path = self.to_file_path(new_path);
            std::fs::create_dir_all(new_file_path.parent().unwrap())?;
            std::fs::rename(old_file_path, new_file_path)?;

            let mut index = self.repository().index()?;
            index.remove_path(old_path)?;
            index.add_path(new_path)?;
            index.write()?;
            Ok(())
        }

        pub fn commit(
            &self,
            parent_commit_id: git2::Oid,
            message: &str,
        ) -> anyhow::Result<git2::Oid> {
            let mut index = self.repository().index()?;
            let signature = self.git.repository.signature()?;
            let tree_id = index.write_tree()?;
            let tree = self.git.repository.find_tree(tree_id)?;
            let commit_id = if parent_commit_id.is_zero() {
                self.git.repository.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    message,
                    &tree,
                    &[],
                )?
            } else {
                let parent_commit = self.repository().find_commit(parent_commit_id)?;
                self.git.repository.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    message,
                    &tree,
                    &[&parent_commit],
                )?
            };
            assert_eq!(
                commit_id,
                self.git.repository.head()?.peel_to_commit()?.id()
            );
            Ok(commit_id)
        }
    }

    #[test]
    fn content_as_string() -> anyhow::Result<()> {
        let git = TempRepository::new()?;
        let path = PathBuf::from("test.txt");
        let content = "Hello, world!";
        git.add_file_content(&path, content)?;
        git.commit(git2::Oid::zero(), "Add file")?;
        assert_eq!(
            git.git.content_as_string(git2::Oid::zero(), &path)?,
            content
        );
        Ok(())
    }
}
