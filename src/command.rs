use std::io::{Write, stdout};

use crossterm::{cursor, event, queue, style, terminal};

#[derive(Debug)]
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
    Resize(u16, u16),
    Quit,
}

pub enum CommandPrompt {
    None,
    Message { message: String },
    Err { error: anyhow::Error },
}

impl Command {
    pub fn read(row: u16, prompt: &CommandPrompt) -> anyhow::Result<Command> {
        let mut out = stdout();
        let mut buffer = String::new();
        loop {
            queue!(
                out,
                cursor::MoveTo(0, row),
                terminal::Clear(terminal::ClearType::CurrentLine),
            )?;
            let mut has_prompt = true;
            match prompt {
                CommandPrompt::None => has_prompt = false,
                CommandPrompt::Message { message } => {
                    queue!(out, style::Print(format!("{message}")),)?
                }
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
                    style::Print("Enter=drill down, BS=back, q(uit), c(opy SHA1)"),
                    style::ResetColor,
                    cursor::MoveTo(0, row),
                    style::Print(format!(":"))
                )?;
            } else {
                queue!(out, style::Print(format!(":{buffer}")))?;
            }
            out.flush()?;

            match event::read()? {
                event::Event::Key(event) => {
                    if event.is_release() {
                        continue;
                    }
                    match event.code {
                        event::KeyCode::Char(ch) => {
                            if buffer.len() > 0 {
                                buffer.push(ch);
                                continue;
                            }
                            if event.modifiers & event::KeyModifiers::CONTROL
                                != event::KeyModifiers::NONE
                            {
                                match ch {
                                    // `vi`, `emacs`, or `less`-like key bindings.
                                    'b' => return Ok(Command::PrevPage),
                                    'f' => return Ok(Command::NextPage),
                                    'n' => return Ok(Command::NextDiff),
                                    'p' => return Ok(Command::PrevDiff),
                                    _ => continue,
                                }
                            }
                            match ch {
                                'c' => return Ok(Command::Copy),
                                'q' => return Ok(Command::Quit),
                                // `vi`, `emacs`, or `less`-like key bindings.
                                'b' => return Ok(Command::PrevPage),
                                'f' => return Ok(Command::NextPage),
                                'j' => return Ok(Command::NextDiff),
                                'k' => return Ok(Command::PrevDiff),
                                _ => {}
                            }
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
                        event::KeyCode::PageUp => return Ok(Command::PrevPage),
                        event::KeyCode::PageDown => return Ok(Command::NextPage),
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
