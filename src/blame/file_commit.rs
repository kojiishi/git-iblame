use std::{
    io::BufReader,
    path::{Path, PathBuf},
};

use log::*;
use regex::Regex;

use crate::extensions::{GitTools, LineReadBuffer};

use super::DiffPart;

#[derive(Copy, Clone, Debug)]
enum GitEngine {
    Git,
    Git2,
}
static mut DIFF_ENGINE: GitEngine = GitEngine::Git;

#[derive(Debug)]
pub struct FileCommit {
    commit_id: git2::Oid,
    path: PathBuf,
    index: usize,
    time: git2::Time,
    summary: Option<String>,
    author_email: Option<String>,
    old_path: Option<PathBuf>,
    diff_parts: Vec<DiffPart>,
}

impl FileCommit {
    pub fn new(commit_id: git2::Oid, path: &Path) -> Self {
        Self {
            commit_id,
            path: path.to_path_buf(),
            index: 0,
            time: git2::Time::new(0, 0),
            summary: None,
            author_email: None,
            old_path: None,
            diff_parts: Vec::new(),
        }
    }

    fn git_engine() -> GitEngine {
        unsafe { DIFF_ENGINE }
    }

    pub fn use_git2() {
        unsafe { DIFF_ENGINE = GitEngine::Git2 }
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

    pub fn summary(&self) -> Option<&String> {
        self.summary.as_ref()
    }

    pub fn author_email(&self) -> Option<&String> {
        self.author_email.as_ref()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The old path before rename if this is a rename, `None` otherwise.
    pub fn old_path_if_rename(&self) -> Option<&Path> {
        self.old_path.as_deref()
    }

    pub fn diff_parts(&self) -> &Vec<DiffPart> {
        &self.diff_parts
    }

    pub fn read(&mut self, git: &GitTools) -> anyhow::Result<()> {
        assert!(self.path.is_relative());
        assert!(self.diff_parts.is_empty());
        match Self::git_engine() {
            GitEngine::Git => self.read_by_git(git),
            GitEngine::Git2 => self.read_by_git2(git),
        }?;
        DiffPart::validate_ascending_parts(&self.diff_parts)
    }

    fn read_by_git(&mut self, git: &GitTools) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();
        let commit_id = self.commit_id;
        debug!("read_by_git.start: {commit_id:?} {:?}", self.path);
        let commit = git.repository().find_commit(commit_id)?;
        self.set_commit(&commit);

        let mut command = git.create_show_command(commit_id);
        let mut child = command.stdout(std::process::Stdio::piped()).spawn()?;
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut buffer = LineReadBuffer::new();
        let re_file = Regex::new(r"^diff --git a/(.+) b/(.+)$")?;
        let re_hunk = Regex::new(r"^@@ -(\d+),(\d+) \+(\d+),(\d+) @@")?;

        let path = self.path.to_str().unwrap();
        let mut is_in_hunk = false;
        let mut is_path_found = false;
        let mut old_path: Option<PathBuf> = None;
        let mut context = DiffReadContext::default();
        while buffer.read_line_from(&mut reader)? {
            let line = buffer.as_ref();
            if buffer.invalid_len() > 0
                && (line.is_empty() || line.starts_with("diff ") || line.starts_with("@@ "))
            {
                return Err(buffer.error());
            }
            if let Some(captures) = re_file.captures(line) {
                if is_path_found {
                    if let Err(error) = child.kill() {
                        warn!("Stopping child: {error}");
                    }
                    break;
                }
                is_in_hunk = false;
                let new_path = captures.get(2).unwrap().as_str();
                is_path_found = new_path == path;
                if !is_path_found {
                    // trace!("file: {new_path:?}, not interesting");
                    continue;
                }
                let old_path_from_git = captures.get(1).unwrap().as_str();
                if old_path_from_git != path {
                    old_path = Some(old_path_from_git.into());
                }
                trace!("file: found {new_path:?} old={old_path:?} {old_path_from_git}");
                continue;
            }
            if !is_path_found {
                continue;
            }
            if let Some(captures) = re_hunk.captures(line) {
                let old_line_number = captures.get(1).unwrap().as_str().parse::<usize>()?;
                let new_line_number = captures.get(3).unwrap().as_str().parse::<usize>()?;
                context.on_git_hunk(old_line_number, new_line_number);
                is_in_hunk = true;
                continue;
            }
            if is_in_hunk {
                context.on_git_line(line);
            }
        }
        context.flush_part();

        self.old_path = old_path;
        self.diff_parts = context.parts;
        debug!("read_by_git.done: elapsed {:?}", start_time.elapsed());
        trace!("read_by_git.result={self:#?}");

        let exit_status = child.wait()?;
        trace!(
            "read_by_git.exit: {exit_status}, elapsed {:?}",
            start_time.elapsed()
        );
        Ok(())
    }

    fn read_by_git2(&mut self, git: &GitTools) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();
        let commit_id = self.commit_id;
        debug!("read_by_git2.start: {commit_id:?} {:?}", self.path);
        let commit = git.repository().find_commit(commit_id)?;
        self.set_commit(&commit);

        let parent = commit.parent(0);
        if parent.is_err() {
            trace!("no parent");
            let mut diff_hunk = DiffPart::default();
            diff_hunk.new.line_numbers = 1..usize::MAX;
            self.diff_parts.push(diff_hunk);
            return Ok(());
        }
        let parent = parent.unwrap();

        let tree = commit.tree()?;
        let parent_tree = parent.tree()?;
        trace!(
            "parent={}, tree={}..{}: elapsed {:?}",
            parent.id(),
            parent_tree.id(),
            tree.id(),
            start_time.elapsed()
        );

        let mut diff_options = git2::DiffOptions::new();
        diff_options.ignore_whitespace(true);
        let mut diff = git.repository().diff_tree_to_tree(
            Some(&parent_tree),
            Some(&tree),
            Some(&mut diff_options),
        )?;
        trace!("diff_tree_to_tree: elapsed {:?}", start_time.elapsed());

        let mut diff_find_options = git2::DiffFindOptions::new();
        diff_find_options.renames(true);
        diff.find_similar(Some(&mut diff_find_options))?;
        trace!("find_similar: elapsed {:?}", start_time.elapsed());

        let path = self.path.as_path();
        let mut old_path: Option<PathBuf> = None;
        let mut context = DiffReadContext::default();
        diff.foreach(
            &mut |delta, _| {
                if !DiffReadContext::is_path(&delta, path) {
                    return true;
                }
                trace!("file: {delta:?}");
                if let Some(delta_old_path) = delta.old_file().path() {
                    if delta_old_path != path {
                        old_path = Some(delta_old_path.to_path_buf());
                    }
                }
                true
            },
            None,
            None,
            Some(&mut |delta, hunk, line| {
                if !DiffReadContext::is_path(&delta, path) {
                    // trace!("line: {delta:?}, not interesting");
                    return true;
                }
                let hunk = hunk.unwrap();
                trace!(
                    "line: {:?} {:?} hunk: {},{}->{},{} line: {}{:?} {},{} -> {},{} {:?}->{:?},{} {:?}",
                    delta.new_file().path(),
                    delta.old_file().path(),
                    hunk.old_start(),
                    hunk.old_lines(),
                    hunk.new_start(),
                    hunk.new_lines(),
                    line.origin(),
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
                context.on_line_callback(line.origin(), line.old_lineno(), line.new_lineno(), line.num_lines());
                true
            }),
        )?;
        context.flush_part();

        if let Some(old_path) = old_path {
            if old_path != self.path() {
                self.old_path = Some(old_path);
            }
        }
        self.diff_parts = context.parts;
        debug!("read_by_git2.done: elapsed {:?}", start_time.elapsed());
        trace!("read_by_git2.result={self:#?}");
        Ok(())
    }

    fn set_commit(&mut self, commit: &git2::Commit) {
        self.time = commit.time();
        self.summary = commit.summary().map(|s| s.to_string());
        self.author_email = commit.author().email().map(|s| s.to_string());
    }
}

#[derive(Debug, Default)]
struct DiffReadContext {
    old_line_number: usize,
    new_line_number: usize,
    part: DiffPart,
    parts: Vec<DiffPart>,
}

impl DiffReadContext {
    fn is_path(delta: &git2::DiffDelta, target_path: &Path) -> bool {
        let new_path = delta.new_file().path();
        if new_path.is_none() || new_path.unwrap() != target_path {
            // trace!("{new_path:?}, not interesting");
            return false;
        }
        true
    }

    fn on_line_callback(
        &mut self,
        origin: char,
        old_line_number: Option<u32>,
        new_line_number: Option<u32>,
        num_lines: u32,
    ) {
        // https://docs.rs/git2/latest/git2/struct.DiffLine.html#method.origin
        match origin {
            ' ' => {
                assert!(old_line_number.is_some());
                assert!(new_line_number.is_some());
                assert_eq!(num_lines, 1);
                self.flush_part();
                self.old_line_number = old_line_number.unwrap() as usize + 1;
                self.new_line_number = new_line_number.unwrap() as usize + 1;
            }
            '+' => {
                assert!(old_line_number.is_none());
                assert!(new_line_number.is_some());
                // `num_lines` is normally 1, but could be 0 if no newline.
                assert!(num_lines <= 1, "{num_lines} {self:?}");
                self.part.new.add_line(new_line_number.unwrap() as usize);
            }
            '-' => {
                assert!(old_line_number.is_some());
                assert!(new_line_number.is_none());
                assert_eq!(num_lines, 1);
                self.part.old.add_line(old_line_number.unwrap() as usize);
            }
            '<' => {}
            _ => {
                debug!("origin {origin:?} skipped");
            }
        }
    }

    fn on_git_hunk(&mut self, old_line_number: usize, new_line_number: usize) {
        trace!("on_git_hunk: {old_line_number},{new_line_number}");
        self.old_line_number = old_line_number;
        self.new_line_number = new_line_number;
    }

    fn on_git_line(&mut self, line: &str) {
        let origin = line.as_bytes().first().map_or('\0', |b| *b as char);
        trace!(
            "on_git_line: {origin} {},{}",
            self.old_line_number, self.new_line_number
        );
        match origin {
            ' ' => {
                self.flush_part();
                self.old_line_number += 1;
                self.new_line_number += 1;
            }
            '+' => {
                self.part.new.add_line(self.new_line_number);
                self.new_line_number += 1;
            }
            '-' => {
                self.part.old.add_line(self.old_line_number);
                self.old_line_number += 1;
            }
            // "\ No newline at end of file"
            '\\' => {}
            _ => unreachable!("Unexpected line: \"{line}\""),
        }
        trace!("on_git_line: done: {:?} {:?}", self.part.old, self.part.new);
    }

    fn flush_part(&mut self) {
        if !self.part.is_empty() {
            trace!("flush_part: {:?}", self.part);
            self.part.old.set_line_number_if_empty(self.old_line_number);
            self.part.new.set_line_number_if_empty(self.new_line_number);
            self.part.validate_ascending().unwrap();
            self.parts.push(self.part.clone());
            self.part = DiffPart::default();
            assert!(self.part.is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::extensions::tests::TempRepository;

    use super::*;

    #[test]
    fn git_add_del() -> anyhow::Result<()> {
        let git = TempRepository::new()?;
        let path = Path::new("text.txt");
        git.add_file_content(path, "1\n2\n3\n4\n5\n")?;
        let commit_id1 = git.commit(git2::Oid::zero(), "Add file")?;

        git.add_file_content(path, "1\n2\nX\nY\nZ\n4\n5\n")?;
        let commit_id2 = git.commit(commit_id1, "Rename file")?;

        let mut file_commit = FileCommit::new(commit_id2, path);
        file_commit.read(&git.git)?;
        assert_eq!(file_commit.diff_parts, [DiffPart::from_ranges(3..4, 3..6)]);
        assert_eq!(file_commit.old_path_if_rename(), None);
        Ok(())
    }

    #[test]
    fn git_add_file_no_newline() -> anyhow::Result<()> {
        // Create a dummy initial commit to avoid "no parent" code path.
        let git = TempRepository::new()?;
        git.add_file_content(Path::new("initial.txt"), "initial")?;
        let commit_id1 = git.commit(git2::Oid::zero(), "Initial")?;

        // Note no new line at end of file.
        let path = Path::new("test.txt");
        git.add_file_content(path, "1\n2\n3\n4\n5")?;
        let commit_id2 = git.commit(commit_id1, "Add file")?;

        let mut file_commit = FileCommit::new(commit_id2, path);
        file_commit.read(&git.git)?;
        assert_eq!(file_commit.diff_parts, [DiffPart::from_ranges(0..0, 1..6)]);
        assert_eq!(file_commit.old_path_if_rename(), None);

        // Add the newline at the end of file.
        git.add_file_content(path, "1\n2\n3\n4\n5\n")?;
        let commit_id3 = git.commit(commit_id2, "Add newline")?;

        let mut file_commit = FileCommit::new(commit_id3, path);
        file_commit.read(&git.git)?;
        match FileCommit::git_engine() {
            GitEngine::Git => {
                assert_eq!(file_commit.diff_parts, [DiffPart::from_ranges(5..6, 5..6)])
            }
            // `git2` can't report changes in the newline at end.
            GitEngine::Git2 => assert_eq!(file_commit.diff_parts, []),
        }
        assert_eq!(file_commit.old_path_if_rename(), None);
        Ok(())
    }

    #[test]
    fn git_rename() -> anyhow::Result<()> {
        let git = TempRepository::new()?;
        let old_path = Path::new("old.txt");
        let new_path = Path::new("new.txt");
        git.add_file_content(old_path, "content")?;
        let commit_id1 = git.commit(git2::Oid::zero(), "Add file")?;

        git.rename_file(old_path, new_path)?;
        let commit_id2 = git.commit(commit_id1, "Rename file")?;

        let mut file_commit = FileCommit::new(commit_id2, new_path);
        file_commit.read(&git.git)?;
        assert_eq!(file_commit.old_path_if_rename(), Some(old_path));
        assert!(file_commit.diff_parts.is_empty());
        Ok(())
    }

    #[test]
    fn context_add() {
        let mut context = DiffReadContext::default();
        context.on_line_callback(' ', Some(2), Some(2), 1);
        context.on_line_callback(' ', Some(3), Some(3), 1);
        context.on_line_callback(' ', Some(4), Some(4), 1);
        context.on_line_callback('+', None, Some(5), 1);
        context.on_line_callback('+', None, Some(6), 1);
        context.on_line_callback(' ', Some(5), Some(7), 1);
        context.on_line_callback(' ', Some(6), Some(8), 1);
        context.on_line_callback('+', None, Some(9), 1);
        context.on_line_callback(' ', Some(7), Some(10), 1);
        context.flush_part();
        assert_eq!(
            context.parts,
            [
                DiffPart::from_ranges(5..5, 5..7),
                DiffPart::from_ranges(7..7, 9..10),
            ]
        );
    }

    #[test]
    fn context_delete() {
        let mut context = DiffReadContext::default();
        context.on_line_callback(' ', Some(1), Some(1), 1);
        context.on_line_callback(' ', Some(2), Some(2), 1);
        context.on_line_callback(' ', Some(3), Some(3), 1);
        context.on_line_callback('-', Some(4), None, 1);
        context.on_line_callback('-', Some(5), None, 1);
        context.on_line_callback(' ', Some(6), Some(8), 1);
        context.on_line_callback('-', Some(7), None, 1);
        context.flush_part();
        assert_eq!(
            context.parts,
            [
                DiffPart::from_ranges(4..6, 4..4),
                DiffPart::from_ranges(7..8, 9..9),
            ]
        );
    }
}
