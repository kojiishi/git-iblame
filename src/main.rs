use std::{env, path::PathBuf};

use git_iblame::Cli;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = PathBuf::from(&args[1]);
    let mut cli: Cli = Cli::new(&path)?;
    cli.run()
}
