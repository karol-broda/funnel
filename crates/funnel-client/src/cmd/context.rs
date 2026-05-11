use crate::config;

#[derive(clap::Subcommand)]
pub enum Command {
    /// list all contexts
    List,
    /// switch to a different context
    Use {
        /// context name to switch to
        name: String,
    },
    /// create a new context
    Create {
        /// context name
        name: String,
        /// server url
        #[arg(long)]
        server: String,
    },
    /// delete a context
    Delete {
        /// context name to delete
        name: String,
    },
}

pub fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::List => {
            let cfg = config::load()?;
            if cfg.contexts.is_empty() {
                println!("no contexts configured");
                println!("  create one with: funnel context create <name> --server <url>");
                return Ok(());
            }
            for (name, ctx) in &cfg.contexts {
                let marker = if name == &cfg.current_context {
                    " *"
                } else {
                    ""
                };
                let token_status = if ctx.token.is_some() {
                    "authenticated"
                } else {
                    "no token"
                };
                println!("{name}{marker}");
                println!("  server: {}", ctx.server);
                println!("  status: {token_status}");
                println!();
            }
            Ok(())
        }
        Command::Use { name } => {
            config::set_current_context(&name)?;
            println!("switched to context '{name}'");
            Ok(())
        }
        Command::Create { name, server } => {
            config::create_context(&name, &server)?;
            println!("created context '{name}' ({server})");
            Ok(())
        }
        Command::Delete { name } => {
            config::delete_context(&name)?;
            println!("deleted context '{name}'");
            Ok(())
        }
    }
}
