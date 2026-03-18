use anyhow::{Context, Result};
use log::info;
use std::path::Path;
use std::process::Command;

/// Install Claude Code CLI using the official install script via Git Bash
pub fn install_claude_cli(git_bash: &Path) -> Result<()> {
    // Check if Claude is already installed
    let home = dirs::home_dir().unwrap_or_default();
    let claude_exe = home.join(".local").join("bin").join("claude.exe");

    if claude_exe.exists() {
        info!("Claude CLI already installed at {}", claude_exe.display());
        return Ok(());
    }

    info!("Running Claude Code installer via Git Bash...");

    // Use Git Bash to run the official install script
    let output = Command::new(git_bash)
        .args(["-c", "curl -fsSL https://claude.ai/install.sh | bash"])
        .output()
        .context("Failed to run Claude installer via Git Bash")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude installer failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("Installer output: {}", stdout);

    // Verify installation
    if !claude_exe.exists() {
        anyhow::bail!(
            "Installer ran but claude.exe not found at {}",
            claude_exe.display()
        );
    }

    info!("Claude Code CLI installed successfully");
    Ok(())
}
