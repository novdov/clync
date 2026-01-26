use console::style;
use dialoguer::Select;

use crate::backup::BackupManager;
use crate::config::{load_config, SyncMode};
use crate::error::ClyncError;
use crate::github::GitHubClient;
use crate::whitelist::WhitelistMatcher;
use crate::Result;

use super::diff::{compute_diff, FileDiff, FileStatus};

pub fn execute(dry_run: bool, force: bool) -> Result<()> {
    let config = load_config()?;
    let repo = config.repo.as_ref().ok_or(ClyncError::RepoNotConfigured)?;

    if config.whitelist.paths.is_empty() && config.sync_mode == SyncMode::Whitelist {
        return Err(ClyncError::EmptyWhitelist);
    }

    let client = GitHubClient::new(repo);
    let matcher = WhitelistMatcher::new(&config.whitelist.paths);

    let diffs = compute_diff(&client, &matcher, &config.sync_mode)?;

    let to_pull: Vec<&FileDiff> = diffs
        .iter()
        .filter(|d| matches!(d.status, FileStatus::RemoteOnly | FileStatus::Modified))
        .collect();

    if to_pull.is_empty() {
        println!("{}", style("Nothing to pull").green());
        return Ok(());
    }

    println!("{}", style("Files to pull:").bold());
    for diff in &to_pull {
        let prefix = match diff.status {
            FileStatus::RemoteOnly => style("+").green(),
            FileStatus::Modified => style("M").yellow(),
            _ => continue,
        };
        println!("  {} {}", prefix, diff.path);
    }
    println!();

    if dry_run {
        println!(
            "{}",
            style("[dry-run] No actual pull was made").dim()
        );
        return Ok(());
    }

    let files_to_backup: Vec<String> = to_pull
        .iter()
        .filter(|d| d.local_content.is_some())
        .map(|d| d.path.clone())
        .collect();

    if !files_to_backup.is_empty() {
        let backup_manager = BackupManager::new();
        let backup_path = backup_manager.create_backup(&files_to_backup)?;
        println!(
            "{}",
            style(format!("Backup created: {}", backup_path.display())).dim()
        );
        println!();
    }

    for diff in &to_pull {
        if diff.status == FileStatus::Modified && !force {
            println!("{}", style(format!("Conflict: {}", diff.path)).yellow().bold());
            println!();
            println!("{}", diff.format_diff());
            println!();

            let choices = vec!["Overwrite with remote", "Keep local", "Skip"];
            let selection = Select::new()
                .with_prompt("How would you like to proceed?")
                .items(&choices)
                .default(0)
                .interact()
                .unwrap_or(2);

            match selection {
                0 => {}
                1 => {
                    println!("  {} {} ... {}", style("→").cyan(), diff.path, style("skipped (keeping local)").dim());
                    continue;
                }
                _ => {
                    println!("  {} {} ... {}", style("→").cyan(), diff.path, style("skipped").dim());
                    continue;
                }
            }
        }

        let content = diff.remote_content.as_ref().ok_or_else(|| {
            ClyncError::FileRead(format!("Cannot read remote file: {}", diff.path))
        })?;

        print!("  {} {} ... ", style("←").cyan(), diff.path);

        matcher.write_local_file(&diff.path, content)?;

        println!("{}", style("done").green());
    }

    println!();
    println!(
        "{}",
        style("✓ Pull complete").green()
    );

    Ok(())
}
