use std::io::ErrorKind;
use std::process::{Command, Output};

use base64::Engine;
use serde::Deserialize;

use crate::error::ClyncError;
use crate::Result;

fn map_gh_error(e: std::io::Error) -> ClyncError {
    if e.kind() == ErrorKind::NotFound {
        ClyncError::GhNotInstalled
    } else {
        ClyncError::GitHubApi(format!("Failed to run gh: {}", e))
    }
}

fn run_gh(args: &[&str]) -> Result<Output> {
    Command::new("gh")
        .args(args)
        .output()
        .map_err(map_gh_error)
}

fn run_gh_with_extra_args(args: &[&str], extra: &[String]) -> Result<Output> {
    Command::new("gh")
        .args(args)
        .args(extra)
        .output()
        .map_err(map_gh_error)
}

fn check_gh_error(output: &Output) -> Option<ClyncError> {
    if output.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Some(ClyncError::GitHubApi(stderr.to_string()))
}

fn is_not_found(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains("Not Found")
}

pub struct GitHubClient {
    pub repo: String,
}

#[derive(Debug, Deserialize)]
pub struct RepoContent {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub sha: Option<String>,
    pub content: Option<String>,
}

impl GitHubClient {
    pub fn new(repo: &str) -> Self {
        Self {
            repo: repo.to_string(),
        }
    }

    pub fn list_files(&self, path: &str) -> Result<Vec<RepoContent>> {
        let api_path = if path.is_empty() {
            format!("repos/{}/contents", self.repo)
        } else {
            format!("repos/{}/contents/{}", self.repo, path)
        };

        let output = run_gh(&["api", &api_path])?;

        if !output.status.success() {
            if is_not_found(&output) {
                return Ok(vec![]);
            }
            return Err(check_gh_error(&output).unwrap());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let contents: Vec<RepoContent> = serde_json::from_str(&stdout)
            .or_else(|_| {
                let single: RepoContent = serde_json::from_str(&stdout)?;
                Ok::<_, serde_json::Error>(vec![single])
            })
            .map_err(|e| ClyncError::GitHubApi(format!("Failed to parse response: {}", e)))?;

        Ok(contents)
    }

    pub fn list_files_recursive(&self, path: &str) -> Result<Vec<RepoContent>> {
        let mut all_files = Vec::new();
        let contents = self.list_files(path)?;

        for item in contents {
            if item.content_type == "dir" {
                let sub_files = self.list_files_recursive(&item.path)?;
                all_files.extend(sub_files);
            } else if item.content_type == "file" {
                all_files.push(item);
            }
        }

        Ok(all_files)
    }

    pub fn get_file_content(&self, path: &str) -> Result<Option<(String, String)>> {
        let api_path = format!("repos/{}/contents/{}", self.repo, path);

        let output = run_gh(&["api", &api_path])?;

        if !output.status.success() {
            if is_not_found(&output) {
                return Ok(None);
            }
            return Err(check_gh_error(&output).unwrap());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let content: RepoContent = serde_json::from_str(&stdout)
            .map_err(|e| ClyncError::GitHubApi(format!("Failed to parse response: {}", e)))?;

        let Some(encoded) = content.content else {
            return Ok(None);
        };

        let encoded_clean = encoded.replace('\n', "");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded_clean)
            .map_err(|e| ClyncError::GitHubApi(format!("Base64 decode failed: {}", e)))?;

        let text = String::from_utf8(decoded)
            .map_err(|e| ClyncError::GitHubApi(format!("UTF-8 decode failed: {}", e)))?;

        let sha = content.sha.unwrap_or_default();

        Ok(Some((text, sha)))
    }

    pub fn put_file(&self, path: &str, content: &str, sha: Option<&str>, message: &str) -> Result<()> {
        let api_path = format!("repos/{}/contents/{}", self.repo, path);
        let encoded = base64::engine::general_purpose::STANDARD.encode(content);
        let message_arg = format!("message={}", message);
        let content_arg = format!("content={}", encoded);

        let base_args = ["api", "-X", "PUT", &api_path, "-f", &message_arg, "-f", &content_arg];

        let output = if let Some(s) = sha {
            let sha_arg = format!("sha={}", s);
            run_gh_with_extra_args(&base_args, &["-f".to_string(), sha_arg])?
        } else {
            run_gh(&base_args)?
        };

        if let Some(err) = check_gh_error(&output) {
            return Err(err);
        }

        Ok(())
    }

    pub fn delete_file(&self, path: &str, sha: &str, message: &str) -> Result<()> {
        let api_path = format!("repos/{}/contents/{}", self.repo, path);
        let message_arg = format!("message={}", message);
        let sha_arg = format!("sha={}", sha);

        let output = run_gh(&["api", "-X", "DELETE", &api_path, "-f", &message_arg, "-f", &sha_arg])?;

        if let Some(err) = check_gh_error(&output) {
            return Err(err);
        }

        Ok(())
    }
}
