mod git;
mod claude;
pub mod shortcuts;
pub mod uninstall;

use anyhow::Result;
use log::info;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Message type for install progress (must match window.rs InstallMessage)
pub enum InstallMsg {
    Progress(String),
    Done,
    Error(String),
}

/// Where WindowedClaude stores its data (Git portable, config, etc.)
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("windowed-claude")
}

/// Path to Git Bash executable
pub fn git_bash_path() -> PathBuf {
    if cfg!(windows) {
        // Check standard Git for Windows install locations
        git::find_system_git_bash()
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"))
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

/// Whether the shortcut prompt has been shown
fn shortcut_prompted_marker() -> PathBuf {
    data_dir().join(".shortcut_prompted")
}

/// Check if first-time setup has already been completed
pub fn is_installed() -> bool {
    if !cfg!(windows) {
        return installed_marker().exists() || claude_cli_path().exists();
    }
    // On Windows: need both Git Bash and Claude CLI
    installed_marker().exists()
        && git::find_system_git_bash().is_some()
        && claude_cli_path().exists()
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

/// Helper to send progress (ignores send errors if receiver is gone)
fn progress<T: std::fmt::Display>(tx: &Sender<InstallMsg>, msg: T) {
    let _ = tx.send(InstallMsg::Progress(msg.to_string()));
}

/// Run first-time setup with progress reporting to the UI thread.
/// Called from a background thread — sends InstallMsg via the channel.
pub fn run_first_time_setup_with_progress(tx: &Sender<InstallMsg>) -> Result<()> {
    let data = data_dir();
    std::fs::create_dir_all(&data)?;

    if cfg!(windows) {
        // Check if Git needs to be installed (track whether we did it)
        let had_git = git::find_system_git_bash().is_some();
        progress(tx, "Installing Git for Windows...");
        git::install_git(&data)?;
        if !had_git && git::find_system_git_bash().is_some() {
            uninstall::mark_git_installed_by_us();
        }

        // Check if Claude CLI needs to be installed (track whether we did it)
        let had_claude = claude_cli_path().exists();
        progress(tx, "Installing Claude Code CLI...");
        claude::install_claude_cli(&git_bash_path())?;
        if !had_claude && claude_cli_path().exists() {
            uninstall::mark_claude_installed_by_us();
        }
    } else {
        progress(tx, "Installing Claude Code CLI...");
        claude::install_claude_cli(&git_bash_path())?;
    }

    // Start Menu shortcut + context menu + ARP registration (Windows)
    if cfg!(windows) {
        progress(tx, "Creating shortcuts...");
        if let Err(e) = shortcuts::create_start_menu_shortcut() {
            log::warn!("Start Menu shortcut failed (non-fatal): {}", e);
        }
        if let Err(e) = shortcuts::register_context_menu() {
            log::warn!("Context menu registration failed (non-fatal): {}", e);
        }
        uninstall::register_arp();
    }

    // Mark as installed
    progress(tx, "Finishing up...");
    std::fs::write(installed_marker(), "ok")?;
    info!("Setup complete!");
    Ok(())
}
