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
            let cfg = config::load_effective()?;
            let content = toml::to_string_pretty(&cfg)?;
            println!("# user config: {}", config::config_path().display());
            match config::project_config_path() {
                Some(path) => println!("# project config: {}", path.display()),
                None => println!("# project config: <none>"),
            }
            println!();
            println!("{content}");
            Ok(())
        }
        Command::Path => {
            println!("{}", config::config_path().display());
            Ok(())
        }
    }
}
