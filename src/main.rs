use std::{env, path::PathBuf};

use git_iblame::*;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let path = PathBuf::from(&args[1]);
    let mut cli: Cli = Cli::new(&path);
    cli.run()
}
