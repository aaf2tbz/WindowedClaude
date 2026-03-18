mod git;
mod claude;

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
        // On macOS/Linux, use system bash
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

/// Check if first-time setup has already been completed
pub fn is_installed() -> bool {
    // On non-Windows, just check for Claude CLI (no Git install needed)
    if !cfg!(windows) {
        return installed_marker().exists() || claude_cli_path().exists();
    }
    installed_marker().exists() && git_bash_path().exists() && claude_cli_path().exists()
}

/// Run the full first-time setup:
/// 1. Download and extract Git for Windows (portable) — Windows only
/// 2. Install Claude Code CLI via the official installer
/// 3. Write the installed marker
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

    // Mark as installed
    std::fs::write(installed_marker(), "ok")?;
    info!("Setup complete!");
    Ok(())
}
