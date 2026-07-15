use std::{borrow::Cow, fmt, io::Write};

use crossterm::{queue, style};
use git2_time_chrono_ext::Git2TimeChronoExt;
use unicode_width_utils::UnicodeWidth;

use super::{FileCommit, FileHistory};
use crate::extensions::OrDefault;

#[derive(Debug, Default, Eq, PartialEq)]
enum LineType {
    #[default]
    Line,
    Deleted,
    Log,
}

#[derive(Debug, Default)]
pub struct Line {
    line_type: LineType,
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

    pub fn new_deleted(line_number: usize, commit_id: git2::Oid) -> Self {
        Self {
            line_type: LineType::Deleted,
            line_number,
            // content: "#deleted#".to_string(),
            commit_id: Some(commit_id),
            ..Default::default()
        }
    }

    pub fn new_log(commit: &FileCommit) -> Self {
        Self {
            line_type: LineType::Log,
            line_number: commit.index(),
            content: commit.summary().cloned().unwrap_or_default(),
            commit_id: Some(commit.commit_id()),
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
        history: &FileHistory,
        is_current_line: bool,
        constraint: &LineConstraint,
    ) -> anyhow::Result<()> {
        let commit = self
            .commit_id
            .map(|commit_id| history.commits().get_by_commit_id(commit_id))
            .transpose()?;
        let mut should_reset = false;
        if is_current_line {
            queue!(
                out,
                style::SetColors(style::Colors::new(style::Color::Black, style::Color::Cyan)),
            )?;
            should_reset = true;
        } else if let Some(commit) = commit
            && commit.is_apply_failed()
        {
            queue!(
                out,
                style::SetColors(style::Colors::new(style::Color::Red, style::Color::Black)),
            )?;
            should_reset = true;
        }

        if self.is_last_line_in_hunk {
            queue!(out, style::SetAttribute(style::Attribute::Underlined))?;
            should_reset = true;
        }

        let blame = self.left_pane(commit)?;
        let left_pane = match self.line_type {
            LineType::Line | LineType::Log => format!("{:4}:{blame:25.25}|", self.line_number),
            LineType::Deleted => format!("    :{blame:25.25}|"),
        };
        let left_pane_len = left_pane.len();
        queue!(out, style::Print(left_pane))?;

        if should_reset {
            queue!(
                out,
                style::ResetColor,
                style::SetAttribute(style::Attribute::Reset)
            )?;
        }

        match self.line_type {
            LineType::Line | LineType::Log => {
                let content = constraint.truncate(&self.content, left_pane_len);
                queue!(out, style::Print(content))?;
            }
            LineType::Deleted => {
                let content = "##deleted##";
                queue!(
                    out,
                    style::SetForegroundColor(style::Color::DarkGrey),
                    style::Print(content),
                    style::ResetColor,
                )?;
            }
        }
        Ok(())
    }

    fn left_pane(&self, commit: Option<&FileCommit>) -> anyhow::Result<Cow<'static, str>> {
        let left_pane = if let Some(commit) = commit {
            match self.index_in_hunk {
                0 => {
                    let datetime = commit.time().to_local_date_time().map_or_else(
                        |e| format!("Invalid date/time: {e}"),
                        |datetime| datetime.format("%Y-%m-%d %H:%M").to_string(),
                    );
                    match self.line_type {
                        LineType::Line | LineType::Deleted => {
                            format!("#{} {}", commit.index(), datetime)
                        }
                        LineType::Log => {
                            format!("{} {}", datetime, commit.author_email())
                        }
                    }
                    .into()
                }
                1 => commit.summary().map(|s| format!("  {s}")).or_default(),
                2 => format!("  {}", commit.author_email()).into(),
                3 => format!("  {}", commit.commit_id()).into(),
                _ => "".into(),
            }
        } else {
            "...".into()
        };
        Ok(left_pane)
    }
}

pub(crate) struct LineConstraint {
    max_columns: usize,
    uw: UnicodeWidth,
}

impl LineConstraint {
    const TAB_SIZE: u8 = 4;

    pub(crate) fn new(max_columns: usize) -> Self {
        let mut uw = UnicodeWidth::new();
        uw.set_tab_size(Self::TAB_SIZE);
        uw.set_expand_tab(true);
        Self { max_columns, uw }
    }

    fn truncate<'a>(&self, input: &'a str, margin: usize) -> Cow<'a, str> {
        let max_columns = self.max_columns.saturating_sub(margin);
        self.uw.truncate(input, max_columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate() {
        let c = LineConstraint::new(5);
        assert_eq!(c.truncate("abc", 0), "abc");
        assert_eq!(c.truncate("abc", 3), "ab");
        assert_eq!(c.truncate("abc", 5), "");
    }

    #[test]
    fn truncate_tab() {
        let c = LineConstraint::new(5);
        assert_eq!(c.truncate("\t", 1), "    ");
        assert_eq!(c.truncate("\t", 2), "");

        assert_eq!(c.truncate("a\t", 0), "a   ");
        assert_eq!(c.truncate("a\t", 1), "a   ");
        assert_eq!(c.truncate("a\t", 2), "a");

        assert_eq!(c.truncate("123\t", 0), "123 ");
        let c = LineConstraint::new(10);
        assert_eq!(c.truncate("1234\t", 0), "1234    ");
    }

    #[test]
    fn truncate_wide() {
        let c = LineConstraint::new(11);
        assert_eq!(c.truncate("あいうえお", 0), "あいうえお");
        assert_eq!(c.truncate("あいうえお", 1), "あいうえお");
        assert_eq!(c.truncate("あいうえお", 2), "あいうえ");
        assert_eq!(c.truncate("あいうえお", 3), "あいうえ");
        assert_eq!(c.truncate("あいうえお", 9), "あ");
        assert_eq!(c.truncate("あいうえお", 10), "");
        assert_eq!(c.truncate("あいうえお", 11), "");
    }
}
