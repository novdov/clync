use std::env;
use std::fs;
use std::path::PathBuf;

use chrono::Local;

use crate::config::loader::claude_dir;
use crate::Result;

const MAX_BACKUPS: usize = 10;

pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new() -> Self {
        let backup_dir = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".clync")
            .join("backup");

        Self { backup_dir }
    }

    pub fn create_backup(&self, files: &[String]) -> Result<PathBuf> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_path = self.backup_dir.join(&timestamp);

        fs::create_dir_all(&backup_path)?;

        let claude = claude_dir();

        for file in files {
            let source = claude.join(file);
            if source.exists() {
                let dest = backup_path.join(file);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&source, &dest)?;
            }
        }

        self.cleanup_old_backups()?;

        Ok(backup_path)
    }

    pub fn cleanup_old_backups(&self) -> Result<()> {
        if !self.backup_dir.exists() {
            return Ok(());
        }

        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        backups.sort_by_key(|e| e.path());

        if backups.len() > MAX_BACKUPS {
            let to_remove = backups.len() - MAX_BACKUPS;
            for entry in backups.into_iter().take(to_remove) {
                fs::remove_dir_all(entry.path())?;
            }
        }

        Ok(())
    }

    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        if !self.backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();

        backups.sort();
        backups.reverse();

        Ok(backups)
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new()
    }
}
