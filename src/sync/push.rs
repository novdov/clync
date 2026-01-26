use console::style;
use dialoguer::Confirm;

use crate::config::{load_config, SyncMode};
use crate::error::ClaudyError;
use crate::github::GitHubClient;
use crate::whitelist::WhitelistMatcher;
use crate::Result;

use super::diff::{compute_diff, FileDiff, FileStatus};

pub fn execute(dry_run: bool, force: bool) -> Result<()> {
    let config = load_config()?;
    let repo = config.repo.as_ref().ok_or(ClaudyError::RepoNotConfigured)?;

    if config.whitelist.paths.is_empty() && config.sync_mode == SyncMode::Whitelist {
        return Err(ClaudyError::EmptyWhitelist);
    }

    let client = GitHubClient::new(repo);
    let matcher = WhitelistMatcher::new(&config.whitelist.paths);

    let diffs = compute_diff(&client, &matcher, &config.sync_mode)?;

    let to_push: Vec<&FileDiff> = diffs
        .iter()
        .filter(|d| matches!(d.status, FileStatus::LocalOnly | FileStatus::Modified))
        .collect();

    if to_push.is_empty() {
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

        if has_conflicts {
            println!(
                "{}",
                style("⚠ Files already exist on remote. Overwrite?").yellow()
            );

            let confirmed = Confirm::new()
                .with_prompt("Continue?")
                .default(false)
                .interact()
                .unwrap_or(false);

            if !confirmed {
                return Err(ClaudyError::UserCancelled);
            }
        }
    }

    for diff in &to_push {
        let content = diff.local_content.as_ref().ok_or_else(|| {
            ClaudyError::FileRead(format!("Cannot read local file: {}", diff.path))
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

    println!();
    println!(
        "{}",
        style(format!("✓ Pushed {} file(s)", to_push.len())).green()
    );

    Ok(())
}
