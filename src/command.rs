use std::io::{Write, stdout};

use crossterm::{cursor, event, queue, style, terminal};

use crate::CommandKeyMap;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Command {
    PrevDiff,
    NextDiff,
    PrevPage,
    NextPage,
    FirstLine,
    LastLine,
    Older,
    Newer,
    LineNumber(usize),
    Copy,
    ShowCommit,
    ShowDiff,
    Repaint,
    Resize(u16, u16),
    Help,
    Quit,
}

pub enum CommandPrompt {
    None,
    Message { message: String },
    Err { error: anyhow::Error },
}

impl Command {
    pub fn read(
        row: u16,
        key_map: &CommandKeyMap,
        prompt: &CommandPrompt,
    ) -> anyhow::Result<Command> {
        let mut buffer = String::new();
        loop {
            Self::show_prompt(row, prompt, &buffer)?;
            match event::read()? {
                event::Event::Key(event) => {
                    if let Some(command) = Self::handle_key(key_map, event, &mut buffer) {
                        return Ok(command);
                    }
                }
                event::Event::Resize(columns, rows) => return Ok(Command::Resize(columns, rows)),
                _ => {}
            }
        }
    }

    fn show_prompt(row: u16, prompt: &CommandPrompt, buffer: &str) -> anyhow::Result<()> {
        let mut out = stdout();
        queue!(
            out,
            cursor::MoveTo(0, row),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;
        let mut has_prompt = true;
        match prompt {
            CommandPrompt::None => has_prompt = false,
            CommandPrompt::Message { message } => queue!(out, style::Print(message.to_string()),)?,
            CommandPrompt::Err { error } => queue!(
                out,
                style::SetColors(style::Colors::new(style::Color::White, style::Color::Red)),
                style::Print(format!("{error}")),
                style::ResetColor
            )?,
        };
        if !has_prompt && buffer.is_empty() {
            queue!(
                out,
                cursor::MoveTo(1, row),
                style::SetForegroundColor(style::Color::DarkGrey),
                style::Print("h(elp), q(uit), Enter=parent, s(how), d(iff)"),
                style::ResetColor,
                cursor::MoveTo(0, row),
                style::Print(":".to_string())
            )?;
        } else {
            queue!(out, style::Print(format!(":{buffer}")))?;
        }
        out.flush()?;
        Ok(())
    }

    fn handle_key(
        key_map: &CommandKeyMap,
        event: event::KeyEvent,
        buffer: &mut String,
    ) -> Option<Command> {
        if event.is_release() {
            return None;
        }

        if !buffer.is_empty() {
            if let Some(command) = Self::handle_buffer_key(event, buffer) {
                return Some(command);
            }
            return None;
        }
        if let Some(command) = key_map.get(event.code, event.modifiers) {
            return Some(command.clone());
        }
        if let Some(command) = Self::handle_buffer_key(event, buffer) {
            return Some(command);
        }
        None
    }

    fn handle_buffer_key(event: event::KeyEvent, buffer: &mut String) -> Option<Command> {
        assert!(!event.is_release());

        match event.code {
            event::KeyCode::Char(ch) => buffer.push(ch),
            event::KeyCode::Enter => {
                if let Ok(number) = buffer.parse() {
                    return Some(Command::LineNumber(number));
                }
            }
            event::KeyCode::Backspace => {
                buffer.pop();
            }
            event::KeyCode::Esc => buffer.clear(),
            _ => {}
        }
        None
    }

    pub fn wait_for_any_key(message: &str) -> anyhow::Result<()> {
        let mut out = stdout();
        queue!(out, style::Print(message))?;
        out.flush()?;
        loop {
            if let event::Event::Key(event) = event::read()? {
                if !event.is_release() {
                    break;
                }
            }
        }
        Ok(())
    }
}
