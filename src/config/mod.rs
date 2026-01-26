pub mod loader;
pub mod model;

pub use loader::{load_config, save_config};
pub use model::{Config, SyncMode, Whitelist};

use crate::Result;
use console::style;

pub fn show() -> Result<()> {
    let config = load_config()?;

    println!("{}", style("Claudy Configuration").bold());
    println!();

    if let Some(repo) = &config.repo {
        println!("  Repository: {}", style(repo).cyan());
    } else {
        println!("  Repository: {}", style("(not configured)").dim());
    }

    println!("  Sync mode: {}", style(format!("{:?}", config.sync_mode)).cyan());
    println!();

    println!("{}", style("Whitelist").bold());
    if config.whitelist.paths.is_empty() {
        println!("  {}", style("(empty)").dim());
    } else {
        for path in &config.whitelist.paths {
            println!("  - {}", path);
        }
    }

    Ok(())
}

pub fn set_repo(repo: Option<String>) -> Result<()> {
    let mut config = load_config()?;

    match repo {
        Some(repo) => {
            println!("Repository set to {}", style(&repo).cyan());
            config.repo = Some(repo);
            save_config(&config)?;
        }
        None => {
            if let Some(repo) = &config.repo {
                println!("Current repository: {}", style(repo).cyan());
            } else {
                println!("{}", style("Repository not configured").dim());
            }
        }
    }

    Ok(())
}

pub fn whitelist_list() -> Result<()> {
    let config = load_config()?;

    println!("{}", style("Whitelist").bold());
    if config.whitelist.paths.is_empty() {
        println!("  {}", style("(empty)").dim());
    } else {
        for path in &config.whitelist.paths {
            println!("  - {}", path);
        }
    }

    Ok(())
}

pub fn whitelist_add(path: &str) -> Result<()> {
    let mut config = load_config()?;

    if config.whitelist.paths.contains(&path.to_string()) {
        println!("{} is already in the whitelist", style(path).yellow());
        return Ok(());
    }

    config.whitelist.paths.push(path.to_string());
    save_config(&config)?;
    println!("Added {} to whitelist", style(path).green());

    Ok(())
}

pub fn whitelist_remove(path: &str) -> Result<()> {
    let mut config = load_config()?;

    if let Some(pos) = config.whitelist.paths.iter().position(|p| p == path) {
        config.whitelist.paths.remove(pos);
        save_config(&config)?;
        println!("Removed {} from whitelist", style(path).green());
    } else {
        println!("{} is not in the whitelist", style(path).yellow());
    }

    Ok(())
}
