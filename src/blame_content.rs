use std::{
    cmp,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::*;
use git2::{BlameOptions, Oid};

#[derive(Debug)]
pub struct BlameContent {
    commit_id: Oid,
    path: PathBuf,
    lines: Vec<BlameLine>,
}

impl BlameContent {
    pub fn new(commit_id: Oid, path: &Path) -> Self {
        assert!(path.is_relative());
        BlameContent {
            commit_id: commit_id,
            path: path.to_path_buf(),
            lines: vec![],
        }
    }

    pub fn commit_id(&self) -> Oid {
        self.commit_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn lines(&self) -> &Vec<BlameLine> {
        &self.lines
    }

    pub fn lines_len(&self) -> usize {
        self.lines.len()
    }

    pub fn line_by_index(&self, index: usize) -> &BlameLine {
        &self.lines[index]
    }

    pub fn saturate_line_index(&self, index: usize) -> usize {
        cmp::min(index, self.lines.len().saturating_sub(1))
    }

    pub fn saturate_line_number(&self, number: usize) -> usize {
        cmp::max(cmp::min(number, self.lines.len()), 1)
    }

    pub fn first_line_index_of_diff(&self, index: usize) -> usize {
        let commit_id = self.lines[index].diff_part.commit_id;
        for i in (0..index).rev() {
            if self.lines[i].diff_part.commit_id != commit_id {
                return i + 1;
            }
        }
        0
    }

    pub fn last_line_index_of_diff(&self, index: usize) -> usize {
        let commit_id = self.lines[index].diff_part.commit_id;
        for i in index + 1..self.lines.len() {
            if self.lines[i].diff_part.commit_id != commit_id {
                return i - 1;
            }
        }
        self.lines.len() - 1
    }

    pub fn read(&mut self, git: &GitTools) -> anyhow::Result<()> {
        if self.commit_id.is_zero() {
            let path = git.root_path().join(&self.path);
            self.read_file(&path)?;
        } else {
            let content = git.content_as_string(self.commit_id, &self.path)?;
            self.read_string(&content);
        }
        self.read_blame(git)
    }

    fn read_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        self.read_string(&contents);
        Ok(())
    }

    fn read_string(&mut self, content: &str) {
        self.lines = content
            .lines()
            .enumerate()
            .map(|(i, line)| BlameLine::new(i + 1, line))
            .collect();
    }

    fn read_blame(&mut self, git: &GitTools) -> anyhow::Result<()> {
        let mut options = BlameOptions::new();
        if !self.commit_id.is_zero() {
            options.newest_commit(self.commit_id);
        }
        let blame = git
            .repository()
            .blame_file(&self.path, Some(&mut options))?;
        for hunk in blame.iter() {
            let part = Rc::new(DiffPart::new(hunk));
            for number in part.range.clone() {
                self.lines[number - 1].diff_part = Rc::clone(&part);
            }
        }
        Ok(())
    }
}
