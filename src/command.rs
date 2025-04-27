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
    Older,
    Newer,
    LineNumber(usize),
    Copy,
    Resize(u16, u16),
    Quit,
}

impl Command {
    pub fn read(row: u16, prompt: &str) -> anyhow::Result<Command> {
        let mut out = stdout();
        let mut buffer = String::new();
        loop {
            queue!(
                out,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine),
                style::Print(format!("{prompt}:{buffer}"))
            )?;
            out.flush()?;

            match event::read()? {
                event::Event::Key(event) => {
                    if event.is_release() {
                        continue;
                    }
                    match event.code {
                        event::KeyCode::Char(ch) => {
                            if buffer.len() == 0 {
                                match ch {
                                    'c' => return Ok(Command::Copy),
                                    'q' => return Ok(Command::Quit),
                                    _ => {}
                                }
                            }
                            buffer.push(ch);
                        }
                        event::KeyCode::Enter => {
                            if buffer.len() == 0 {
                                return Ok(Command::Older);
                            }
                            if let Ok(number) = buffer.parse() {
                                return Ok(Command::LineNumber(number));
                            }
                        }
                        event::KeyCode::Backspace => {
                            if buffer.len() == 0 {
                                return Ok(Command::Newer);
                            }
                            buffer.pop();
                        }
                        event::KeyCode::Esc => buffer.clear(),
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
