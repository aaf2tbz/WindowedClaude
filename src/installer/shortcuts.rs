use anyhow::Result;
use log::info;
use std::path::PathBuf;

/// Create Start Menu shortcuts (Windows only)
pub fn create_start_menu_shortcut() -> Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }
    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    if let Some(dir) = start_menu_path() {
        let lnk = dir.join("WindowedClaude.lnk");
        create_lnk(&lnk, &exe_str, "", "WindowedClaude — Claude Code Terminal")?;
        info!("Created Start Menu shortcut: {}", lnk.display());

        let auto_lnk = dir.join("WindowedClaude (Auto-Accept).lnk");
        create_lnk(
            &auto_lnk,
            &exe_str,
            "--auto-accept",
            "WindowedClaude — Auto-Accept Mode (skip permission prompts)",
        )?;
        info!("Created Start Menu auto-accept shortcut: {}", auto_lnk.display());
    }
    Ok(())
}

/// Create Desktop shortcuts inside ~/Desktop/WindowedClaude/.
/// Windows: .lnk files. macOS: .app bundles (no Terminal.app). Linux: .desktop files.
pub fn create_desktop_shortcut() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    if let Some(desktop) = dirs::desktop_dir() {
        // All platforms: put shortcuts in a WindowedClaude folder
        let dir = desktop.join("WindowedClaude");
        std::fs::create_dir_all(&dir)?;

        if cfg!(windows) {
            // Windows: .lnk files
            let lnk = dir.join("WindowedClaude.lnk");
            create_lnk(&lnk, &exe_str, "", "WindowedClaude — Claude Code Terminal")?;
            info!("Created Desktop shortcut: {}", lnk.display());

            let auto_lnk = dir.join("WindowedClaude (Auto-Accept).lnk");
            create_lnk(
                &auto_lnk,
                &exe_str,
                "--auto-accept",
                "WindowedClaude — Auto-Accept Mode (skip permission prompts)",
            )?;
            info!("Created Desktop auto-accept shortcut: {}", auto_lnk.display());
        } else if cfg!(target_os = "macos") {
            // macOS: .app bundles — launches without Terminal.app
            create_macos_app_bundle(&dir, "WindowedClaude", &exe_str, "")?;
            create_macos_app_bundle(&dir, "WindowedClaude (Auto-Accept)", &exe_str, "--auto-accept")?;
        } else {
            // Linux: .desktop files with Terminal=false
            create_linux_desktop_entry(&dir, "WindowedClaude", &exe_str, "")?;
            create_linux_desktop_entry(&dir, "WindowedClaude (Auto-Accept)", &exe_str, "--auto-accept")?;
        }
    }
    Ok(())
}

/// Create a macOS .app bundle that launches the binary without Terminal.app.
/// Structure: Name.app/Contents/MacOS/launcher (bash script that execs the real binary)
fn create_macos_app_bundle(
    parent_dir: &std::path::Path,
    name: &str,
    exe_path: &str,
    extra_args: &str,
) -> Result<()> {
    let app_dir = parent_dir.join(format!("{}.app", name));
    let contents_dir = app_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    std::fs::create_dir_all(&macos_dir)?;

    // Info.plist — minimal, tells macOS this is a GUI app (LSUIElement hides dock icon)
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>launcher</string>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>com.windowedclaude.launcher</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>LSUIElement</key>
    <false/>
</dict>
</plist>"#,
        name = name,
    );
    std::fs::write(contents_dir.join("Info.plist"), plist)?;

    // Launcher script — execs the real binary so no extra process lingers
    let args_part = if extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", extra_args)
    };
    let launcher = format!(
        "#!/bin/bash\nexec \"{}\"{}",
        exe_path, args_part
    );
    let launcher_path = macos_dir.join("launcher");
    std::fs::write(&launcher_path, launcher)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launcher_path, std::fs::Permissions::from_mode(0o755))?;
    }

    info!("Created macOS app bundle: {}", app_dir.display());
    Ok(())
}

/// Create a Linux .desktop file that launches without a terminal.
fn create_linux_desktop_entry(
    parent_dir: &std::path::Path,
    name: &str,
    exe_path: &str,
    extra_args: &str,
) -> Result<()> {
    let args_part = if extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", extra_args)
    };
    let desktop_entry = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{exe}\"{args}\nTerminal=false\nCategories=Development;\n",
        name = name,
        exe = exe_path,
        args = args_part,
    );
    let desktop_file = parent_dir.join(format!("{}.desktop", name));
    std::fs::write(&desktop_file, &desktop_entry)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&desktop_file, std::fs::Permissions::from_mode(0o755))?;
    }

    info!("Created Linux desktop entry: {}", desktop_file.display());
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

/// Check if the Desktop shortcut folder exists
pub fn has_desktop_shortcut() -> bool {
    dirs::desktop_dir()
        .map(|d| d.join("WindowedClaude").exists())
        .unwrap_or(false)
}

/// Remove the Desktop shortcut folder and all contents
pub fn remove_desktop_shortcut() -> Result<()> {
    if let Some(dir) = dirs::desktop_dir() {
        let folder = dir.join("WindowedClaude");
        if folder.exists() {
            std::fs::remove_dir_all(&folder)?;
            info!("Removed Desktop shortcut folder");
        }
        // Also clean up legacy shortcuts from older versions
        for legacy in &[
            "WindowedClaude.lnk",
            "WindowedClaude.command",
            "WindowedClaude (Auto-Accept).command",
        ] {
            let path = dir.join(legacy);
            if path.exists() {
                let _ = std::fs::remove_file(&path);
                info!("Removed legacy shortcut: {}", path.display());
            }
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

    // Set IconLocation to the exe itself so the shortcut inherits the app icon
    let ps_script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{lnk}'); $s.TargetPath = '{target}'; $s.Arguments = '{args}'; $s.Description = '{desc}'; $s.IconLocation = '{target},0'; $s.Save()"#,
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
