use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub version: u32,
    pub tabs: Vec<SavedTab>,
    pub active_tab: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTab {
    pub title: String,
    pub lines: Vec<String>,
}

impl SavedSession {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("windowed-claude")
            .join("session.json")
    }

    /// Atomic write: serialize → write to .tmp → rename to final path.
    /// Sets 0600 permissions on Unix before rename.
    pub fn save(&self) {
        let path = Self::path();
        let tmp_path = path.with_extension("json.tmp");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = match serde_json::to_string_pretty(self) {
            Ok(j) => j,
            Err(e) => {
                log::error!("Failed to serialize session: {}", e);
                return;
            }
        };
        if let Err(e) = std::fs::write(&tmp_path, &json) {
            log::error!("Failed to write session tmp file: {}", e);
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600));
        }
        if let Err(e) = std::fs::rename(&tmp_path, &path) {
            log::error!("Failed to rename session file: {}", e);
        }
    }

    /// Load session from disk. Deletes the file after reading regardless of success.
    /// Returns None on any error or if the session has no tabs.
    pub fn load() -> Option<Self> {
        let path = Self::path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok();
        // Delete file after read (consume-once)
        Self::delete();
        let session: Self = serde_json::from_str(&content?).ok()?;
        if session.tabs.is_empty() {
            return None;
        }
        Some(session)
    }

    pub fn delete() {
        let _ = std::fs::remove_file(Self::path());
    }
}
