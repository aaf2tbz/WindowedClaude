mod git;
mod claude;
pub mod shortcuts;

use anyhow::Result;
use log::info;
use std::path::PathBuf;

/// Where WindowedClaude stores its data (Git portable, config, etc.)
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("windowed-claude")
}

/// Path to the bundled/downloaded Git Bash executable (Windows only)
pub fn git_bash_path() -> PathBuf {
    if cfg!(windows) {
        data_dir().join("git").join("bin").join("bash.exe")
    } else {
        PathBuf::from("/bin/bash")
    }
}

/// Path to the Claude CLI executable
pub fn claude_cli_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    if cfg!(windows) {
        home.join(".local").join("bin").join("claude.exe")
    } else {
        home.join(".local").join("bin").join("claude")
    }
}

/// Marker file that indicates setup has completed
fn installed_marker() -> PathBuf {
    data_dir().join(".installed")
}

/// Whether the shortcut prompt has been shown (so we only ask once)
fn shortcut_prompted_marker() -> PathBuf {
    data_dir().join(".shortcut_prompted")
}

/// Check if first-time setup has already been completed
pub fn is_installed() -> bool {
    if !cfg!(windows) {
        return installed_marker().exists() || claude_cli_path().exists();
    }
    installed_marker().exists() && git_bash_path().exists() && claude_cli_path().exists()
}

/// Check if we still need to show the shortcut prompt
pub fn needs_shortcut_prompt() -> bool {
    if !cfg!(windows) {
        return false;
    }
    !shortcut_prompted_marker().exists()
}

/// Mark that the shortcut prompt has been shown
pub fn mark_shortcut_prompted() {
    let marker = shortcut_prompted_marker();
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(marker, "ok");
}

/// Run the full first-time setup (does NOT create shortcuts — that's handled by the welcome screen)
pub fn run_first_time_setup() -> Result<()> {
    let data = data_dir();
    std::fs::create_dir_all(&data)?;

    if cfg!(windows) {
        info!("Step 1/2: Installing Git for Windows...");
        git::install_git_portable(&data)?;

        info!("Step 2/2: Installing Claude Code CLI...");
        claude::install_claude_cli(&git_bash_path())?;
    } else {
        info!("Step 1/1: Installing Claude Code CLI...");
        claude::install_claude_cli(&git_bash_path())?;
    }

    // Always create Start Menu shortcut (standard Windows behavior)
    if cfg!(windows) {
        if let Err(e) = shortcuts::create_start_menu_shortcut() {
            log::warn!("Start Menu shortcut failed (non-fatal): {}", e);
        }
        // Register right-click "Run with Auto-Accept" context menu on the exe
        if let Err(e) = shortcuts::register_context_menu() {
            log::warn!("Context menu registration failed (non-fatal): {}", e);
        }
    }

    // Mark as installed
    std::fs::write(installed_marker(), "ok")?;
    info!("Setup complete!");
    Ok(())
}
