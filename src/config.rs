use crate::ui::theme;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// User-configurable settings, persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Font size in points
    pub font_size: f32,
    /// Font family name
    pub font_family: String,
    /// Theme ID (matches theme::THEME_IDS)
    pub theme_id: String,
    /// Window opacity 0.0 (fully transparent) to 1.0 (fully opaque)
    pub opacity: f32,
    /// Whether transparency mode is enabled
    pub transparent: bool,
    /// Custom Git Bash path override
    pub git_bash_path: Option<PathBuf>,
    /// Runtime flag: launch Claude with --dangerously-skip-permissions
    #[serde(skip)]
    pub auto_accept: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_family: "JetBrains Mono".to_string(),
            theme_id: "claude-dark".to_string(),
            opacity: 1.0,
            transparent: false,
            git_bash_path: None,
            auto_accept: false,
        }
    }
}

impl Config {
    /// Load config from disk, or create default
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let mut config: Config = serde_json::from_str(&content).unwrap_or_default();
            config.validate();
            config
        } else {
            let config = Self::default();
            config.save();
            config
        }
    }

    /// Save config to disk
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Clamp values to valid ranges, fix unknown theme IDs
    fn validate(&mut self) {
        self.opacity = self.opacity.clamp(0.05, 1.0);
        self.font_size = self.font_size.clamp(8.0, 48.0);

        // Fall back to claude-dark if theme ID is invalid
        if !theme::THEME_IDS.contains(&self.theme_id.as_str()) {
            self.theme_id = "claude-dark".to_string();
        }
    }

    /// Get the effective opacity (1.0 if transparency is disabled)
    pub fn effective_opacity(&self) -> f32 {
        if self.transparent {
            self.opacity
        } else {
            1.0
        }
    }

    /// Cycle to next theme and persist
    pub fn cycle_theme(&mut self) {
        self.theme_id = theme::next_theme_id(&self.theme_id).to_string();
        self.save();
    }

    /// Toggle transparency on/off and persist
    pub fn toggle_transparency(&mut self) {
        self.transparent = !self.transparent;
        // Default to 85% if turning on for the first time at 100%
        if self.transparent && self.opacity >= 1.0 {
            self.opacity = 0.85;
        }
        self.save();
    }

    /// Adjust opacity by delta (clamped), persist
    pub fn adjust_opacity(&mut self, delta: f32) {
        self.transparent = true;
        self.opacity = (self.opacity + delta).clamp(0.05, 1.0);
        self.save();
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("windowed-claude")
            .join("config.json")
    }
}
