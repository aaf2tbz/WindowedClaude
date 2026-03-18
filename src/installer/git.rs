use anyhow::{Context, Result};
use log::info;
use std::path::Path;

/// Git for Windows portable download URL (MinGit — minimal portable distribution)
/// MinGit is ~45MB and contains just enough for Claude Code to work
const MINGIT_URL: &str =
    "https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.2/MinGit-2.47.1.2-64-bit.zip";

/// Download and extract Git for Windows portable into `data_dir/git/`
pub fn install_git_portable(data_dir: &Path) -> Result<()> {
    let git_dir = data_dir.join("git");

    if git_dir.join("bin").join("bash.exe").exists() {
        info!("Git already installed at {}", git_dir.display());
        return Ok(());
    }

    info!("Downloading MinGit from GitHub...");
    let response = reqwest::blocking::get(MINGIT_URL)
        .context("Failed to download Git for Windows")?;

    let bytes = response.bytes().context("Failed to read download")?;
    info!("Downloaded {} bytes", bytes.len());

    // Extract zip to git directory
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .context("Failed to open Git zip archive")?;

    std::fs::create_dir_all(&git_dir)?;

    info!("Extracting Git to {}...", git_dir.display());
    archive.extract(&git_dir)
        .context("Failed to extract Git archive")?;

    // Verify bash.exe exists
    let bash = git_dir.join("bin").join("bash.exe");
    if !bash.exists() {
        anyhow::bail!(
            "Git extraction succeeded but bash.exe not found at {}",
            bash.display()
        );
    }

    info!("Git for Windows installed successfully");
    Ok(())
}
