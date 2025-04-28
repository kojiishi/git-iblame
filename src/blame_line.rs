use crossterm::{cursor, queue, style};
use std::{fmt, io::Write, rc::Rc};

use crate::*;

#[derive(Debug, Default)]
pub struct BlameLine {
    line_number: usize,
    line: String,
    pub diff_part: Rc<DiffPart>,
}

impl BlameLine {
    pub fn new(line_number: usize, line: &str) -> Self {
        Self {
            line_number,
            line: line.to_string(),
            ..Default::default()
        }
    }

    pub fn render(
        &self,
        out: &mut impl Write,
        row: u16,
        current_line_number: usize,
    ) -> anyhow::Result<()> {
        queue!(out, cursor::MoveTo(0, row))?;
        let mut should_reset = false;
        if self.line_number == current_line_number {
            queue!(
                out,
                style::SetForegroundColor(style::Color::Black),
                style::SetBackgroundColor(style::Color::Cyan)
            )?;
            should_reset = true;
        }

        if self.line_number == self.diff_part.range.end - 1 {
            queue!(out, style::SetAttribute(style::Attribute::Underlined))?;
            should_reset = true;
        }

        let blame_index = self.line_number - self.diff_part.range.start;
        let blame = match blame_index {
            0 => GitTools::to_local_date_time(self.diff_part.when).map_or_else(
                |e| format!("Invalid date/time: {e}"),
                |datetime| datetime.format("%Y-%m-%d %H:%M %Z").to_string(),
            ),
            1 => format!("  {} {}", self.diff_part.email, self.diff_part.name),
            2 => format!("  {}", self.diff_part.commit_id),
            _ => String::new(),
        };
        let left_side = format!("{number:4}:{blame:25.25}|", number = self.line_number);
        queue!(out, style::Print(left_side))?;

        if should_reset {
            queue!(
                out,
                style::ResetColor,
                style::SetAttribute(style::Attribute::Reset)
            )?;
        }

        queue!(out, style::Print(&self.line))?;

        Ok(())
    }
}

impl fmt::Display for BlameLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.line_number, self.line)
    }
}
