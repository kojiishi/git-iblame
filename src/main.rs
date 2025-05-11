use std::path::PathBuf;

use clap::Parser;
use git_iblame::ui::Cli;

/// Interactive enhanced `git blame` command line tool.
#[derive(Debug, Default, Parser)]
#[command(version, about)]
struct Args {
    /// Path of the file to blame.
    path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let options = Args::parse();
    let mut cli: Cli = Cli::new(&options.path);
    cli.run()
}
