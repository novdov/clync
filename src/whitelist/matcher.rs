use std::fs;

use glob::Pattern;
use sha1::{Digest, Sha1};
use walkdir::WalkDir;

use crate::config::loader::claude_dir;
use crate::Result;

pub fn compute_git_blob_sha(content: &[u8]) -> String {
    let header = format!("blob {}\0", content.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

pub struct WhitelistMatcher {
    patterns: Vec<Pattern>,
}

impl WhitelistMatcher {
    pub fn new(paths: &[String]) -> Self {
        let patterns = paths
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect();

        Self { patterns }
    }

    pub fn matches(&self, path: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(path))
    }

    pub fn list_local_files(&self) -> Result<Vec<String>> {
        let base = claude_dir();
        let mut matched = Vec::new();

        if !base.exists() {
            return Ok(matched);
        }

        for entry in WalkDir::new(&base).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(relative) = entry.path().strip_prefix(&base) {
                    let path_str = relative.to_string_lossy().into_owned();
                    if self.matches(&path_str) {
                        matched.push(path_str);
                    }
                }
            }
        }

        Ok(matched)
    }

    pub fn read_local_file(&self, relative_path: &str) -> Result<Option<String>> {
        let full_path = claude_dir().join(relative_path);

        if !full_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&full_path)
            .map_err(|e| crate::ClyncError::FileRead(format!("{}: {}", relative_path, e)))?;

        Ok(Some(content))
    }

    pub fn read_local_file_with_sha(&self, relative_path: &str) -> Result<Option<(String, String)>> {
        let full_path = claude_dir().join(relative_path);

        if !full_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&full_path)
            .map_err(|e| crate::ClyncError::FileRead(format!("{}: {}", relative_path, e)))?;

        let sha = compute_git_blob_sha(&bytes);

        let content = String::from_utf8(bytes)
            .map_err(|e| crate::ClyncError::FileRead(format!("{}: {}", relative_path, e)))?;

        Ok(Some((content, sha)))
    }

    pub fn write_local_file(&self, relative_path: &str, content: &str) -> Result<()> {
        let full_path = claude_dir().join(relative_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)
            .map_err(|e| crate::ClyncError::FileWrite(format!("{}: {}", relative_path, e)))?;

        Ok(())
    }

    pub fn delete_local_file(&self, relative_path: &str) -> Result<()> {
        let full_path = claude_dir().join(relative_path);

        if full_path.exists() {
            fs::remove_file(&full_path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let matcher = WhitelistMatcher::new(&["settings.json".to_string()]);
        assert!(matcher.matches("settings.json"));
        assert!(!matcher.matches("other.json"));
    }

    #[test]
    fn test_glob_pattern() {
        let matcher = WhitelistMatcher::new(&["commands/**/*.md".to_string()]);
        assert!(matcher.matches("commands/git/commit.md"));
        assert!(matcher.matches("commands/test.md"));
        assert!(!matcher.matches("settings.json"));
    }

    #[test]
    fn test_multiple_patterns() {
        let matcher = WhitelistMatcher::new(&[
            "settings.json".to_string(),
            "CLAUDE.md".to_string(),
            "skills/**/*.md".to_string(),
        ]);
        assert!(matcher.matches("settings.json"));
        assert!(matcher.matches("CLAUDE.md"));
        assert!(matcher.matches("skills/coding/rust.md"));
        assert!(!matcher.matches("random.txt"));
    }
}
