use std::{fmt, io::Write};

use crossterm::{queue, style};

use crate::Git2TimeToChronoExt;

use super::FileHistory;

#[derive(Debug, Default)]
pub struct Line {
    line_number: usize,
    content: String,
    commit_id: Option<git2::Oid>,
    index_in_hunk: usize,
    is_last_line_in_hunk: bool,
}

impl Line {
    pub fn new(line_number: usize, content: String) -> Self {
        Self {
            line_number,
            content,
            ..Default::default()
        }
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn commit_id(&self) -> Option<git2::Oid> {
        self.commit_id
    }

    pub fn set_commit_id(&mut self, commit_id: git2::Oid) {
        self.commit_id = Some(commit_id);
    }

    pub fn clear_commit_id(&mut self) {
        self.commit_id = None;
    }

    pub fn set_index_in_hunk(&mut self, index_in_hunk: usize) {
        self.index_in_hunk = index_in_hunk;
    }

    pub fn set_is_last_line_in_hunk(&mut self, value: bool) {
        self.is_last_line_in_hunk = value;
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.line_number, self.content)
    }
}

impl Line {
    pub fn render(
        &self,
        out: &mut impl Write,
        current_line_number: usize,
        history: &FileHistory,
        max_columns: usize,
    ) -> anyhow::Result<()> {
        let mut should_reset = false;
        if self.line_number == current_line_number {
            queue!(
                out,
                style::SetForegroundColor(style::Color::Black),
                style::SetBackgroundColor(style::Color::Cyan)
            )?;
            should_reset = true;
        }

        if self.is_last_line_in_hunk {
            queue!(out, style::SetAttribute(style::Attribute::Underlined))?;
            should_reset = true;
        }

        let blame = if let Some(commit_id) = self.commit_id {
            let commit = history.commit_from_commit_id(commit_id)?;
            match self.index_in_hunk {
                0 => format!(
                    "#{} {}",
                    commit.index(),
                    commit.time().to_local_date_time().map_or_else(
                        |e| format!("Invalid date/time: {e}"),
                        |datetime| datetime.format("%Y-%m-%d %H:%M %Z").to_string(),
                    )
                ),
                1 => commit
                    .summary()
                    .map_or(String::new(), |s| format!("  {}", s)),
                2 => commit
                    .author()
                    .map_or(String::new(), |s| format!("  {}", s)),
                3 => format!("  {}", commit.commit_id()),
                _ => String::new(),
            }
        } else {
            "...".to_string()
        };
        let left_pane = format!("{number:4}:{blame:25.25}|", number = self.line_number);
        let left_pane_len = left_pane.len();
        queue!(out, style::Print(left_pane))?;

        if should_reset {
            queue!(
                out,
                style::ResetColor,
                style::SetAttribute(style::Attribute::Reset)
            )?;
        }

        let max_main_pane = max_columns.saturating_sub(left_pane_len);
        let mut content = self.content.as_str();
        content = match content.char_indices().nth(max_main_pane) {
            None => content,
            Some((idx, _)) => &content[..idx],
        };
        queue!(out, style::Print(content))?;

        Ok(())
    }
}
