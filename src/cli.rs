use std::{io::stdout, path::Path};

use crate::*;
use crossterm::{clipboard::CopyToClipboard, cursor, execute, style, terminal};
use git2::Oid;

/// The `git-iblame` command line interface.
/// # Examples
/// ```no_run
/// use git_iblame::Cli;
///
/// # use std::path::PathBuf;
/// fn main() -> anyhow::Result<()> {
///   let path = PathBuf::from("path/to/repo");
///   let mut cli: Cli = Cli::new(&path)?;
///   cli.run()
/// }
/// ```
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
        let mut terminal_raw_mode = TerminalRawModeScope::new(true)?;

        let renderer = &mut self.renderer;
        let size = terminal::size()?;
        renderer.set_view_size((size.0, size.1 - 1));
        renderer.read()?;
        let mut history: Vec<Oid> = vec![];
        let mut out = stdout();
        let mut prompt: CommandPrompt = CommandPrompt::None;
        loop {
            renderer.render(&mut out)?;

            let command = Command::read(renderer.rendered_rows(), &prompt)?;
            prompt = CommandPrompt::None;
            match command {
                Command::PrevDiff => renderer.move_to_prev_diff(),
                Command::NextDiff => renderer.move_to_next_diff(),
                Command::PrevPage => renderer.move_to_prev_page(),
                Command::NextPage => renderer.move_to_next_page(),
                Command::FirstLine => renderer.move_to_first_line(),
                Command::LastLine => renderer.move_to_last_line(),
                Command::LineNumber(number) => renderer.set_current_line_number(number),
                Command::Older => {
                    execute!(
                        out,
                        terminal::Clear(terminal::ClearType::All),
                        cursor::MoveTo(0, 0),
                        style::Print("Working...")
                    )?;
                    let old_commit_id = renderer.commit_id();
                    if let Err(error) = renderer.set_commit_id_to_older_than_current_line() {
                        prompt = CommandPrompt::Err { error };
                        // Invalidate because the "working" message cleared the screen.
                        renderer.invalidate_render();
                        continue;
                    }
                    history.push(old_commit_id);
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

        terminal_raw_mode.reset();
        Ok(())
    }
}
