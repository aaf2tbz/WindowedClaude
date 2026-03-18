use anyhow::Result;
use log::info;
use std::path::PathBuf;

/// Create Windows Start Menu and Desktop shortcuts for WindowedClaude.
/// Uses PowerShell COM automation to create .lnk files.
///
/// This is a no-op on non-Windows platforms.
pub fn create_shortcuts() -> Result<()> {
    if !cfg!(windows) {
        info!("Shortcuts: skipped (not Windows)");
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    // Start Menu shortcut
    let start_menu = start_menu_path();
    if let Some(dir) = &start_menu {
        let lnk = dir.join("WindowedClaude.lnk");
        if !lnk.exists() {
            create_lnk(&lnk, &exe_str, "WindowedClaude — Claude Code Terminal")?;
            info!("Created Start Menu shortcut: {}", lnk.display());
        }
    }

    // Desktop shortcut
    let desktop = dirs::desktop_dir();
    if let Some(dir) = &desktop {
        let lnk = dir.join("WindowedClaude.lnk");
        if !lnk.exists() {
            create_lnk(&lnk, &exe_str, "WindowedClaude — Claude Code Terminal")?;
            info!("Created Desktop shortcut: {}", lnk.display());
        }
    }

    Ok(())
}

/// Get the Start Menu Programs directory
fn start_menu_path() -> Option<PathBuf> {
    // %APPDATA%\Microsoft\Windows\Start Menu\Programs
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

    // PowerShell script to create shortcut via WScript.Shell COM
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
