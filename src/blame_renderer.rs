use std::{collections::HashMap, io::Write, ops::Range, path::Path, rc::Rc};

use crate::*;
use anyhow::bail;
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
    cache: HashMap<Oid, Box<BlameContent>>,
}

impl BlameRenderer {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let git = GitTools::from_path(path)?;

        let mut relative_path = path.canonicalize()?;
        relative_path = relative_path.strip_prefix(git.root_path())?.to_path_buf();

        Ok(Self {
            git,
            content: Box::new(BlameContent::new(Oid::zero(), &relative_path)),
            view_size: terminal::size()?,
            rendered_rows: 0,
            rendered_current_line_index: 0,
            rendered_view_start_line_index: 0,
            view_start_line_index: 0,
            cache: HashMap::new(),
        })
    }

    #[cfg(test)]
    pub fn from_git_tools(git: GitTools, path: &Path, view_size: (u16, u16)) -> Self {
        Self {
            git,
            content: Box::new(BlameContent::new(Oid::zero(), path)),
            view_size,
            rendered_rows: 0,
            rendered_current_line_index: 0,
            rendered_view_start_line_index: 0,
            view_start_line_index: 0,
            cache: HashMap::new(),
        }
    }

    pub fn view_rows(&self) -> u16 {
        self.view_size.1
    }

    pub fn set_view_size(&mut self, size: (u16, u16)) {
        self.view_size = size;
        self.scroll_current_line_into_view();
    }

    fn view_end_line_index(&self) -> usize {
        self.view_start_line_index + self.view_rows() as usize
    }

    fn view_line_indexes(&self) -> Range<usize> {
        self.view_start_line_index..self.view_end_line_index()
    }

    fn adjust_line_index_range_into_view(&self, line_index_range: &Range<usize>) -> Range<usize> {
        line_index_range.intersect(self.view_line_indexes())
    }

    pub fn rendered_rows(&self) -> u16 {
        self.rendered_rows
    }

    fn current_line_index(&self) -> usize {
        self.content.current_line_index()
    }

    fn current_line_number(&self) -> usize {
        self.content.current_line_number()
    }

    fn current_line(&self) -> &BlameLine {
        self.content.current_line()
    }

    pub fn current_line_commit_id(&self) -> Oid {
        self.current_line().commit_id()
    }

    fn set_current_line_index(&mut self, line_index: usize) {
        self.content.set_current_line_index(line_index);
        self.scroll_current_line_into_view();
    }

    pub fn set_current_line_number(&mut self, line_number: usize) {
        self.content.set_current_line_number(line_number);
        self.scroll_current_line_into_view();
    }

    pub fn move_to_first_line(&mut self) {
        self.set_current_line_index(0);
    }

    pub fn move_to_last_line(&mut self) {
        self.set_current_line_index(self.content.lines_len().saturating_sub(1));
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

    pub fn search(&mut self, search: &str, reverse: bool) {
        if let Some(line_index) = self.content.search(search, reverse) {
            self.set_current_line_index(line_index);
        }
    }

    fn scroll_current_line_into_view(&mut self) {
        // Content may became smaller. Ensure all view rows are filled.
        let view_rows = self.view_rows() as usize;
        let max_start_line_index = self.content.lines_len().saturating_sub(view_rows);
        if self.view_start_line_index > max_start_line_index {
            self.view_start_line_index = max_start_line_index;
        }

        // Scroll up to ensure `MARGIN` lines above the current line are visible.
        const MARGIN: usize = 5;
        let line_index = self.current_line_index();
        let above_margin = line_index.saturating_sub(MARGIN);
        if self.view_start_line_index > above_margin {
            self.view_start_line_index = above_margin;
            return;
        }

        // Scroll down to ensure `MARGIN` lines below the current line are visible.
        let below_margin = self.content.saturate_line_index(line_index + MARGIN);
        let below_margin_start_index = (below_margin + 1).saturating_sub(view_rows);
        if self.view_start_line_index < below_margin_start_index {
            self.view_start_line_index = below_margin_start_index;
        }
    }

    pub fn path(&self) -> &Path {
        self.content.path()
    }

    pub fn read(&mut self) -> anyhow::Result<()> {
        self.content.read(&self.git)?;
        self.invalidate_render();
        self.scroll_current_line_into_view();
        Ok(())
    }

    fn swap_content(&mut self, content: &mut Box<BlameContent>) {
        std::mem::swap(&mut self.content, content);
        self.invalidate_render();
        self.scroll_current_line_into_view();
    }

    pub fn commit_id(&self) -> Oid {
        self.content.commit_id()
    }

    pub fn set_commit_id(&mut self, commit_id: Oid) -> anyhow::Result<()> {
        self.set_commit_id_core(commit_id, None, None)
    }

    fn set_commit_id_core(
        &mut self,
        commit_id: Oid,
        path: Option<&Path>,
        line_index: Option<usize>,
    ) -> anyhow::Result<()> {
        let mut content = if let Some(content) = self.cache.remove(&commit_id) {
            content
        } else {
            let mut content = Box::new(BlameContent::new(
                commit_id,
                path.unwrap_or_else(|| self.content.path()),
            ));
            content.read(&self.git)?;
            content
        };
        if let Some(line_index) = line_index {
            content.set_current_line_index(line_index);
        }
        self.swap_content(&mut content);
        if content.lines_len() > 0 {
            self.cache.insert(content.commit_id(), content);
        }
        Ok(())
    }

    pub fn set_commit_id_to_older_than_current_line(&mut self) -> anyhow::Result<()> {
        let diff_part = Rc::clone(&self.current_line().diff_part);
        let commit_id = self.git.older_commit_id(diff_part.commit_id())?;
        if commit_id.is_none() {
            bail!("No commits before {}", diff_part.commit_id());
        }
        let commit_id = commit_id.unwrap();

        let mut line_number = self.current_line_number();
        assert!(line_number >= diff_part.line_number.start);
        line_number = line_number - diff_part.line_number.start + diff_part.orig_start_line_number;
        let line_index = self.content.line_index_from_number(line_number);

        let path = diff_part.orig_path.as_deref();
        self.set_commit_id_core(commit_id, path, Some(line_index))
    }

    pub fn show_current_line_commit(&mut self, current_file_only: bool) -> anyhow::Result<()> {
        let commit_id = self.current_line_commit_id();
        self.git.show(
            commit_id,
            if current_file_only {
                Some(self.content.path())
            } else {
                None
            },
        )?;
        self.invalidate_render();
        Ok(())
    }

    pub fn invalidate_render(&mut self) {
        self.rendered_rows = 0;
    }

    pub fn render(&mut self, out: &mut impl Write) -> anyhow::Result<()> {
        if self.try_render_by_update(out)? {
            return Ok(());
        }

        queue!(out, terminal::Clear(terminal::ClearType::All))?;
        self.rendered_rows =
            self.render_line_index_range_unchecked(out, false, self.view_line_indexes())?;
        self.rendered_view_start_line_index = self.view_start_line_index;
        self.rendered_current_line_index = self.current_line_index();
        Ok(())
    }

    fn try_render_by_update(&mut self, out: &mut impl Write) -> anyhow::Result<bool> {
        if self.rendered_rows == 0 {
            return Ok(false);
        }

        if self.rendered_view_start_line_index != self.view_start_line_index {
            let view_start_line_index = self.view_start_line_index;
            let render_range = if view_start_line_index > self.rendered_view_start_line_index {
                let scroll_up = view_start_line_index - self.rendered_view_start_line_index;
                if scroll_up >= self.view_rows() as usize {
                    return Ok(false);
                }
                queue!(out, terminal::ScrollUp(scroll_up as u16))?;
                let view_end_line_index = self.view_end_line_index();
                view_end_line_index - scroll_up..view_end_line_index
            } else {
                let scroll_down = self.rendered_view_start_line_index - view_start_line_index;
                if scroll_down >= self.view_rows() as usize {
                    return Ok(false);
                }
                queue!(out, terminal::ScrollDown(scroll_down as u16))?;
                view_start_line_index..view_start_line_index + scroll_down
            };
            self.render_line_index_range_unchecked(out, true, render_range)?;
            self.rendered_view_start_line_index = self.view_start_line_index;
        }

        let current_line_index = self.current_line_index();
        if self.rendered_current_line_index != current_line_index {
            self.render_line_index(out, self.rendered_current_line_index)?;
            self.render_line_index(out, current_line_index)?;
            self.rendered_current_line_index = current_line_index;
        }
        Ok(true)
    }

    fn render_line_index(&self, out: &mut impl Write, line_index: usize) -> anyhow::Result<()> {
        self.render_line_index_range(out, true, line_index..line_index + 1)?;
        Ok(())
    }

    fn render_line_index_range(
        &self,
        out: &mut impl Write,
        should_clear_lines: bool,
        line_index_range: Range<usize>,
    ) -> anyhow::Result<u16> {
        let adjusted_range = self.adjust_line_index_range_into_view(&line_index_range);
        if adjusted_range.is_empty() {
            return Ok(0);
        }
        self.render_line_index_range_unchecked(out, should_clear_lines, adjusted_range)
    }

    fn render_line_index_range_unchecked(
        &self,
        out: &mut impl Write,
        should_clear_lines: bool,
        line_index_range: Range<usize>,
    ) -> anyhow::Result<u16> {
        assert!(!line_index_range.is_empty());
        assert!(line_index_range.start >= self.view_start_line_index);
        assert!(line_index_range.end <= self.view_end_line_index());
        let start_row = line_index_range.start - self.view_start_line_index;
        let lines = self
            .content
            .lines()
            .iter()
            .take(line_index_range.end)
            .skip(line_index_range.start);
        self.render_lines(out, start_row as u16, should_clear_lines, lines)
    }

    fn render_lines<'a, Iter>(
        &self,
        out: &mut impl Write,
        start_row: u16,
        should_clear_lines: bool,
        lines: Iter,
    ) -> anyhow::Result<u16>
    where
        Iter: Iterator<Item = &'a BlameLine>,
    {
        let mut row = start_row;
        let current_line_number = self.current_line_number();
        for line in lines {
            queue!(out, cursor::MoveTo(0, row))?;
            if should_clear_lines {
                queue!(out, terminal::Clear(terminal::ClearType::CurrentLine))?;
            }
            line.render(out, current_line_number)?;
            row += 1;
        }
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use crate::git_tools::tests::TempRepository;

    use super::*;

    #[test]
    fn scroll_current_line_into_view() -> anyhow::Result<()> {
        let git = TempRepository::new()?;
        let mut renderer = BlameRenderer::from_git_tools(git.git, Path::new("a"), (10, 10));

        // No adjustment needed.
        assert_eq!(adjust_start_line_index(&mut renderer, 0, 30, 0, 20), 0);

        // Need to scroll up.
        assert_eq!(adjust_start_line_index(&mut renderer, 14, 30, 0, 20), 0);
        assert_eq!(adjust_start_line_index(&mut renderer, 15, 30, 0, 20), 1);

        // Need to scroll down.
        assert_eq!(adjust_start_line_index(&mut renderer, 15, 30, 10, 20), 10);
        assert_eq!(adjust_start_line_index(&mut renderer, 14, 30, 10, 20), 9);

        // Content is updated to a smaller size. Ensure all view rows are filled.
        assert_eq!(adjust_start_line_index(&mut renderer, 14, 30, 5, 20), 5);
        assert_eq!(adjust_start_line_index(&mut renderer, 14, 21, 5, 20), 1);
        assert_eq!(adjust_start_line_index(&mut renderer, 14, 15, 5, 20), 0);
        Ok(())
    }

    fn adjust_start_line_index(
        renderer: &mut BlameRenderer,
        current_line_index: usize,
        lines_len: usize,
        start_line_index: usize,
        view_rows: u16,
    ) -> usize {
        renderer.content.set_lines_len_for_test(lines_len);
        renderer.set_view_size((10, view_rows));
        renderer.view_start_line_index = start_line_index;
        renderer.set_current_line_index(current_line_index);
        renderer.scroll_current_line_into_view();
        renderer.view_start_line_index
    }
}
