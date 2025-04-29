use std::{collections::HashMap, io::Write, ops::Range, path::Path};

use crate::*;
use crossterm::{cursor, queue, terminal};
use git2::Oid;

pub struct BlameRenderer {
    git: GitTools,
    content: Box<BlameContent>,
    view_size: (u16, u16),
    rendered_rows: u16,
    rendered_current_line_index: usize,
    rendered_view_start_line_index: usize,
    view_start_line_index: usize,
    current_line_number: usize,
    cache: HashMap<Oid, Box<BlameContent>>,
}

impl BlameRenderer {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let git = GitTools::from_path(path)?;

        let mut relative_path = path.canonicalize()?;
        relative_path = relative_path.strip_prefix(git.root_path())?.to_path_buf();

        Ok(Self {
            git: git,
            content: Box::new(BlameContent::new(Oid::zero(), &relative_path)),
            view_size: terminal::size()?,
            rendered_rows: 0,
            rendered_current_line_index: 0,
            rendered_view_start_line_index: 0,
            view_start_line_index: 0,
            current_line_number: 1,
            cache: HashMap::new(),
        })
    }

    pub fn view_rows(&self) -> u16 {
        self.view_size.1
    }

    pub fn set_view_size(&mut self, size: (u16, u16)) {
        self.view_size = size;
        self.scroll_current_line_into_view();
    }

    fn view_line_indexes(&self) -> Range<usize> {
        self.view_start_line_index..self.view_start_line_index + self.view_rows() as usize
    }

    pub fn rendered_rows(&self) -> u16 {
        self.rendered_rows
    }

    fn current_line_index(&self) -> usize {
        self.current_line_number - 1
    }

    fn current_line_number(&self) -> usize {
        self.current_line_number
    }

    fn current_line(&self) -> &BlameLine {
        self.content.line_by_index(self.current_line_index())
    }

    pub fn current_line_commit_id(&self) -> Oid {
        self.current_line().diff_part.commit_id
    }

    fn set_current_line_index(&mut self, index: usize) {
        self.set_current_line_number(index + 1);
    }

    pub fn set_current_line_number(&mut self, number: usize) {
        self.current_line_number = self.content.saturate_line_number(number);
        self.scroll_current_line_into_view();
    }

    fn adjust_current_line_to_valid(&mut self) {
        self.set_current_line_number(self.current_line_number());
    }

    pub fn move_to_first_line(&mut self) {
        self.set_current_line_index(0);
    }

    pub fn move_to_last_line(&mut self) {
        self.set_current_line_number(self.content.lines_len());
    }

    pub fn move_to_prev_page(&mut self) {
        let page_size = (self.view_rows() - 1) as usize;
        self.set_current_line_index(self.current_line_index().saturating_sub(page_size));
    }

    pub fn move_to_next_page(&mut self) {
        let page_size = (self.view_rows() - 1) as usize;
        self.set_current_line_index(self.current_line_index() + page_size);
    }

    pub fn move_to_prev_diff(&mut self) {
        let current_index = self.current_line_index();
        let mut first_index = self.content.first_line_index_of_diff(current_index);
        if first_index > 0 && first_index == current_index {
            first_index = self.content.first_line_index_of_diff(first_index - 1);
        }
        self.set_current_line_index(first_index);
    }

    pub fn move_to_next_diff(&mut self) {
        let last_index = self
            .content
            .last_line_index_of_diff(self.current_line_index());
        self.set_current_line_index(last_index + 1);
    }

    fn scroll_current_line_into_view(&mut self) {
        const MARGIN: usize = 5;
        let index = self.current_line_index();
        let above_margin = index.saturating_sub(MARGIN);
        if self.view_start_line_index > above_margin {
            self.view_start_line_index = above_margin;
            return;
        }
        let below_margin = self.content.saturate_line_index(index + MARGIN);
        let below_margin_start_index = (below_margin + 1).saturating_sub(self.view_rows() as usize);
        if self.view_start_line_index < below_margin_start_index {
            self.view_start_line_index = below_margin_start_index;
        }
    }

    pub fn read(&mut self) -> anyhow::Result<()> {
        self.content.read(&self.git)?;
        self.invalidate_render();
        self.adjust_current_line_to_valid();
        self.scroll_current_line_into_view();
        Ok(())
    }

    fn swap_content(&mut self, content: &mut Box<BlameContent>) {
        std::mem::swap(&mut self.content, content);
        self.invalidate_render();
        self.adjust_current_line_to_valid();
        self.scroll_current_line_into_view();
    }

    pub fn commit_id(&self) -> Oid {
        self.content.commit_id()
    }

    pub fn set_commit_id(&mut self, commit_id: Oid) -> anyhow::Result<()> {
        let mut content = if let Some(content) = self.cache.remove(&commit_id) {
            content
        } else {
            let mut content = Box::new(BlameContent::new(commit_id, self.content.path()));
            content.read(&self.git)?;
            content
        };
        self.swap_content(&mut content);
        if content.lines_len() > 0 {
            self.cache.insert(content.commit_id(), content);
        }
        Ok(())
    }

    pub fn set_commit_id_to_older_than_current_line(&mut self) -> anyhow::Result<()> {
        let diff_part = &self.current_line().diff_part;
        let id = self.git.older_commit_id(diff_part.commit_id)?;
        self.set_commit_id(id)
    }

    fn invalidate_render(&mut self) {
        self.rendered_rows = 0;
    }

    pub fn render(&mut self, out: &mut impl Write) -> anyhow::Result<()> {
        if self.rendered_rows > 0
            && self.rendered_view_start_line_index == self.view_start_line_index
        {
            self.render_line(out, self.rendered_current_line_index)?;
            self.render_line(out, self.current_line_index())?;
            self.rendered_current_line_index = self.current_line_index();
            return Ok(());
        }

        queue!(out, terminal::Clear(terminal::ClearType::All))?;
        let lines = self
            .content
            .lines()
            .iter()
            .skip(self.view_start_line_index)
            .take(self.view_rows() as usize);
        self.rendered_rows = self.render_lines(out, 0, false, lines)?;
        self.rendered_view_start_line_index = self.view_start_line_index;
        self.rendered_current_line_index = self.current_line_index();
        Ok(())
    }

    fn render_line(&mut self, out: &mut impl Write, line_index: usize) -> anyhow::Result<()> {
        if !self.view_line_indexes().contains(&line_index) {
            return Ok(());
        }
        let row = (line_index - self.view_start_line_index) as u16;
        let line = self.content.lines().iter().skip(line_index).take(1);
        self.render_lines(out, row, true, line)?;
        Ok(())
    }

    fn render_lines<'a, Iter>(
        &self,
        out: &mut impl Write,
        start_row: u16,
        should_clear: bool,
        lines: Iter,
    ) -> anyhow::Result<u16>
    where
        Iter: Iterator<Item = &'a BlameLine>,
    {
        let mut row = start_row;
        let current_line_number = self.current_line_number();
        for line in lines {
            queue!(out, cursor::MoveTo(0, row))?;
            if should_clear {
                queue!(out, terminal::Clear(terminal::ClearType::CurrentLine))?;
            }
            line.render(out, current_line_number)?;
            row += 1;
        }
        Ok(row)
    }
}
