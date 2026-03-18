use anyhow::Result;
use log::info;
use std::path::PathBuf;

/// Create only the Start Menu shortcut (always created on install)
pub fn create_start_menu_shortcut() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    if let Some(dir) = start_menu_path() {
        let lnk = dir.join("WindowedClaude.lnk");
        if !lnk.exists() {
            create_lnk(&lnk, &exe_str, "WindowedClaude — Claude Code Terminal")?;
            info!("Created Start Menu shortcut: {}", lnk.display());
        }
    }
    Ok(())
}

/// Create the Desktop shortcut (only when the user opts in)
pub fn create_desktop_shortcut() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    if let Some(dir) = dirs::desktop_dir() {
        let lnk = dir.join("WindowedClaude.lnk");
        if !lnk.exists() {
            create_lnk(&lnk, &exe_str, "WindowedClaude — Claude Code Terminal")?;
            info!("Created Desktop shortcut: {}", lnk.display());
        } else {
            info!("Desktop shortcut already exists");
        }
    }
    Ok(())
}

/// Check if the Desktop shortcut exists
pub fn has_desktop_shortcut() -> bool {
    dirs::desktop_dir()
        .map(|d| d.join("WindowedClaude.lnk").exists())
        .unwrap_or(false)
}

/// Remove the Desktop shortcut
pub fn remove_desktop_shortcut() -> Result<()> {
    if let Some(dir) = dirs::desktop_dir() {
        let lnk = dir.join("WindowedClaude.lnk");
        if lnk.exists() {
            std::fs::remove_file(&lnk)?;
            info!("Removed Desktop shortcut");
        }
    }
    Ok(())
}

/// Get the Start Menu Programs directory
fn start_menu_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| {
        d.parent()
            .unwrap_or(&d)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
    })
}

/// Create a .lnk shortcut file using PowerShell COM
fn create_lnk(lnk_path: &std::path::Path, target: &str, description: &str) -> Result<()> {
    let lnk_str = lnk_path.to_string_lossy();

    let ps_script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath = '{}'; $s.Description = '{}'; $s.Save()"#,
        lnk_str, target, description
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("Shortcut creation failed: {}", stderr);
    }

    Ok(())
}
