use std::{
    cmp,
    ops::Range,
    path::{Path, PathBuf},
};

use log::*;

use crate::GitTools;

use super::{DiffPart, FileCommit, FileHistory, Line, LineNumberMap};

pub struct FileContent {
    commit_id: git2::Oid,
    path: PathBuf,
    lines: Vec<Line>,
    current_line_index: usize,
}

impl FileContent {
    pub fn new(commit_id: git2::Oid, path: &Path) -> Self {
        Self {
            commit_id,
            path: path.to_path_buf(),
            lines: vec![],
            current_line_index: 0,
        }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self::new(git2::Oid::zero(), Path::new(""))
    }

    pub fn line_index_from_number(&self, line_number: usize) -> usize {
        assert!(line_number > 0);
        self.lines
            .iter()
            .position(|line| line.line_number() == line_number)
            .unwrap_or(0)
    }

    pub fn commit_id(&self) -> git2::Oid {
        self.commit_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn lines(&self) -> &Vec<Line> {
        &self.lines
    }

    pub fn lines_len(&self) -> usize {
        self.lines.len()
    }

    pub fn saturate_line_index(&self, line_index: usize) -> usize {
        cmp::min(line_index, self.lines_len().saturating_sub(1))
    }

    pub fn current_line_index(&self) -> usize {
        self.current_line_index
    }

    pub fn set_current_line_index(&mut self, line_index: usize) {
        self.current_line_index = self.saturate_line_index(line_index);
    }

    pub fn set_current_line_number(&mut self, line_number: usize) {
        self.set_current_line_index(self.line_index_from_number(line_number));
    }

    pub fn current_line(&self) -> &Line {
        &self.lines[self.current_line_index()]
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
        lines: impl Iterator<Item = (usize, &'a Line)>,
    ) -> Option<usize> {
        for (i, line) in lines {
            if line.content().to_lowercase().contains(search) {
                return Some(i);
            }
        }
        None
    }

    pub fn read(&mut self, git: &GitTools) -> anyhow::Result<()> {
        if self.commit_id.is_zero() {
            self.commit_id = git.head_commit_id()?;
        }
        let content = git.content_as_string(self.commit_id, &self.path)?;
        self.read_string(&content);
        Ok(())
    }

    fn read_string(&mut self, content: &str) {
        self.read_lines(content.lines().map(|line| line.to_string()));
    }

    fn read_lines(&mut self, lines: impl Iterator<Item = String>) {
        self.lines = lines
            .enumerate()
            .map(|(i, line)| Line::new(i + 1, line))
            .collect();
    }

    #[cfg(test)]
    pub fn set_lines_for_test(&mut self, lines: impl Iterator<Item = String>) {
        self.read_lines(lines);
    }

    #[cfg(test)]
    pub fn set_lines_len_for_test(&mut self, lines_len: usize) {
        let lines = (0..lines_len).map(|i| i.to_string());
        self.read_lines(lines);
    }

    pub fn reapply(&mut self, history: &FileHistory) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();
        for line in self.lines.iter_mut() {
            line.clear_commit_id();
        }
        let first_index = if history.file_commits().is_empty() {
            debug!("reapply: no commits loaded so far");
            assert_eq!(self.commit_id(), history.git().head_commit_id()?);
            0
        } else {
            let self_commit_index = history.commit_index_from_commit_id(self.commit_id())?;
            debug!("reapply {self_commit_index} {}", self.commit_id());
            self_commit_index
        };
        self.apply_diffs(history, first_index)?;
        self.update_after_apply();
        debug!("reapply done, elapsed: {:?}", start_time.elapsed());
        Ok(())
    }

    fn apply_diffs(&mut self, history: &FileHistory, first_index: usize) -> anyhow::Result<()> {
        let all_commits = history.file_commits();
        let commits = &all_commits[first_index..];
        for i in 0..commits.len() {
            let commit = &commits[i];
            if i > 0 {
                let mut adjusted_parts = commit.diff_parts().clone();
                for j in (0..i).rev() {
                    let parts = &commits[j].diff_parts();
                    let map = LineNumberMap::new_new_from_old(parts);
                    map.apply_to_parts(&mut adjusted_parts);
                }
                self.apply_diff_parts(&adjusted_parts, commit)?;
            } else {
                self.apply_diff_parts(commit.diff_parts(), commit)?;
            }
        }
        Ok(())
    }

    fn apply_diff_parts(
        &mut self,
        parts: &Vec<DiffPart>,
        commit: &FileCommit,
    ) -> anyhow::Result<()> {
        for part in parts {
            self.apply_diff_part(part, commit)?;
        }
        Ok(())
    }

    fn apply_diff_part(&mut self, part: &DiffPart, commit: &FileCommit) -> anyhow::Result<()> {
        let commit_id = commit.commit_id();
        let new_line_numbers = part.new.line_numbers();
        trace!("apply: #{} {new_line_numbers:?}", commit.index());
        if new_line_numbers.is_empty() {
            return Ok(()); // TODO: handle empty hunk
        }

        // Saturate `end`, as it may be set to `MAX`.
        let new_line_numbers =
            new_line_numbers.start..cmp::min(new_line_numbers.end, self.lines_len() + 1);
        let mut line_index = self.line_index_from_number(new_line_numbers.start);
        trace!("apply: index={line_index} for {new_line_numbers:?}");
        while line_index < self.lines_len() {
            let line = &mut self.lines[line_index];
            // trace!("apply: [{line_index}]={} {:?}", line.line_number(), line.commit_id());
            if line.line_number() >= new_line_numbers.end {
                break;
            }
            if line.commit_id().is_none() {
                line.set_commit_id(commit_id);
            }
            line_index += 1;
        }

        Ok(())
    }

    fn update_after_apply(&mut self) {
        let mut last_line: Option<&mut Line> = None;
        let mut last_commit_id: Option<git2::Oid> = None;
        let mut index_in_hunk = 0;
        for line in &mut self.lines {
            let commit_id = line.commit_id();
            let is_first_line_in_hunk;
            if let Some(commit_id) = commit_id {
                if last_commit_id.is_some() && last_commit_id.unwrap() == commit_id {
                    index_in_hunk += 1;
                    is_first_line_in_hunk = false;
                } else {
                    index_in_hunk = 0;
                    is_first_line_in_hunk = true
                }
            } else {
                index_in_hunk = 0;
                is_first_line_in_hunk = last_commit_id.is_some();
            }
            line.set_index_in_hunk(index_in_hunk);
            if let Some(last_line) = last_line {
                last_line.set_is_last_line_in_hunk(is_first_line_in_hunk);
            }

            last_line = Some(line);
            last_commit_id = commit_id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search() -> anyhow::Result<()> {
        let mut content = FileContent::new_for_test();
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
