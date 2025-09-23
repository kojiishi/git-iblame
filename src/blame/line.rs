use std::{borrow::Cow, fmt, io::Write};

use crossterm::{queue, style};

use crate::extensions::{Git2TimeToChronoExt, OrDefault};

use super::{FileCommit, FileHistory};

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
        max_columns: usize,
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

        let max_main_pane = max_columns.saturating_sub(left_pane_len);
        match self.line_type {
            LineType::Line | LineType::Log => {
                let mut content = self.content.as_str();
                content = match content.char_indices().nth(max_main_pane) {
                    None => content,
                    Some((idx, _)) => &content[..idx],
                };
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
                            format!("{} {}", datetime, commit.author_email().or_default())
                        }
                    }
                    .into()
                }
                1 => commit.summary().map(|s| format!("  {s}")).or_default(),
                2 => commit.author_email().map(|s| format!("  {s}")).or_default(),
                3 => format!("  {}", commit.commit_id()).into(),
                _ => "".into(),
            }
        } else {
            "...".into()
        };
        Ok(left_pane)
    }
}
