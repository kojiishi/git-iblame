use std::{
    cmp,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::*;
use crossterm::{queue, terminal};
use git2::{BlameOptions, Oid};

pub struct BlameRenderer {
    original_path: PathBuf,
    lines: Vec<BlameLine>,
    git: GitTools,
    newest_commit_id: Oid,
    relative_path: PathBuf,
    size: (u16, u16),
    rendered_rows: u16,
    start_index: usize,
    current_number: usize,
}

impl BlameRenderer {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let git = GitTools::from_path(path)?;

        let mut relative_path = path.canonicalize()?;
        relative_path = relative_path.strip_prefix(git.root_path())?.to_path_buf();

        Ok(Self {
            original_path: path.to_path_buf(),
            lines: vec![],
            git: git,
            newest_commit_id: Oid::zero(),
            relative_path,
            size: terminal::size()?,
            rendered_rows: 0,
            start_index: 0,
            current_number: 1,
        })
    }

    pub fn rows(&self) -> u16 {
        self.size.1
    }

    pub fn set_size(&mut self, size: (u16, u16)) {
        self.size = size;
        self.scroll_current_into_view();
    }

    pub fn rendered_rows(&self) -> u16 {
        self.rendered_rows
    }

    fn current_index(&self) -> usize {
        self.current_number - 1
    }

    fn current_line(&self) -> &BlameLine {
        &self.lines[self.current_index()]
    }

    fn saturate_index(&self, index: usize) -> usize {
        cmp::min(index, self.lines.len().saturating_sub(1))
    }

    fn saturate_number(&self, number: usize) -> usize {
        cmp::max(cmp::min(number, self.lines.len()), 1)
    }

    fn set_current_index(&mut self, index: usize) {
        self.set_current_number(index + 1);
    }

    pub fn set_current_number(&mut self, number: usize) {
        self.current_number = self.saturate_number(number);
        self.scroll_current_into_view();
    }

    fn adjust_current_to_valid(&mut self) {
        self.set_current_number(self.current_number);
    }

    pub fn move_to_first_line(&mut self) {
        self.set_current_index(0);
    }

    pub fn move_to_last_line(&mut self) {
        self.set_current_number(self.lines.len());
    }

    pub fn move_to_prev_page(&mut self) {
        let page_size = (self.rows() - 1) as usize;
        self.set_current_index(self.current_index().saturating_sub(page_size));
    }

    pub fn move_to_next_page(&mut self) {
        let page_size = (self.rows() - 1) as usize;
        self.set_current_index(self.current_index() + page_size);
    }

    pub fn move_to_prev_diff(&mut self) {
        let current_index = self.current_index();
        let mut first_index = self.first_index_of_diff(current_index);
        if first_index > 0 && first_index == current_index {
            first_index = self.first_index_of_diff(first_index - 1);
        }
        self.set_current_index(first_index);
    }

    pub fn move_to_next_diff(&mut self) {
        let last_index = self.last_index_of_diff(self.current_index());
        self.set_current_index(last_index + 1);
    }

    fn first_index_of_diff(&self, index: usize) -> usize {
        let commit_id = self.lines[index].diff_part.commit_id;
        for i in (0..index).rev() {
            if self.lines[i].diff_part.commit_id != commit_id {
                return i + 1;
            }
        }
        0
    }

    fn last_index_of_diff(&self, index: usize) -> usize {
        let commit_id = self.lines[index].diff_part.commit_id;
        for i in index + 1..self.lines.len() {
            if self.lines[i].diff_part.commit_id != commit_id {
                return i - 1;
            }
        }
        self.lines.len() - 1
    }

    fn scroll_current_into_view(&mut self) {
        const MARGIN: usize = 5;
        let index = self.current_index();
        let above_margin = index.saturating_sub(MARGIN);
        if self.start_index > above_margin {
            self.start_index = above_margin;
            return;
        }
        let below_margin = self.saturate_index(index + MARGIN);
        let below_margin_start_index = (below_margin + 1).saturating_sub(self.rows() as usize);
        if self.start_index < below_margin_start_index {
            self.start_index = below_margin_start_index;
        }
    }

    pub fn current_commit_id(&self) -> Oid {
        self.current_line().diff_part.commit_id
    }

    pub fn newest_commit_id(&mut self) -> Oid {
        self.newest_commit_id
    }

    pub fn set_newest_commit_id(&mut self, id: Oid) -> anyhow::Result<()> {
        self.newest_commit_id = id;
        self.read()
    }

    pub fn set_newest_commit_id_to_older(&mut self) -> anyhow::Result<()> {
        let id = self.git.older_commit_id(self.current_commit_id())?;
        self.set_newest_commit_id(id)
    }

    pub fn read(&mut self) -> anyhow::Result<()> {
        if self.newest_commit_id.is_zero() {
            self.read_file()
        } else {
            let content = self
                .git
                .content_as_string(self.newest_commit_id, &self.relative_path)?;
            self.read_string(&content)
        }
    }

    fn read_file(&mut self) -> anyhow::Result<()> {
        let mut file = File::open(&self.original_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        self.read_string(&contents)
    }

    fn read_string(&mut self, content: &str) -> anyhow::Result<()> {
        self.lines = content
            .lines()
            .enumerate()
            .map(|(i, line)| BlameLine::new(i + 1, line))
            .collect();
        self.adjust_current_to_valid();
        self.read_blame()
    }

    fn read_blame(&mut self) -> anyhow::Result<()> {
        let mut options = BlameOptions::new();
        if !self.newest_commit_id.is_zero() {
            options.newest_commit(self.newest_commit_id);
        }
        let blame = self
            .git
            .repository()
            .blame_file(&self.relative_path, Some(&mut options))?;
        for hunk in blame.iter() {
            let part = Rc::new(DiffPart::new(hunk));
            for number in part.range.clone() {
                self.lines[number - 1].diff_part = Rc::clone(&part);
            }
        }
        Ok(())
    }

    pub fn render(&mut self, out: &mut impl Write) -> anyhow::Result<()> {
        queue!(out, terminal::Clear(terminal::ClearType::All))?;

        let lines = self
            .lines
            .iter()
            .skip(self.start_index)
            .take(self.rows() as usize);
        let mut row = 0;
        for line in lines {
            line.render(out, row, self.current_number)?;
            row += 1;
        }
        self.rendered_rows = row;
        Ok(())
    }
}
