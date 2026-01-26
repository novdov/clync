use clap::Parser;
use claudy::cli::{Cli, Commands, ConfigCommands, WhitelistCommands};
use claudy::config;
use claudy::github;
use claudy::sync;
use claudy::update;
use claudy::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Push { dry_run, force } => {
            github::check_auth()?;
            sync::push(dry_run, force)?;
        }
        Commands::Pull { dry_run, force } => {
            github::check_auth()?;
            sync::pull(dry_run, force)?;
        }
        Commands::Diff => {
            github::check_auth()?;
            sync::diff()?;
        }
        Commands::Status => {
            github::check_auth()?;
            sync::status()?;
        }
        Commands::Config { command } => match command {
            ConfigCommands::Show => {
                config::show()?;
            }
            ConfigCommands::Repo { repo } => {
                config::set_repo(repo)?;
            }
            ConfigCommands::Whitelist { command } => match command {
                WhitelistCommands::List => {
                    config::whitelist_list()?;
                }
                WhitelistCommands::Add { path } => {
                    config::whitelist_add(&path)?;
                }
                WhitelistCommands::Remove { path } => {
                    config::whitelist_remove(&path)?;
                }
            },
        },
        Commands::SelfUpdate => {
            update::self_update()?;
        }
    }

    Ok(())
}
