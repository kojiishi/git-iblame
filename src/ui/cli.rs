use std::{
    io::stdout,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;
use crossterm::{clipboard::CopyToClipboard, cursor, execute, terminal};
use git2::Oid;
use log::debug;

use crate::{blame::FileHistory, extensions::TerminalRawModeScope};

use super::*;

/// Interactive enhanced `git blame` command line tool.
#[derive(Debug, Default, Parser)]
#[command(version, about)]
struct Args {
    /// Use git2 to compute the commit diff.
    #[arg(long, default_value_t = false)]
    diff_git2: bool,

    /// Path of the file to annotate the history.
    path: PathBuf,
}

#[derive(Debug, Default)]
/// The `git-iblame` command line interface.
/// # Examples
/// ```no_run
/// use git_iblame::ui::Cli;
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
    history: Vec<Oid>,
    last_search: Option<String>,
}

impl Cli {
    pub fn new_from_args() -> Self {
        let args = Args::parse();
        if args.diff_git2 {
            crate::blame::FileCommit::use_git2();
        }
        Self {
            path: args.path,
            ..Default::default()
        }
    }

    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            ..Default::default()
        }
    }

    /// Run the `git-iblame` command line interface.
    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut history = FileHistory::new(&self.path);
        history.read_start()?;

        let mut renderer = BlameRenderer::new(history)?;
        let size = terminal::size()?;
        renderer.set_view_size((size.0, size.1 - 1));

        let mut ui = CommandUI::new();
        let mut out = stdout();
        let mut terminal_raw_mode = TerminalRawModeScope::new_with_alternate_screen()?;
        loop {
            let result = renderer.render(&mut out);
            ui.set_result(result);
            let command_rows = renderer.rendered_rows();

            if renderer.history().is_reading() {
                ui.timeout = Duration::from_millis(1000);
                if matches!(ui.prompt, CommandPrompt::None) {
                    ui.prompt = CommandPrompt::Loading;
                }
            } else {
                ui.timeout = Duration::ZERO;
                if matches!(ui.prompt, CommandPrompt::Loading) {
                    ui.prompt = CommandPrompt::None;
                }
            }
            let command = ui.read(command_rows)?;
            match command {
                Command::Quit => break,
                Command::Timeout => {}
                _ => ui.prompt = CommandPrompt::None,
            }
            let result = self.handle_command(command, &mut renderer, &mut ui);
            ui.set_result(result);
        }

        terminal_raw_mode.reset()?;
        Ok(())
    }

    fn handle_command(
        &mut self,
        command: Command,
        renderer: &mut BlameRenderer,
        ui: &mut CommandUI,
    ) -> anyhow::Result<()> {
        let mut out = stdout();
        match command {
            Command::PrevLine => renderer.move_to_prev_line_by(1),
            Command::NextLine => renderer.move_to_next_line_by(1),
            // Command::PrevDiff => renderer.move_to_prev_diff(),
            // Command::NextDiff => renderer.move_to_next_diff(),
            Command::PrevPage => renderer.move_to_prev_page(),
            Command::NextPage => renderer.move_to_next_page(),
            Command::FirstLine => renderer.move_to_first_line(),
            Command::LastLine => renderer.move_to_last_line(),
            Command::LineNumber(number) => renderer.set_current_line_number(number)?,
            Command::Search(search) => {
                renderer.search(&search, /*reverses*/ false);
                self.last_search = Some(search);
            }
            Command::SearchPrev | Command::SearchNext => {
                if let Some(search) = self.last_search.as_ref() {
                    renderer.search(search, command == Command::SearchPrev);
                }
            }
            Command::Older => {
                let path_before = renderer.path().to_path_buf();
                let old_commit_id = renderer.commit_id();
                renderer.set_commit_id_to_older_than_current_line()?;
                if !old_commit_id.is_zero() {
                    self.history.push(old_commit_id);
                }
                if path_before != renderer.path() {
                    ui.set_prompt(format!("Path changed to {}", renderer.path().display()));
                }
            }
            Command::Newer => {
                if let Some(commit_id) = self.history.pop() {
                    let path_before = renderer.path().to_path_buf();
                    renderer.set_commit_id(commit_id)?;
                    if path_before != renderer.path() {
                        ui.set_prompt(format!("Path changed to {}", renderer.path().display()));
                    }
                }
            }
            Command::Log => {
                let old_commit_id = renderer.commit_id();
                renderer.set_log_content()?;
                if !old_commit_id.is_zero() {
                    self.history.push(old_commit_id);
                }
            }
            Command::Copy => {
                if let Ok(commit_id) = renderer.current_line_commit_id() {
                    execute!(
                        out,
                        CopyToClipboard::to_clipboard_from(commit_id.to_string())
                    )?;
                    ui.set_prompt("Copied to clipboard".to_string());
                }
            }
            Command::ShowCommit | Command::ShowDiff => {
                let mut terminal_raw_mode = TerminalRawModeScope::new(false)?;
                renderer.show_current_line_commit(command == Command::ShowDiff)?;
                terminal_raw_mode.reset()?;
                CommandUI::wait_for_any_key("Press any key to continue...")?;
            }
            Command::Help => {
                execute!(
                    out,
                    terminal::Clear(terminal::ClearType::All),
                    cursor::MoveTo(0, 0),
                )?;
                renderer.invalidate_render();
                let mut terminal_raw_mode = TerminalRawModeScope::new(false)?;
                ui.key_map.print_help();
                println!();
                terminal_raw_mode.reset()?;
                CommandUI::wait_for_any_key("Press any key to continue...")?;
            }
            Command::Timeout => renderer.read_poll()?,
            Command::Repaint => renderer.invalidate_render(),
            Command::Resize(columns, rows) => renderer.set_view_size((columns, rows - 1)),
            Command::Debug => {
                let commit_id = renderer.current_line_commit_id()?;
                let commit = renderer.history().commits().get_by_commit_id(commit_id)?;
                debug!("debug_current_line: {commit:?}");
            }
            Command::Quit => {}
        }
        Ok(())
    }
}
