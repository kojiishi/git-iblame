use std::{
    io::{Write, stdout},
    time::Duration,
};

use crossterm::{event, queue, style};
use log::debug;

use super::*;

#[derive(Debug, Default)]
pub struct CommandUI {
    pub prompt: CommandPrompt,
    pub buffer: String,
    pub key_map: CommandKeyMap,
    pub timeout: Duration,
}

impl CommandUI {
    pub fn new() -> Self {
        Self {
            key_map: CommandKeyMap::new(),
            ..Default::default()
        }
    }

    pub fn read(&mut self, row: u16) -> anyhow::Result<Command> {
        loop {
            self.prompt.show(row, &self.buffer)?;
            if !self.timeout.is_zero() && !event::poll(self.timeout)? {
                return Ok(Command::Timeout);
            }
            match event::read()? {
                event::Event::Key(event) => {
                    if let Some(command) = self.handle_key(event) {
                        return Ok(command);
                    }
                }
                event::Event::Resize(columns, rows) => return Ok(Command::Resize(columns, rows)),
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, event: event::KeyEvent) -> Option<Command> {
        if event.is_release() {
            return None;
        }

        if !self.buffer.is_empty() {
            if let Some(command) = self.handle_buffer_key(event) {
                return Some(command);
            }
            return None;
        }
        if let Some(command) = self.key_map.get(event.code, event.modifiers) {
            return Some(command.clone());
        }
        if let Some(command) = self.handle_buffer_key(event) {
            return Some(command);
        }
        None
    }

    fn handle_buffer_key(&mut self, event: event::KeyEvent) -> Option<Command> {
        assert!(!event.is_release());

        match event.code {
            event::KeyCode::Char(ch) => {
                if !self.buffer.is_empty() || ch == '/' || ch.is_ascii_digit() {
                    self.buffer.push(ch);
                }
            }
            event::KeyCode::Enter => {
                if let Ok(number) = self.buffer.parse() {
                    self.buffer.clear();
                    return Some(Command::LineNumber(number));
                }
                if let Some(search) = self.buffer.strip_prefix('/') {
                    let search = search.to_string();
                    self.buffer.clear();
                    return Some(Command::Search(search));
                }
            }
            event::KeyCode::Backspace => {
                self.buffer.pop();
            }
            event::KeyCode::Esc => self.buffer.clear(),
            _ => {}
        }
        None
    }

    pub fn set_error(&mut self, error: anyhow::Error) {
        self.prompt = CommandPrompt::Err { error };
    }

    pub fn set_result(&mut self, result: anyhow::Result<()>) {
        if let Err(error) = result {
            debug!("set_result: error: {error:?}");
            self.set_error(error);
        }
    }

    pub fn set_prompt(&mut self, message: String) {
        self.prompt = CommandPrompt::Message { message };
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
