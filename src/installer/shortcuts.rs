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
            create_lnk(&lnk, &exe_str, "", "WindowedClaude — Claude Code Terminal")?;
            info!("Created Start Menu shortcut: {}", lnk.display());
        }
    }
    Ok(())
}

/// Create the Desktop shortcut (only when the user opts in).
/// Also creates a second shortcut for auto-accept mode.
pub fn create_desktop_shortcut() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    if let Some(dir) = dirs::desktop_dir() {
        // Main shortcut
        let lnk = dir.join("WindowedClaude.lnk");
        if !lnk.exists() {
            create_lnk(&lnk, &exe_str, "", "WindowedClaude — Claude Code Terminal")?;
            info!("Created Desktop shortcut: {}", lnk.display());
        }
    }
    Ok(())
}

/// Register a Windows shell context menu entry on the exe.
/// Adds "Run with Auto-Accept" to the right-click menu.
pub fn register_context_menu() -> Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    // Use PowerShell to write registry entries for the context menu.
    // This registers under HKCU so no admin rights needed.
    //
    // Registry path:
    //   HKCU\Software\Classes\Applications\windowed-claude.exe\shell\autoaccept
    //     (Default) = "Run with Auto-Accept"
    //     Icon = "<exe_path>"
    //   HKCU\...\autoaccept\command
    //     (Default) = "<exe_path>" --auto-accept
    let exe_name = exe_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "windowed-claude.exe".to_string());

    let reg_base = format!(
        r"HKCU:\Software\Classes\Applications\{}\shell\autoaccept",
        exe_name
    );

    let ps_script = format!(
        r#"
        New-Item -Path '{reg_base}' -Force | Out-Null
        Set-ItemProperty -Path '{reg_base}' -Name '(Default)' -Value 'Run with Auto-Accept'
        Set-ItemProperty -Path '{reg_base}' -Name 'Icon' -Value '"{exe}"'
        New-Item -Path '{reg_base}\command' -Force | Out-Null
        Set-ItemProperty -Path '{reg_base}\command' -Name '(Default)' -Value '"{exe}" --auto-accept'
        "#,
        reg_base = reg_base,
        exe = exe_str,
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()?;

    if output.status.success() {
        info!("Registered context menu: Run with Auto-Accept");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("Context menu registration failed: {}", stderr);
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
fn create_lnk(
    lnk_path: &std::path::Path,
    target: &str,
    arguments: &str,
    description: &str,
) -> Result<()> {
    let lnk_str = lnk_path.to_string_lossy();

    let ps_script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{lnk}'); $s.TargetPath = '{target}'; $s.Arguments = '{args}'; $s.Description = '{desc}'; $s.Save()"#,
        lnk = lnk_str,
        target = target,
        args = arguments,
        desc = description,
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
