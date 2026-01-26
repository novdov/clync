use std::io::ErrorKind;
use std::process::Command;

use crate::error::ClyncError;

pub fn check_auth() -> crate::Result<()> {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                ClyncError::GhNotInstalled
            } else {
                ClyncError::GitHubApi(format!("Failed to run gh: {}", e))
            }
        })?;

    if !output.status.success() {
        return Err(ClyncError::NotAuthenticated);
    }

    Ok(())
}
