use crate::ui::theme;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// All configurable keybind action IDs
pub const KEYBIND_ACTIONS: &[(&str, &str, &str)] = &[
    // (id, display_name, default_combo)
    ("new_tab",         "New Tab",          "Ctrl+N"),
    ("close_tab",       "Close Tab",        "Ctrl+W"),
    ("next_tab",        "Next Tab",         "Ctrl+Tab"),
    ("prev_tab",        "Prev Tab",         "Ctrl+Shift+Tab"),
    ("toggle_transparency", "Transparency", "Ctrl+Shift+O"),
    ("copy",            "Copy",             "Ctrl+Shift+C"),
    ("paste",           "Paste",            "Ctrl+Shift+V"),
    ("increase_opacity","Opacity +",        "Ctrl+Shift+="),
    ("decrease_opacity","Opacity -",        "Ctrl+Shift+-"),
    ("increase_font",   "Font +",           "Ctrl+="),
    ("decrease_font",   "Font -",           "Ctrl+-"),
    ("reset_font",      "Font Reset",       "Ctrl+0"),
    ("force_kill",      "Force Kill",       "Ctrl+Shift+K"),
];

/// Keybind configuration — maps action IDs to key combo strings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinds {
    #[serde(flatten)]
    pub bindings: HashMap<String, String>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        for (id, _, default_combo) in KEYBIND_ACTIONS {
            bindings.insert(id.to_string(), default_combo.to_string());
        }
        Self { bindings }
    }
}

impl KeyBinds {
    /// Get the combo string for an action
    pub fn get(&self, action_id: &str) -> &str {
        self.bindings.get(action_id)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Get the default combo for an action
    pub fn default_for(action_id: &str) -> &'static str {
        KEYBIND_ACTIONS.iter()
            .find(|(id, _, _)| *id == action_id)
            .map(|(_, _, combo)| *combo)
            .unwrap_or("")
    }

    /// Set a keybind
    pub fn set(&mut self, action_id: &str, combo: &str) {
        self.bindings.insert(action_id.to_string(), combo.to_string());
    }

    /// Reset all to defaults
    pub fn reset_all(&mut self) {
        *self = Self::default();
    }

    /// Check if a specific combo string matches ctrl+key press
    pub fn combo_matches(combo: &str, ctrl: bool, shift: bool, key: &str) -> bool {
        let parts: Vec<&str> = combo.split('+').collect();
        let needs_ctrl = parts.iter().any(|p| p.eq_ignore_ascii_case("ctrl") || p.eq_ignore_ascii_case("cmd"));
        let needs_shift = parts.iter().any(|p| p.eq_ignore_ascii_case("shift"));
        let key_part = parts.last().unwrap_or(&"");

        needs_ctrl == ctrl && needs_shift == shift && key_part.eq_ignore_ascii_case(key)
    }
}

fn default_padding() -> usize { 12 }

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
    /// Padding around the terminal grid in pixels (0-48)
    #[serde(default = "default_padding")]
    pub padding: usize,
    /// Custom Git Bash path override
    pub git_bash_path: Option<PathBuf>,
    /// Custom keybindings
    #[serde(default)]
    pub keybinds: KeyBinds,
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
            padding: 12,
            git_bash_path: None,
            keybinds: KeyBinds::default(),
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

    /// Save config to disk atomically (write to temp file, then rename).
    /// Prevents config corruption from rapid saves or crashes mid-write.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let tmp = parent.join(".config.json.tmp");
                if std::fs::write(&tmp, &json).is_ok() {
                    let _ = std::fs::rename(&tmp, &path);
                }
            }
        }
    }

    /// Clamp values to valid ranges, fix unknown theme IDs
    fn validate(&mut self) {
        self.opacity = self.opacity.clamp(0.05, 1.0);
        self.font_size = self.font_size.clamp(8.0, 48.0);
        self.padding = self.padding.min(48);

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

    /// Cycle to next theme (caller handles persistence)
    pub fn cycle_theme(&mut self) {
        self.theme_id = theme::next_theme_id(&self.theme_id).to_string();
    }

    /// Toggle transparency on/off (caller handles persistence)
    pub fn toggle_transparency(&mut self) {
        self.transparent = !self.transparent;
        // Default to 85% if turning on for the first time at 100%
        if self.transparent && self.opacity >= 1.0 {
            self.opacity = 0.85;
        }
    }

    /// Adjust opacity by delta (clamped) (caller handles persistence)
    pub fn adjust_opacity(&mut self, delta: f32) {
        self.transparent = true;
        self.opacity = (self.opacity + delta).clamp(0.05, 1.0);
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("windowed-claude")
            .join("config.json")
    }
}
