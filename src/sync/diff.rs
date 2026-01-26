use std::collections::HashSet;

use console::style;
use similar::{ChangeTag, TextDiff};

use crate::config::{load_config, SyncMode};
use crate::error::ClaudyError;
use crate::github::GitHubClient;
use crate::whitelist::WhitelistMatcher;
use crate::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Same,
    LocalOnly,
    RemoteOnly,
    Modified,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub local_content: Option<String>,
    pub remote_content: Option<String>,
    pub remote_sha: Option<String>,
}

impl FileDiff {
    pub fn format_diff(&self) -> String {
        match self.status {
            FileStatus::Same => String::new(),
            FileStatus::LocalOnly => {
                format!("{} {}", style("+").green(), style(&self.path).green())
            }
            FileStatus::RemoteOnly => {
                format!("{} {}", style("-").red(), style(&self.path).red())
            }
            FileStatus::Modified => {
                let mut output = format!("{} {}\n", style("M").yellow(), style(&self.path).yellow());

                if let (Some(local), Some(remote)) = (&self.local_content, &self.remote_content) {
                    let diff = TextDiff::from_lines(remote, local);

                    for change in diff.iter_all_changes() {
                        let (sign, style_fn): (&str, fn(&str) -> console::StyledObject<&str>) = match change.tag() {
                            ChangeTag::Delete => ("-", |s| style(s).red()),
                            ChangeTag::Insert => ("+", |s| style(s).green()),
                            ChangeTag::Equal => (" ", |s| style(s).dim()),
                        };
                        output.push_str(&format!("  {}{}", sign, style_fn(change.value())));
                    }
                }

                output
            }
        }
    }
}

pub fn compute_diff(client: &GitHubClient, matcher: &WhitelistMatcher, sync_mode: &SyncMode) -> Result<Vec<FileDiff>> {
    let local_files: HashSet<String> = matcher.list_local_files()?.into_iter().collect();

    let remote_files: HashSet<String> = client
        .list_files_recursive("")?
        .into_iter()
        .filter(|f| sync_mode == &SyncMode::Remote || matcher.matches(&f.path))
        .map(|f| f.path)
        .collect();

    let all_files: HashSet<String> = local_files.union(&remote_files).cloned().collect();

    let mut diffs = Vec::new();

    for path in all_files {
        let local_content = matcher.read_local_file(&path)?;
        let remote_result = client.get_file_content(&path)?;

        let (remote_content, remote_sha) = match remote_result {
            Some((content, sha)) => (Some(content), Some(sha)),
            None => (None, None),
        };

        let status = match (&local_content, &remote_content) {
            (None, None) => continue,
            (Some(_), None) => FileStatus::LocalOnly,
            (None, Some(_)) => FileStatus::RemoteOnly,
            (Some(local), Some(remote)) => {
                if local == remote {
                    FileStatus::Same
                } else {
                    FileStatus::Modified
                }
            }
        };

        diffs.push(FileDiff {
            path,
            status,
            local_content,
            remote_content,
            remote_sha,
        });
    }

    diffs.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(diffs)
}

pub fn show_diff() -> Result<()> {
    let config = load_config()?;
    let repo = config.repo.as_ref().ok_or(ClaudyError::RepoNotConfigured)?;

    if config.whitelist.paths.is_empty() && config.sync_mode == SyncMode::Whitelist {
        return Err(ClaudyError::EmptyWhitelist);
    }

    let client = GitHubClient::new(repo);
    let matcher = WhitelistMatcher::new(&config.whitelist.paths);

    let diffs = compute_diff(&client, &matcher, &config.sync_mode)?;

    let has_changes = diffs.iter().any(|d| d.status != FileStatus::Same);

    if !has_changes {
        println!("{}", style("Local and remote are in sync").green());
        return Ok(());
    }

    for diff in &diffs {
        if diff.status != FileStatus::Same {
            println!("{}", diff.format_diff());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_display() {
        let diff = FileDiff {
            path: "test.md".to_string(),
            status: FileStatus::LocalOnly,
            local_content: Some("content".to_string()),
            remote_content: None,
            remote_sha: None,
        };

        let output = diff.format_diff();
        assert!(output.contains("test.md"));
    }
}
