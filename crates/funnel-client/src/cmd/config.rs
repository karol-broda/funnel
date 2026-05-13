use crate::config;

#[derive(clap::Subcommand)]
pub enum Command {
    /// show current configuration
    Show,
    /// print config file path
    Path,
}

pub fn run(command: &Command) -> anyhow::Result<()> {
    match command {
        Command::Show => {
            let cfg = config::load()?;
            let content = toml::to_string_pretty(&cfg)?;
            println!("# {}\n", config::config_path().display());
            println!("{content}");
            Ok(())
        }
        Command::Path => {
            println!("{}", config::config_path().display());
            Ok(())
        }
    }
}
