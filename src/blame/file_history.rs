use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use log::*;

use super::{super::GitTools, CommitIterator, FileCommit, FileContent};

pub struct FileHistory {
    path: PathBuf,
    git: Option<GitTools>,
    file_diffs: Vec<FileCommit>,
    commit_diff_index_from_commit_id: HashMap<git2::Oid, usize>,
    read_thread: Option<thread::JoinHandle<anyhow::Result<()>>>,
    rx: Option<mpsc::Receiver<FileCommit>>,
}

impl FileHistory {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            git: None,
            file_diffs: Vec::new(),
            commit_diff_index_from_commit_id: HashMap::new(),
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

    fn ensure_git(&mut self) -> anyhow::Result<()> {
        if self.git.is_none() {
            let git = GitTools::from_file_path(&self.path)?;
            self.path = git.path_in_repository(&self.path)?.to_path_buf();
            self.git = Some(git);
        }
        Ok(())
    }

    pub fn git(&self) -> &GitTools {
        self.git.as_ref().unwrap()
    }

    pub fn repository_path(&self) -> &Path {
        self.git().root_path()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_diffs(&self) -> &Vec<FileCommit> {
        &self.file_diffs
    }

    pub fn commit_diff_from_commit_id(&self, commit_id: &git2::Oid) -> Option<&FileCommit> {
        self.commit_diff_index_from_commit_id
            .get(commit_id)
            .map(|index| &self.file_diffs[*index])
    }

    pub fn is_reading(&self) -> bool {
        self.read_thread.is_some()
    }

    pub fn read_start(&mut self) -> anyhow::Result<()> {
        self.ensure_git()?;
        debug!("path: {:?}, repo: {:?}", self.path, self.repository_path());
        let path = self.path.clone();
        let repository_path = self.repository_path().to_path_buf();
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
        let mut path = path.to_path_buf();
        for commit_id in &mut commits {
            trace!("Commit ID: {:?}, Path: {:?}", commit_id, path);
            let mut diff = FileCommit::new(commit_id);
            diff.read(&path, repository_path)?;
            if let Some(old_path) = diff.old_path() {
                if path != old_path {
                    trace!("read_thread: rename detected {:?} -> {:?}", old_path, path);
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
                Ok(mut diff) => {
                    let index = self.file_diffs.len();
                    diff.set_index(index);
                    self.commit_diff_index_from_commit_id
                        .insert(diff.commit_id(), index);
                    self.file_diffs.push(diff);
                    count += 1;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    debug!(
                        "read_poll: {count} items, total {} items, {:?}",
                        self.file_diffs.len(),
                        start_time.elapsed()
                    );
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    debug!(
                        "read_poll: disconnected, {count} items, total {} items, {:?}, {:#?}",
                        self.file_diffs.len(),
                        start_time.elapsed(),
                        self.file_diffs
                    );
                    self.read_join()?;
                    break;
                }
            }
        }
        Ok(count > 0)
    }

    pub fn content(&mut self, commit_id: git2::Oid) -> anyhow::Result<FileContent> {
        let mut content = FileContent::new(commit_id, &self.path);
        // For testing, don't read if `path` is empty. See `new_for_test()`.
        if self.is_path_empty() {
            return Ok(content);
        }
        content.read(self.git())?;
        content.reapply(self)?;
        Ok(content)
    }
}
