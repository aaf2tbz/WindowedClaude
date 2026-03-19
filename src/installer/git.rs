use anyhow::{Context, Result};
use log::info;
use std::path::Path;

/// Git for Windows full installer (includes Git Bash)
const GIT_INSTALLER_URL: &str =
    "https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.2/Git-2.47.1.2-64-bit.exe";

/// Expected size of the Git installer in bytes (integrity check).
/// Update this when bumping GIT_INSTALLER_URL to a new version.
const GIT_INSTALLER_EXPECTED_SIZE: u64 = 69_096_664;

/// Standard Git Bash install locations to check
const GIT_BASH_PATHS: &[&str] = &[
    r"C:\Program Files\Git\bin\bash.exe",
    r"C:\Program Files (x86)\Git\bin\bash.exe",
];

/// Check if Git for Windows is already installed on the system
pub fn find_system_git_bash() -> Option<std::path::PathBuf> {
    // Check standard locations
    for path in GIT_BASH_PATHS {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    // Check PATH
    if let Ok(output) = std::process::Command::new("where")
        .arg("bash.exe")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let p = std::path::PathBuf::from(line.trim());
                if p.exists() && line.contains("Git") {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Install Git for Windows using the official installer in silent mode.
/// If Git is already installed, this is a no-op.
pub fn install_git(data_dir: &Path) -> Result<()> {
    // Check if Git is already installed
    if find_system_git_bash().is_some() {
        info!("Git for Windows already installed");
        return Ok(());
    }

    info!("Downloading Git for Windows installer...");
    let response = reqwest::blocking::get(GIT_INSTALLER_URL)
        .context("Failed to download Git for Windows installer")?;

    let bytes = response.bytes().context("Failed to read download")?;
    info!("Downloaded {} bytes", bytes.len());

    // Verify download size to catch truncated or tampered downloads
    if bytes.len() as u64 != GIT_INSTALLER_EXPECTED_SIZE {
        anyhow::bail!(
            "Git installer size mismatch: expected {} bytes, got {} — possible corrupted or tampered download",
            GIT_INSTALLER_EXPECTED_SIZE,
            bytes.len()
        );
    }

    // Save installer to temp file
    let installer_path = data_dir.join("git-installer.exe");
    std::fs::write(&installer_path, &bytes)
        .context("Failed to save Git installer")?;

    info!("Running Git installer (silent)...");

    // Run the installer silently
    // /VERYSILENT = no UI at all
    // /NORESTART = don't restart
    // /NOCANCEL = can't cancel
    // /SP- = don't show "This will install..." prompt
    let output = std::process::Command::new(&installer_path)
        .args(["/VERYSILENT", "/NORESTART", "/NOCANCEL", "/SP-"])
        .output()
        .context("Failed to run Git installer")?;

    // Clean up installer
    let _ = std::fs::remove_file(&installer_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git installer failed (exit {}): {}", output.status, stderr);
    }

    // Verify installation
    if find_system_git_bash().is_none() {
        anyhow::bail!("Git installer ran but bash.exe not found in standard locations");
    }

    info!("Git for Windows installed successfully");
    Ok(())
}
