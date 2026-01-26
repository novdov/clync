use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clync")]
#[command(about = "Claude Code config sync tool")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Push local config to remote repository
    Push {
        /// Show expected result without making changes
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Pull config from remote repository
    Pull {
        /// Show expected result without making changes
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Show differences between local and remote
    Diff,

    /// Check sync status
    Status,

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Update to latest version
    SelfUpdate,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set remote repository
    Repo {
        /// Repository (owner/repo format)
        repo: Option<String>,
    },

    /// Manage whitelist
    Whitelist {
        #[command(subcommand)]
        command: WhitelistCommands,
    },
}

#[derive(Subcommand)]
pub enum WhitelistCommands {
    /// Show whitelist
    List,

    /// Add path to whitelist
    Add {
        /// Path to add (supports glob patterns)
        path: String,
    },

    /// Remove path from whitelist
    Remove {
        /// Path to remove
        path: String,
    },
}
