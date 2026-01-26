use console::style;

use crate::config::{load_config, SyncMode};
use crate::error::ClaudyError;
use crate::github::GitHubClient;
use crate::whitelist::WhitelistMatcher;
use crate::Result;

use super::diff::{compute_diff, FileStatus};

pub fn get_status() -> Result<StatusSummary> {
    let config = load_config()?;
    let repo = config.repo.as_ref().ok_or(ClaudyError::RepoNotConfigured)?;

    let client = GitHubClient::new(repo);
    let matcher = WhitelistMatcher::new(&config.whitelist.paths);

    let diffs = compute_diff(&client, &matcher, &config.sync_mode)?;

    let mut summary = StatusSummary::default();

    for diff in diffs {
        match diff.status {
            FileStatus::Same => summary.synced += 1,
            FileStatus::LocalOnly => summary.local_only.push(diff.path),
            FileStatus::RemoteOnly => summary.remote_only.push(diff.path),
            FileStatus::Modified => summary.modified.push(diff.path),
        }
    }

    Ok(summary)
}

#[derive(Default)]
pub struct StatusSummary {
    pub synced: usize,
    pub local_only: Vec<String>,
    pub remote_only: Vec<String>,
    pub modified: Vec<String>,
}

impl StatusSummary {
    pub fn is_synced(&self) -> bool {
        self.local_only.is_empty() && self.remote_only.is_empty() && self.modified.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.local_only.len() + self.remote_only.len() + self.modified.len()
    }
}

pub fn show_status() -> Result<()> {
    let config = load_config()?;

    println!("{}", style("Claudy Status").bold());
    println!();

    if let Some(repo) = &config.repo {
        println!("  Repository: {}", style(repo).cyan());
    } else {
        println!("  Repository: {}", style("(not configured)").red());
        println!();
        println!(
            "{}",
            style("Set repository with 'claudy config repo <owner/repo>'").dim()
        );
        return Ok(());
    }

    println!("  Sync mode: {:?}", config.sync_mode);
    println!();

    if config.whitelist.paths.is_empty() && config.sync_mode == SyncMode::Whitelist {
        println!(
            "{}",
            style("Whitelist is empty. Add with 'claudy config whitelist add <path>'").yellow()
        );
        return Ok(());
    }

    let summary = get_status()?;

    if summary.is_synced() {
        println!("{}", style("✓ Local and remote are in sync").green());
        println!("  Synced files: {}", summary.synced);
        return Ok(());
    }

    println!("{}", style("Sync status:").bold());

    if !summary.local_only.is_empty() {
        println!();
        println!(
            "  {} ({}):",
            style("Local only").green(),
            summary.local_only.len()
        );
        for path in &summary.local_only {
            println!("    + {}", path);
        }
    }

    if !summary.remote_only.is_empty() {
        println!();
        println!(
            "  {} ({}):",
            style("Remote only").red(),
            summary.remote_only.len()
        );
        for path in &summary.remote_only {
            println!("    - {}", path);
        }
    }

    if !summary.modified.is_empty() {
        println!();
        println!(
            "  {} ({}):",
            style("Modified").yellow(),
            summary.modified.len()
        );
        for path in &summary.modified {
            println!("    M {}", path);
        }
    }

    println!();
    println!(
        "  Total {} file(s) changed, {} file(s) synced",
        summary.total_changes(),
        summary.synced
    );

    Ok(())
}
