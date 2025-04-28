use std::{io::stdout, path::Path};

use crate::*;
use crossterm::{clipboard::CopyToClipboard, execute, terminal};
use git2::Oid;
use log::*;

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
        terminal::enable_raw_mode()?;
        let result = self.run_core();
        if let Err(e) = terminal::disable_raw_mode() {
            warn!("Failed to disable raw mode, ignored: {e}");
        }
        result
    }

    fn run_core(&mut self) -> anyhow::Result<()> {
        let renderer = &mut self.renderer;
        let size = terminal::size()?;
        renderer.set_view_size((size.0, size.1 - 1));
        renderer.read()?;
        let mut history: Vec<Oid> = vec![];
        let mut out = stdout();
        let prompt = String::new();
        loop {
            renderer.render(&mut out)?;

            let command = Command::read(renderer.rendered_rows(), &prompt)?;
            match command {
                Command::PrevDiff => renderer.move_to_prev_diff(),
                Command::NextDiff => renderer.move_to_next_diff(),
                Command::PageUp => renderer.move_to_prev_page(),
                Command::PageDown => renderer.move_to_next_page(),
                Command::FirstLine => renderer.move_to_first_line(),
                Command::LastLine => renderer.move_to_last_line(),
                Command::LineNumber(number) => renderer.set_current_line_number(number),
                Command::Older => {
                    history.push(renderer.commit_id());
                    renderer.set_commit_id_to_older_than_current_line()?;
                }
                Command::Newer => {
                    if let Some(commit_id) = history.pop() {
                        renderer.set_commit_id(commit_id)?;
                    }
                }
                Command::Copy => execute!(
                    out,
                    CopyToClipboard::to_clipboard_from(
                        renderer.current_line_commit_id().to_string()
                    )
                )?,
                Command::Resize(columns, rows) => renderer.set_view_size((columns, rows - 1)),
                Command::Quit => break,
            }
        }
        Ok(())
    }
}
