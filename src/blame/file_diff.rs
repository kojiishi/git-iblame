use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use log::*;

use crate::GitTools;

#[derive(Debug)]
pub struct FileDiff {
    commit_id: git2::Oid,
    index: usize,
    time: git2::Time,
    summary: Option<String>,
    old_path: Option<PathBuf>,
    parts: Vec<DiffPart>,
}

impl FileDiff {
    pub fn new(commit_id: git2::Oid) -> Self {
        Self {
            commit_id,
            index: 0,
            time: git2::Time::new(0, 0),
            summary: None,
            old_path: None,
            parts: Vec::new(),
        }
    }

    pub fn commit_id(&self) -> git2::Oid {
        self.commit_id
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn time(&self) -> git2::Time {
        self.time
    }

    pub fn summary(&self) -> &Option<String> {
        &self.summary
    }

    pub fn old_path(&self) -> Option<&Path> {
        self.old_path.as_deref()
    }

    pub fn parts(&self) -> &Vec<DiffPart> {
        &self.parts
    }

    pub fn old_line_number(&self, line_number: usize) -> usize {
        for diff_hunk in &self.parts {
            if diff_hunk.new.line_numbers.contains(&line_number) {
                debug!("map {line_number} by {diff_hunk:?}");
                let index_in_hunk = line_number - diff_hunk.new.line_numbers.start;
                if !diff_hunk.old.line_numbers.is_empty() {
                    return index_in_hunk + diff_hunk.old.line_numbers.start;
                }
                // TODO: What to do? Accumulate diffs up to this point?
            }
        }
        // TODO: What to do? Accumulate diffs up to this point?
        line_number
    }

    pub fn read(&mut self, path: &Path, repository_path: &Path) -> anyhow::Result<()> {
        trace!("read diff for commit_id: {:?} {path:?}", self.commit_id);
        assert!(path.is_relative());
        assert!(self.parts.is_empty());
        let start_time = std::time::Instant::now();
        let git = GitTools::from_repository_path(repository_path)?;
        let commit_id = self.commit_id;
        trace!("commit_id: {commit_id}");
        let commit = git.repository().find_commit(commit_id)?;
        self.time = commit.time();
        self.summary = commit.summary().map(|s| s.to_string());

        let parent = commit.parent(0);
        if parent.is_err() {
            trace!("no parent");
            let mut diff_hunk = DiffPart::default();
            diff_hunk.new.line_numbers = 1..usize::MAX;
            self.parts.push(diff_hunk);
            return Ok(());
        }
        let parent = parent.unwrap();

        let tree = commit.tree()?;
        let parent_tree = parent.tree()?;
        let mut diff_options = git2::DiffOptions::new();
        diff_options.ignore_whitespace(true);
        let mut diff = git.repository().diff_tree_to_tree(
            Some(&parent_tree),
            Some(&tree),
            Some(&mut diff_options),
        )?;
        diff.find_similar(None)?;

        let mut old_path: Option<PathBuf> = None;
        let mut context: DiffReadContext = DiffReadContext::default();
        diff.foreach(
            &mut |delta, _| {
                let new_path = delta.new_file().path();
                if new_path.is_none() || new_path.unwrap() != path {
                    // trace!("file: {new_path:?}, not interesting");
                    return true;
                }
                trace!("file: {new_path:?} {delta:?}");
                old_path = delta.old_file().path().map(|p| p.to_path_buf());
                true
            },
            None,
            None,
            Some(&mut |delta, hunk, line| {
                let new_path = delta.new_file().path();
                if new_path.is_none() || new_path.unwrap() != path {
                    // trace!("line: {new_path:?}, not interesting");
                    return true;
                }
                let hunk = hunk.unwrap();
                trace!(
                    "line: {new_path:?} {:?} hunk: {},{} -> {},{} {:?} {},{} -> {},{} {:?}->{:?},{} {:?}",
                    delta.old_file().path(),
                    hunk.old_start(),
                    hunk.old_lines(),
                    hunk.new_start(),
                    hunk.new_lines(),
                    line.origin_value(),
                    hunk.old_start(),
                    hunk.old_lines(),
                    hunk.new_start(),
                    hunk.new_lines(),
                    line.old_lineno(),
                    line.new_lineno(),
                    line.num_lines(),
                    String::from_utf8(line.content().to_vec())
                );
                if context.hunk_new_start.is_none()
                    || hunk.new_start() != context.hunk_new_start.unwrap()
                {
                    context.hunk_new_start = Some(hunk.new_start());
                    context.flush_part();
                }
                match line.origin_value() {
                    git2::DiffLineType::Context => {
                        context.flush_part();
                    }
                    git2::DiffLineType::Addition => {
                        assert_eq!(line.num_lines(), 1);
                        let part = context.ensure_part();
                        part.new.add_line_raw(line.new_lineno());
                    }
                    git2::DiffLineType::Deletion => {
                        assert_eq!(line.num_lines(), 1);
                        let part = context.ensure_part();
                        part.old.add_line_raw(line.old_lineno());
                    }
                    _ => {
                        trace!("origin {:?} skipped", line.origin_value());
                    }
                }
                true
            }),
        )?;
        context.flush_part();

        self.old_path = old_path;
        self.parts = context.parts;
        trace!(
            "read diff for commit_id: {:?} done, elapsed {:?}: {self:#?}",
            self.commit_id,
            start_time.elapsed()
        );
        Ok(())
    }
}

#[derive(Debug, Default)]
struct DiffReadContext {
    parts: Vec<DiffPart>,
    part: Option<DiffPart>,
    hunk_new_start: Option<u32>,
}

impl DiffReadContext {
    pub fn ensure_part(&mut self) -> &mut DiffPart {
        if self.part.is_none() {
            self.part = Some(DiffPart::default());
        }
        self.part.as_mut().unwrap()
    }

    pub fn flush_part(&mut self) {
        if let Some(part) = self.part.take() {
            trace!("diff_hunk: {part:?}");
            self.parts.push(part);
        }
    }
}

#[derive(Debug, Default)]
pub struct DiffPart {
    pub old: DiffLines,
    pub new: DiffLines,
}

#[derive(Debug, Default)]
pub struct DiffLines {
    pub line_numbers: Range<usize>,
}

impl DiffLines {
    pub fn start_line_number(&self) -> usize {
        self.line_numbers.start
    }

    pub fn line_numbers(&self) -> &Range<usize> {
        &self.line_numbers
    }

    fn add_line_raw(&mut self, line_number: Option<u32>) {
        self.add_line(line_number.unwrap_or(0) as usize, 1);
    }

    pub fn add_line(&mut self, line_number: usize, len: usize) {
        if self.line_numbers.is_empty() {
            self.line_numbers = line_number..line_number + len;
        } else {
            assert_eq!(self.line_numbers.end, line_number);
            self.line_numbers.end += len;
        }
    }
}
