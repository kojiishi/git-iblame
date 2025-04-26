use crossterm::{cursor, event, queue, style, terminal};
use std::io::{Write, stdout};

#[derive(Debug)]
pub enum Command {
    PrevDiff,
    NextDiff,
    PageUp,
    PageDown,
    FirstLine,
    LastLine,
    Deep,
    LineNumber(usize),
    Copy,
    Resize(u16, u16),
    Quit,
}

impl Command {
    pub fn read(row: u16) -> anyhow::Result<Command> {
        let mut out = stdout();
        let mut command = String::new();
        loop {
            queue!(
                out,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine),
                style::Print(format!(":{command}"))
            )?;
            out.flush()?;
            match event::read()? {
                event::Event::Key(event) => {
                    if event.is_release() {
                        continue;
                    }
                    match event.code {
                        event::KeyCode::Char(ch) => {
                            if command.len() == 0 {
                                match ch {
                                    'c' => return Ok(Command::Copy),
                                    'q' => return Ok(Command::Quit),
                                    _ => {}
                                }
                            }
                            command.push(ch);
                        }
                        event::KeyCode::Enter => {
                            if command.len() == 0 {
                                return Ok(Command::Deep);
                            }
                            if let Ok(number) = command.parse() {
                                return Ok(Command::LineNumber(number));
                            }
                        }
                        event::KeyCode::Backspace => {
                            if command.len() > 0 {
                                command.pop();
                            }
                        }
                        event::KeyCode::Esc => command.clear(),
                        event::KeyCode::Up => return Ok(Command::PrevDiff),
                        event::KeyCode::Down => return Ok(Command::NextDiff),
                        event::KeyCode::PageUp => return Ok(Command::PageUp),
                        event::KeyCode::PageDown => return Ok(Command::PageDown),
                        event::KeyCode::Home => return Ok(Command::FirstLine),
                        event::KeyCode::End => return Ok(Command::LastLine),
                        _ => {}
                    }
                }
                event::Event::Resize(columns, rows) => return Ok(Command::Resize(columns, rows)),
                _ => {}
            }
        }
    }
}
