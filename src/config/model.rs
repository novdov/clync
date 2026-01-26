use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub repo: Option<String>,
    #[serde(default)]
    pub sync_mode: SyncMode,
    #[serde(default)]
    pub whitelist: Whitelist,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    #[default]
    Whitelist,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Whitelist {
    #[serde(default)]
    pub paths: Vec<String>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}
