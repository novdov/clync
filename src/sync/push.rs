use console::style;
use dialoguer::Confirm;

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

    let to_push: Vec<&FileDiff> = diffs
        .iter()
        .filter(|d| matches!(d.status, FileStatus::LocalOnly | FileStatus::Modified))
        .collect();

    let to_delete: Vec<&FileDiff> = diffs
        .iter()
        .filter(|d| d.status == FileStatus::RemoteOnly)
        .collect();

    if to_push.is_empty() && to_delete.is_empty() {
        println!("{}", style("Nothing to push").green());
        return Ok(());
    }

    println!("{}", style("Files to push:").bold());
    for diff in &to_push {
        let prefix = match diff.status {
            FileStatus::LocalOnly => style("+").green(),
            FileStatus::Modified => style("M").yellow(),
            _ => continue,
        };
        println!("  {} {}", prefix, diff.path);
    }
    for diff in &to_delete {
        println!("  {} {}", style("-").red(), style(&diff.path).red());
    }
    println!();

    if dry_run {
        println!(
            "{}",
            style("[dry-run] No actual push was made").dim()
        );
        return Ok(());
    }

    if !force {
        let has_conflicts = to_push.iter().any(|d| d.status == FileStatus::Modified);

        if has_conflicts || !to_delete.is_empty() {
            if has_conflicts {
                println!(
                    "{}",
                    style("⚠ Files already exist on remote. Overwrite?").yellow()
                );
            }
            if !to_delete.is_empty() {
                println!(
                    "{}",
                    style(format!(
                        "⚠ {} file(s) will be deleted from remote.",
                        to_delete.len()
                    ))
                    .yellow()
                );
            }

            let confirmed = Confirm::new()
                .with_prompt("Continue?")
                .default(false)
                .interact()
                .unwrap_or(false);

            if !confirmed {
                return Err(ClyncError::UserCancelled);
            }
        }
    }

    for diff in &to_push {
        let content = diff.local_content.as_ref().ok_or_else(|| {
            ClyncError::FileRead(format!("Cannot read local file: {}", diff.path))
        })?;

        let message = match diff.status {
            FileStatus::LocalOnly => format!("Add {}", diff.path),
            FileStatus::Modified => format!("Update {}", diff.path),
            _ => continue,
        };

        let sha = diff.remote_sha.as_deref();

        print!("  {} {} ... ", style("→").cyan(), diff.path);

        client.put_file(&diff.path, content, sha, &message)?;

        println!("{}", style("done").green());
    }

    for diff in &to_delete {
        let sha = diff.remote_sha.as_deref().ok_or_else(|| {
            ClyncError::FileRead(format!("Missing remote SHA for: {}", diff.path))
        })?;

        print!("  {} {} ... ", style("×").red(), diff.path);

        client.delete_file(&diff.path, sha, &format!("Delete {}", diff.path))?;

        println!("{}", style("done").green());
    }

    println!();
    let pushed = to_push.len();
    let deleted = to_delete.len();
    let summary = match (pushed, deleted) {
        (0, d) => format!("✓ Deleted {} file(s)", d),
        (p, 0) => format!("✓ Pushed {} file(s)", p),
        (p, d) => format!("✓ Pushed {} file(s), deleted {} file(s)", p, d),
    };
    println!("{}", style(summary).green());

    Ok(())
}
