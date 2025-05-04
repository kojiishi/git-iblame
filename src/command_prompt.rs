use std::io::{Write, stdout};

use crossterm::{cursor, queue, style, terminal};

pub enum CommandPrompt {
    None,
    Message { message: String },
    Err { error: anyhow::Error },
}

impl CommandPrompt {
    pub fn show(&self, row: u16, buffer: &str) -> anyhow::Result<()> {
        let mut out = stdout();
        queue!(
            out,
            cursor::MoveTo(0, row),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;
        let mut has_prompt = true;
        match self {
            CommandPrompt::None => has_prompt = false,
            CommandPrompt::Message { message } => queue!(out, style::Print(message.to_string()),)?,
            CommandPrompt::Err { error } => queue!(
                out,
                style::SetColors(style::Colors::new(style::Color::White, style::Color::Red)),
                style::Print(error.to_string()),
                style::ResetColor
            )?,
        };
        if !has_prompt && buffer.is_empty() {
            queue!(
                out,
                cursor::MoveTo(1, row),
                style::SetForegroundColor(style::Color::DarkGrey),
                style::Print("h(elp), q(uit), Right=parent, s(how), d(iff)"),
                style::ResetColor,
                cursor::MoveTo(0, row),
                style::Print(":".to_string())
            )?;
        } else if buffer.starts_with('/') {
            queue!(out, style::Print(buffer))?;
        } else {
            queue!(out, style::Print(format!(":{buffer}")))?;
        }
        out.flush()?;
        Ok(())
    }
}
