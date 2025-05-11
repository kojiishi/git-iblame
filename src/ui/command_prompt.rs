use std::{
    io::{Write, stdout},
    time::{SystemTime, UNIX_EPOCH},
};

use crossterm::{cursor, queue, style, terminal};

#[derive(Debug, Default)]
pub enum CommandPrompt {
    #[default]
    None,
    Loading,
    Message {
        message: String,
    },
    Err {
        error: anyhow::Error,
    },
}

impl CommandPrompt {
    pub fn show(&self, row: u16, buffer: &str) -> anyhow::Result<()> {
        let mut out = stdout();
        queue!(
            out,
            cursor::MoveTo(0, row),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;
        let mut suppress_help = false;
        match self {
            CommandPrompt::None => {}
            CommandPrompt::Loading => {
                let icon = Self::loading_indicator()?;
                queue!(out, style::Print(icon.to_string()),)?;
            }
            CommandPrompt::Message { message } => {
                queue!(out, style::Print(message.to_string()),)?;
                suppress_help = true;
            }
            CommandPrompt::Err { error } => {
                let error_message = error.to_string();
                queue!(
                    out,
                    style::SetColors(style::Colors::new(style::Color::White, style::Color::Red)),
                    style::Print(error_message),
                    style::ResetColor
                )?;
                suppress_help = true;
            }
        }
        if buffer.starts_with('/') {
            queue!(out, style::Print(buffer))?;
        } else {
            queue!(out, style::Print(format!(":{buffer}")))?;
            if !suppress_help && buffer.is_empty() {
                queue!(
                    out,
                    cursor::SavePosition,
                    style::SetForegroundColor(style::Color::DarkGrey),
                    style::Print("h(elp), q(uit), Right=parent, s(how), d(iff)"),
                    style::ResetColor,
                    cursor::RestorePosition,
                )?;
            }
        }
        out.flush()?;
        Ok(())
    }

    const ICON_CYCLE: &str = r"-\|/";

    fn loading_indicator() -> anyhow::Result<char> {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH)?;
        let index = (duration.as_secs() % (Self::ICON_CYCLE.len() as u64)) as usize;
        let icon = Self::ICON_CYCLE.chars().nth(index).unwrap();
        Ok(icon)
    }
}
