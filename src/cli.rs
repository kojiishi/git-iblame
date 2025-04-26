use std::{io::stdout, path::Path};

use crate::*;
use crossterm::{clipboard::CopyToClipboard, execute, terminal};

pub struct Cli {
    renderer: BlameRenderer,
}

impl Cli {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            renderer: BlameRenderer::new(path)?,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let renderer = &mut self.renderer;
        let size = terminal::size()?;
        renderer.set_size((size.0, size.1 - 1));
        renderer.read_file()?;
        let mut out = stdout();
        loop {
            renderer.render(&mut out)?;
            let command = Command::read(renderer.rendered_rows())?;
            match command {
                Command::PrevDiff => renderer.move_to_prev_diff(),
                Command::NextDiff => renderer.move_to_next_diff(),
                Command::PageUp => renderer.move_to_prev_page(),
                Command::PageDown => renderer.move_to_next_page(),
                Command::FirstLine => renderer.move_to_first_line(),
                Command::LastLine => renderer.move_to_last_line(),
                Command::LineNumber(number) => renderer.set_current_number(number),
                Command::Deep => renderer.set_newest_commit_id_to_older()?,
                Command::Copy => execute!(
                    out,
                    CopyToClipboard::to_clipboard_from(renderer.current_commit_id().to_string())
                )?,
                Command::Resize(columns, rows) => renderer.set_size((columns, rows - 1)),
                Command::Quit => break,
            }
        }
        Ok(())
    }
}
