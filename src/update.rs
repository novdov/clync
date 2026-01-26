use crate::{ClyncError, Result};
use console::style;
use std::env;

const REPO_OWNER: &str = "novdov";
const REPO_NAME: &str = "clync";

pub fn self_update() -> Result<()> {
    let token = env::var("GITHUB_TOKEN").map_err(|_| {
        ClyncError::Update(
            "GITHUB_TOKEN environment variable required.\n\
             Create token: https://github.com/settings/tokens (needs repo permission)"
                .to_string(),
        )
    })?;

    let current_version = env!("CARGO_PKG_VERSION");
    println!(
        "Current version: {}",
        style(format!("v{}", current_version)).cyan()
    );
    println!("Checking for updates...");

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("clync")
        .current_version(current_version)
        .auth_token(&token)
        .build()
        .map_err(|e| ClyncError::Update(e.to_string()))?
        .update()
        .map_err(|e| ClyncError::Update(e.to_string()))?;

    match status {
        self_update::Status::UpToDate(v) => {
            println!("{} Already up to date (v{})", style("✓").green(), v);
        }
        self_update::Status::Updated(v) => {
            println!("{} Updated to v{}!", style("✓").green(), v);
        }
    }

    Ok(())
}
