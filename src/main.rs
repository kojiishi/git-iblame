use git_iblame::ui::Cli;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut cli: Cli = Cli::new_from_args();
    cli.run()
}
