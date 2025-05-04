use std::{
    io::stdout,
    path::{Path, PathBuf},
};

use crossterm::{clipboard::CopyToClipboard, cursor, execute, style, terminal};
use git2::Oid;

use crate::*;

#[derive(Debug, Default)]
/// The `git-iblame` command line interface.
/// # Examples
/// ```no_run
/// use git_iblame::Cli;
///
/// # use std::path::PathBuf;
/// fn main() -> anyhow::Result<()> {
///   let path = PathBuf::from("path/to/file");
///   let mut cli: Cli = Cli::new(&path);
///   cli.run()
/// }
/// ```
pub struct Cli {
    path: PathBuf,
}

impl Cli {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Run the `git-iblame` command line interface.
    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut renderer = BlameRenderer::new(&self.path)?;
        let size = terminal::size()?;
        renderer.set_view_size((size.0, size.1 - 1));
        renderer.read()?;

        let mut history: Vec<Oid> = vec![];
        let mut last_search: Option<String> = None;
        let mut out = stdout();
        let key_map = CommandKeyMap::new();
        let mut prompt: CommandPrompt = CommandPrompt::None;
        let mut terminal_raw_mode = TerminalRawModeScope::new(true)?;
        loop {
            renderer.render(&mut out)?;
            let command_rows = renderer.rendered_rows();

            let command = Command::read(command_rows, &key_map, &prompt)?;
            prompt = CommandPrompt::None;
            match command {
                Command::PrevDiff => renderer.move_to_prev_diff(),
                Command::NextDiff => renderer.move_to_next_diff(),
                Command::PrevPage => renderer.move_to_prev_page(),
                Command::NextPage => renderer.move_to_next_page(),
                Command::FirstLine => renderer.move_to_first_line(),
                Command::LastLine => renderer.move_to_last_line(),
                Command::LineNumber(number) => renderer.set_current_line_number(number),
                Command::Search(search) => {
                    renderer.search(&search, /*reverses*/ false);
                    last_search = Some(search);
                }
                Command::SearchPrev | Command::SearchNext => {
                    if let Some(search) = last_search.as_ref() {
                        renderer.search(search, command == Command::SearchPrev);
                    }
                }
                Command::Older => {
                    execute!(
                        out,
                        terminal::Clear(terminal::ClearType::All),
                        cursor::MoveTo(0, 0),
                        style::Print("Working...")
                    )?;
                    let path_before = renderer.path().to_path_buf();
                    let old_commit_id = renderer.commit_id();
                    if let Err(error) = renderer.set_commit_id_to_older_than_current_line() {
                        prompt = CommandPrompt::Err { error };
                        // Invalidate because the "working" message cleared the screen.
                        renderer.invalidate_render();
                        continue;
                    }
                    history.push(old_commit_id);
                    if path_before != renderer.path() {
                        prompt = CommandPrompt::Message {
                            message: format!("Path changed to {}", renderer.path().display()),
                        };
                    }
                }
                Command::Newer => {
                    if let Some(commit_id) = history.pop() {
                        renderer.set_commit_id(commit_id)?;
                    }
                }
                Command::Copy => {
                    execute!(
                        out,
                        CopyToClipboard::to_clipboard_from(
                            renderer.current_line_commit_id().to_string()
                        )
                    )?;
                    prompt = CommandPrompt::Message {
                        message: "Copied to clipboard".to_string(),
                    };
                }
                Command::ShowCommit | Command::ShowDiff => {
                    let mut terminal_raw_mode = TerminalRawModeScope::new(false)?;
                    renderer.show_current_line_commit(command == Command::ShowDiff)?;
                    terminal_raw_mode.reset()?;
                    Command::wait_for_any_key("Press any key to continue...")?;
                }
                Command::Help => {
                    execute!(
                        out,
                        terminal::Clear(terminal::ClearType::All),
                        cursor::MoveTo(0, 0),
                    )?;
                    renderer.invalidate_render();
                    let mut terminal_raw_mode = TerminalRawModeScope::new(false)?;
                    key_map.print_help();
                    println!();
                    terminal_raw_mode.reset()?;
                    Command::wait_for_any_key("Press any key to continue...")?;
                }
                Command::Repaint => renderer.invalidate_render(),
                Command::Resize(columns, rows) => renderer.set_view_size((columns, rows - 1)),
                Command::Quit => break,
            }
        }

        terminal_raw_mode.reset()?;
        Ok(())
    }
}
