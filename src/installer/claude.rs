use anyhow::{Context, Result};
use log::info;
use std::path::Path;
use std::process::Command;

/// Install Claude Code CLI using the official method for each platform
pub fn install_claude_cli(git_bash: &Path) -> Result<()> {
    let home = dirs::home_dir().unwrap_or_default();

    // Check platform-specific exe location
    let claude_exe = if cfg!(windows) {
        home.join(".local").join("bin").join("claude.exe")
    } else {
        home.join(".local").join("bin").join("claude")
    };

    if claude_exe.exists() {
        info!("Claude CLI already installed at {}", claude_exe.display());
        return Ok(());
    }

    if cfg!(windows) {
        install_claude_windows()?;
    } else {
        install_claude_unix(git_bash)?;
    }

    // Verify installation
    if !claude_exe.exists() {
        anyhow::bail!(
            "Installer ran but claude not found at {}",
            claude_exe.display()
        );
    }

    info!("Claude Code CLI installed successfully");
    Ok(())
}

/// Install Claude on Windows using PowerShell (official method)
fn install_claude_windows() -> Result<()> {
    info!("Installing Claude Code via PowerShell...");

    // Official Windows install command: irm https://claude.ai/install.ps1 | iex
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "irm https://claude.ai/install.ps1 | iex",
        ])
        .output()
        .context("Failed to run Claude installer via PowerShell")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    info!("Installer stdout: {}", stdout);

    if !output.status.success() {
        anyhow::bail!("Claude installer failed: {}", stderr);
    }

    Ok(())
}

/// Install Claude on macOS/Linux using bash (official method)
fn install_claude_unix(bash: &Path) -> Result<()> {
    info!("Installing Claude Code via bash...");

    let output = Command::new(bash)
        .args(["-c", "curl -fsSL https://claude.ai/install.sh | bash"])
        .output()
        .context("Failed to run Claude installer via bash")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude installer failed: {}", stderr);
    }

    Ok(())
}
