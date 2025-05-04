use std::{
    io::{Write, stdout},
    time::Duration,
};

use crossterm::{event, queue, style};

use crate::{CommandKeyMap, CommandPrompt};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Command {
    PrevLine,
    NextLine,
    PrevPage,
    NextPage,
    FirstLine,
    LastLine,
    Older,
    Newer,
    LineNumber(usize),
    Search(String),
    SearchPrev,
    SearchNext,
    Copy,
    ShowCommit,
    ShowDiff,
    Repaint,
    Resize(u16, u16),
    Help,
    Quit,
    Timeout,
}

impl Command {
    pub fn read(
        row: u16,
        key_map: &CommandKeyMap,
        prompt: &CommandPrompt,
        timeout: Duration,
    ) -> anyhow::Result<Command> {
        let mut buffer = String::new();
        loop {
            prompt.show(row, &buffer)?;
            if !timeout.is_zero() && !event::poll(timeout)? {
                return Ok(Command::Timeout);
            }
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
            event::KeyCode::Char(ch) => {
                if !buffer.is_empty() || ch == '/' || ch.is_ascii_digit() {
                    buffer.push(ch);
                }
            }
            event::KeyCode::Enter => {
                if let Ok(number) = buffer.parse() {
                    return Some(Command::LineNumber(number));
                }
                if let Some(search) = buffer.strip_prefix('/') {
                    return Some(Command::Search(search.to_string()));
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
