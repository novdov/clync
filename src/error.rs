use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClyncError {
    #[error("GitHub CLI (gh) is not installed. Install from https://cli.github.com")]
    GhNotInstalled,

    #[error("GitHub CLI authentication required. Run 'gh auth login'")]
    NotAuthenticated,

    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    #[error("Config file not found: {0}")]
    ConfigNotFound(String),

    #[error("Config parse error: {0}")]
    ConfigParse(String),

    #[error("Repository not configured. Set with 'clync config repo <owner/repo>'")]
    RepoNotConfigured,

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("File read error: {0}")]
    FileRead(String),

    #[error("File write error: {0}")]
    FileWrite(String),

    #[error("Whitelist is empty")]
    EmptyWhitelist,

    #[error("Conflict occurred: {0}")]
    Conflict(String),

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("Update error: {0}")]
    Update(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ClyncError>;
