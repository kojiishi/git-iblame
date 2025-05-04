use std::{
    cmp,
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

use crate::*;
use git2::{BlameOptions, Oid};
use log::*;

#[derive(Debug)]
pub struct BlameContent {
    commit_id: Oid,
    path: PathBuf,
    lines: Vec<BlameLine>,
    current_line_index: usize,
}

impl BlameContent {
    pub fn new(commit_id: Oid, path: &Path) -> Self {
        assert!(path.is_relative());
        BlameContent {
            commit_id,
            path: path.to_path_buf(),
            lines: vec![],
            current_line_index: 0,
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

    pub fn line_index_from_number(&self, line_number: usize) -> usize {
        assert!(line_number > 0);
        line_number.saturating_sub(1)
    }

    pub fn line_number_from_index(&self, line_index: usize) -> usize {
        line_index + 1
    }

    pub fn saturate_line_index(&self, index: usize) -> usize {
        cmp::min(index, self.lines.len().saturating_sub(1))
    }

    pub fn current_line_index(&self) -> usize {
        self.current_line_index
    }

    pub fn current_line_number(&self) -> usize {
        self.line_number_from_index(self.current_line_index)
    }

    pub fn current_line(&self) -> &BlameLine {
        self.line_by_index(self.current_line_index())
    }

    pub fn set_current_line_index(&mut self, line_index: usize) {
        self.current_line_index = self.saturate_line_index(line_index);
    }

    pub fn set_current_line_number(&mut self, line_number: usize) {
        self.set_current_line_index(self.line_index_from_number(line_number));
    }

    fn adjust_current_line_to_valid(&mut self) {
        self.set_current_line_index(self.current_line_index());
    }

    pub fn first_line_index_of_diff(&self, line_index: usize) -> usize {
        let diff_part = &self.lines[line_index].diff_part;
        self.line_index_from_number(diff_part.line_number.start)
    }

    pub fn last_line_index_of_diff(&self, line_index: usize) -> usize {
        let diff_part = &self.lines[line_index].diff_part;
        self.line_index_from_number(diff_part.line_number.end.saturating_sub(1))
    }

    pub fn search(&self, search: &str, reverse: bool) -> Option<usize> {
        let search = search.to_lowercase();
        let mut start_line_index = self.current_line_index();
        self.search_ranges(
            &search,
            reverse,
            if reverse {
                [0..start_line_index, start_line_index..self.lines.len()]
            } else {
                start_line_index += 1;
                [start_line_index..self.lines.len(), 0..start_line_index]
            },
        )
    }

    fn search_ranges(
        &self,
        search: &str,
        reverse: bool,
        line_index_ranges: [Range<usize>; 2],
    ) -> Option<usize> {
        for line_index_range in line_index_ranges {
            if let Some(line_index) = self.search_range(search, reverse, line_index_range.clone()) {
                return Some(line_index);
            }
        }
        None
    }

    fn search_range(
        &self,
        search: &str,
        reverse: bool,
        line_index_range: Range<usize>,
    ) -> Option<usize> {
        let start_line_index = line_index_range.start;
        let lines = self.lines[line_index_range].iter().enumerate();
        let result = if reverse {
            self.search_lines_enumerate(search, lines.rev())
        } else {
            self.search_lines_enumerate(search, lines)
        };
        result.map(|i| start_line_index + i)
    }

    fn search_lines_enumerate<'a>(
        &self,
        search: &str,
        lines: impl Iterator<Item = (usize, &'a BlameLine)>,
    ) -> Option<usize> {
        for (i, line) in lines {
            if line.line.to_lowercase().contains(search) {
                return Some(i);
            }
        }
        None
    }

    pub fn read(&mut self, git: &GitTools) -> anyhow::Result<()> {
        let content = git.content_as_string(self.commit_id, &self.path)?;
        self.read_string(&content);
        self.read_blame(git)
    }

    fn read_string(&mut self, content: &str) {
        self.read_lines(content.lines().map(|line| line.to_string()));
    }

    fn read_lines(&mut self, lines: impl Iterator<Item = String>) {
        self.lines = lines
            .enumerate()
            .map(|(i, line)| BlameLine::new(i + 1, line))
            .collect();
        self.adjust_current_line_to_valid();
    }

    #[cfg(test)]
    pub fn set_lines_len_for_test(&mut self, lines_len: usize) {
        let lines = (0..lines_len).map(|i| i.to_string());
        self.read_lines(lines);
    }

    fn read_blame(&mut self, git: &GitTools) -> anyhow::Result<()> {
        debug!("read_blame: {:?}", self.path);
        let start_time = Instant::now();

        let mut options = BlameOptions::new();
        options.ignore_whitespace(true);
        if !self.commit_id.is_zero() {
            options.newest_commit(self.commit_id);
        }
        let blame = git
            .repository()
            .blame_file(&self.path, Some(&mut options))?;

        // To assign indexes to each commit, because the fields of `Rc` are
        // immutable, create two `BlameCommit` objects: one with only the commit
        // ID, and one with the signature.
        struct Entry {
            commit_id_only: Rc<BlameCommit>,
            with_signature: BlameCommit,
        }
        let mut commit_map = HashMap::<Oid, Entry>::new();
        let mut diff_parts = Vec::<DiffPart>::new();
        let start_iterate_time = Instant::now();
        for hunk in blame.iter() {
            let diff_part = DiffPart::new(hunk, |commit_id, signature| {
                if let Some(entry) = commit_map.get(&commit_id) {
                    return Rc::clone(&entry.commit_id_only);
                }
                let commit_id_only = Rc::new(BlameCommit::new_with_commit_id(commit_id));
                commit_map.insert(
                    commit_id,
                    Entry {
                        commit_id_only: Rc::clone(&commit_id_only),
                        with_signature: BlameCommit::new_with_signature(commit_id, signature),
                    },
                );
                commit_id_only
            });
            diff_parts.push(diff_part);
        }

        // To assign indexes to each commit, sort the commits by time.
        // The `commit_id_only` is no longer needed and thus discarded.
        let mut commits: Vec<BlameCommit> = commit_map
            .into_values()
            .map(|entry| entry.with_signature)
            .collect();
        commits.sort_by_key(|commit| commit.when);
        for (i, commit) in commits.iter_mut().enumerate() {
            commit.index = i;
        }

        // Assign the commit to `DiffPart` and `BlameLine`.
        let commit_map = commits
            .into_iter()
            .map(|commit| (commit.commit_id, Rc::new(commit)))
            .collect::<HashMap<_, _>>();
        for mut diff_part in diff_parts.into_iter() {
            diff_part.commit = commit_map.get(&diff_part.commit_id()).unwrap().clone();
            let diff_part = Rc::new(diff_part);
            for number in diff_part.line_number.clone() {
                self.lines[number - 1].diff_part = Rc::clone(&diff_part);
            }
        }

        debug!(
            "read_blame: done, elapsed {:?} ({:?} to iterate)",
            start_time.elapsed(),
            start_iterate_time.elapsed()
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search() -> anyhow::Result<()> {
        let mut content = BlameContent::new(Oid::zero(), Path::new("a"));
        content.read_lines(((0..10).chain(0..10)).map(|i| i.to_string()));
        let mut test = |start_index: usize, search: &str| -> (Option<usize>, Option<usize>) {
            content.set_current_line_index(start_index);
            (content.search(search, false), content.search(search, true))
        };

        assert_eq!(test(0, "X"), (None, None));

        assert_eq!(test(0, "5"), (Some(5), Some(15)));
        assert_eq!(test(5, "5"), (Some(15), Some(15)));
        assert_eq!(test(10, "5"), (Some(15), Some(5)));
        assert_eq!(test(15, "5"), (Some(5), Some(5)));
        assert_eq!(test(18, "5"), (Some(5), Some(15)));
        Ok(())
    }
}
