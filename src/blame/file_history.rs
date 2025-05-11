use std::{
    cmp,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use log::*;

use crate::extensions::GitTools;

use super::{CommitIterator, DiffPart, FileCommit, FileCommits, FileContent, LineNumberMap};

pub struct FileHistory {
    path: PathBuf,
    git: Option<GitTools>,
    commits: FileCommits,
    read_thread: Option<thread::JoinHandle<anyhow::Result<()>>>,
    rx: Option<mpsc::Receiver<FileCommit>>,
}

impl FileHistory {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            git: None,
            commits: FileCommits::new(),
            read_thread: None,
            rx: None,
        }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        let result = Self::new(Path::new(""));
        assert!(result.is_path_empty());
        result
    }

    fn is_path_empty(&self) -> bool {
        if let Some(path_str) = self.path.to_str() {
            return path_str.is_empty();
        }
        false
    }

    pub fn git(&self) -> &GitTools {
        self.git.as_ref().unwrap()
    }

    fn ensure_git(&mut self) -> anyhow::Result<()> {
        if self.git.is_none() {
            let git = GitTools::from_file_path(&self.path)?;
            self.path = git.path_in_workdir(&self.path)?;
            self.git = Some(git);
        }
        Ok(())
    }

    /// Returns a reference to the underlying `FileCommits` collection.
    ///
    /// Consumers can use methods like `iter()`, `get_by_id()`, or index directly.
    pub fn commits(&self) -> &FileCommits {
        &self.commits
    }

    pub fn commit(&self, index: usize) -> &FileCommit {
        &self.commits[index]
    }

    pub fn map_line_number_by_commit_ids(
        &self,
        line_number: usize,
        new_commit_id: git2::Oid,
        old_commit_id: git2::Oid,
    ) -> anyhow::Result<usize> {
        assert!(old_commit_id != new_commit_id);
        let current_index = self.commits.index_from_commit_id(old_commit_id)?;
        let new_index = self.commits.index_from_commit_id(new_commit_id)?;
        Ok(self.map_line_number_by_commit_indexes(line_number, new_index, current_index))
    }

    pub fn map_line_number_by_commit_indexes(
        &self,
        line_number: usize,
        new_index: usize,
        old_index: usize,
    ) -> usize {
        let new_line_number = match new_index.cmp(&old_index) {
            cmp::Ordering::Less => self.map_line_number_by_commit_index_iterator(
                line_number,
                (new_index..old_index).rev(),
                LineNumberMap::new_new_from_old,
            ),
            cmp::Ordering::Greater => self.map_line_number_by_commit_index_iterator(
                line_number,
                old_index..new_index,
                LineNumberMap::new_old_from_new,
            ),
            cmp::Ordering::Equal => unreachable!("new and current should not be equal"),
        };
        debug!(
            "map_line_number_by_indexes: {line_number}@{old_index} \
            -> {new_line_number}@{new_index}"
        );
        new_line_number
    }

    fn map_line_number_by_commit_index_iterator(
        &self,
        line_number: usize,
        indexes: impl Iterator<Item = usize>,
        get_line_number_map: fn(&Vec<DiffPart>) -> LineNumberMap,
    ) -> usize {
        let mut new_line_number = line_number;
        for index in indexes {
            let commit = self.commit(index);
            let line_number_map = get_line_number_map(commit.diff_parts());
            new_line_number = line_number_map.map(new_line_number);
        }
        debug!("map_line_number: {line_number} -> {new_line_number}");
        new_line_number
    }

    pub fn is_reading(&self) -> bool {
        self.read_thread.is_some()
    }

    pub fn read_start(&mut self) -> anyhow::Result<()> {
        self.ensure_git()?;
        let path = self.path.clone();
        let repository_path = self.git().repository_path().to_path_buf();
        debug!("path: {:?}, repo: {:?}", path, repository_path);
        let (tx, rx) = mpsc::channel::<FileCommit>();
        self.rx = Some(rx);
        self.read_thread = Some(thread::spawn(move || {
            Self::read_thread(&path, &repository_path, tx)
        }));
        Ok(())
    }

    pub fn read_join(&mut self) -> anyhow::Result<()> {
        if let Some(read_thread) = self.read_thread.take() {
            read_thread.join().unwrap()?; // TODO: handle error
        }
        Ok(())
    }

    fn read_thread(
        path: &Path,
        repository_path: &Path,
        tx: mpsc::Sender<FileCommit>,
    ) -> anyhow::Result<()> {
        let mut commits = CommitIterator::new(path, repository_path);
        commits.start()?;
        let git = GitTools::from_repository_path(repository_path)?;
        let mut path = path.to_path_buf();
        for commit_id in &mut commits {
            trace!("Commit ID: {:?}, Path: {:?}", commit_id, path);
            let mut diff = FileCommit::new(commit_id, &path);
            diff.read(&git)?;
            if let Some(old_path) = diff.old_path() {
                if path != old_path {
                    debug!("read_thread: rename detected {:?} -> {:?}", old_path, path);
                    path = old_path.to_path_buf();
                }
            }
            tx.send(diff)?;
        }
        commits.join()?;
        Ok(())
    }

    pub fn read_poll(&mut self) -> anyhow::Result<bool> {
        let start_time = std::time::Instant::now();
        if self.rx.is_none() {
            return Ok(false);
        }
        let rx = self.rx.as_mut().unwrap();
        let mut count = 0;
        loop {
            match rx.try_recv() {
                Ok(commit_data) => {
                    self.commits.push(commit_data);
                    count += 1;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if count > 0 {
                        debug!(
                            "read_poll: {count} items, total {} items, {:?}",
                            self.commits.len(),
                            start_time.elapsed()
                        );
                    } else {
                        trace!(
                            "read_poll: 0 items, total {} items, {:?}",
                            self.commits.len(),
                            start_time.elapsed()
                        );
                    }
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    debug!(
                        "read_poll: disconnected, {count} items, total {} items, {:?}",
                        self.commits.len(),
                        start_time.elapsed(),
                    );
                    trace!("{:#?}", self.commits);
                    self.read_join()?;
                    break;
                }
            }
        }
        Ok(count > 0)
    }

    pub fn content(&self, commit_id: git2::Oid) -> anyhow::Result<FileContent> {
        debug!("content for {commit_id}");
        let path = if commit_id.is_zero() {
            &self.path
        } else {
            self.commits().get_by_commit_id(commit_id)?.path()
        };
        let mut content = FileContent::new(commit_id, path);
        // For testing, don't read if `path` is empty. See `new_for_test()`.
        if self.is_path_empty() {
            return Ok(content);
        }
        content.read(self.git())?;
        if !self.commits.is_empty() {
            content.update_commits(self)?;
        }
        Ok(content)
    }
}
