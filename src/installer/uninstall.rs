use anyhow::Result;
use log::info;
use std::path::PathBuf;

/// Run the full uninstall process.
/// Removes shortcuts, registry entries, data, and optionally Git + Claude CLI.
pub fn run_uninstall() -> Result<()> {
    info!("Starting uninstall...");

    // 1. Remove Desktop + Start Menu shortcuts
    remove_shortcuts();

    // 2. Remove context menu registry entries
    remove_context_menu();

    // 3. Remove Add/Remove Programs registry entry
    remove_arp_entry();

    // 4. Remove windowed-claude data dir + config dir
    remove_data_dirs();

    // 5. Remove Claude CLI if we installed it (marker check)
    remove_claude_cli_if_ours();

    // 6. Uninstall Git for Windows if we installed it (marker check)
    uninstall_git_if_ours();

    // 7. Self-delete the exe via delayed cmd trick
    self_delete();

    info!("Uninstall complete.");
    Ok(())
}

fn remove_shortcuts() {
    // Desktop shortcut
    if let Some(desktop) = dirs::desktop_dir() {
        let lnk = desktop.join("WindowedClaude.lnk");
        if lnk.exists() {
            let _ = std::fs::remove_file(&lnk);
            info!("Removed Desktop shortcut");
        }
    }

    // Start Menu shortcut
    if let Some(data) = dirs::data_dir() {
        let start_menu = data
            .parent()
            .unwrap_or(&data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("WindowedClaude.lnk");
        if start_menu.exists() {
            let _ = std::fs::remove_file(&start_menu);
            info!("Removed Start Menu shortcut");
        }
    }
}

fn remove_context_menu() {
    if !cfg!(windows) {
        return;
    }

    let exe_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "windowed-claude.exe".to_string());

    let reg_path = format!(
        r"HKCU:\Software\Classes\Applications\{}\shell",
        exe_name
    );

    let reg_path_escaped = reg_path.replace('\'', "''");
    let ps_script = format!(
        r#"Remove-Item -Path '{}' -Recurse -Force -ErrorAction SilentlyContinue"#,
        reg_path_escaped
    );

    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();

    info!("Removed context menu registry entries");
}

fn remove_arp_entry() {
    if !cfg!(windows) {
        return;
    }

    let ps_script = r#"Remove-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\WindowedClaude' -Recurse -Force -ErrorAction SilentlyContinue"#;

    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_script])
        .output();

    info!("Removed Add/Remove Programs entry");
}

fn remove_data_dirs() {
    // Data dir
    let data = super::data_dir();
    if data.exists() {
        let _ = std::fs::remove_dir_all(&data);
        info!("Removed data dir: {}", data.display());
    }

    // Config dir
    if let Some(config_dir) = dirs::config_dir() {
        let config = config_dir.join("windowed-claude");
        if config.exists() {
            let _ = std::fs::remove_dir_all(&config);
            info!("Removed config dir: {}", config.display());
        }
    }
}

fn remove_claude_cli_if_ours() {
    // Only remove if we have a marker indicating we installed it
    let marker = super::data_dir().join(".claude_installed_by_us");
    if !marker.exists() {
        info!("Claude CLI was not installed by us, skipping");
        return;
    }

    let home = dirs::home_dir().unwrap_or_default();
    let claude_exe = if cfg!(windows) {
        home.join(".local").join("bin").join("claude.exe")
    } else {
        home.join(".local").join("bin").join("claude")
    };

    if claude_exe.exists() {
        let _ = std::fs::remove_file(&claude_exe);
        info!("Removed Claude CLI: {}", claude_exe.display());
    }
}

fn uninstall_git_if_ours() {
    if !cfg!(windows) {
        return;
    }

    let marker = super::data_dir().join(".git_installed_by_us");
    if !marker.exists() {
        info!("Git was not installed by us, skipping");
        return;
    }

    // Try to find Git's uninstaller
    let git_uninstaller = PathBuf::from(r"C:\Program Files\Git\unins000.exe");
    if git_uninstaller.exists() {
        info!("Running Git uninstaller silently...");
        let _ = std::process::Command::new(&git_uninstaller)
            .args(["/VERYSILENT", "/NORESTART"])
            .output();
        info!("Git uninstaller finished");
    } else {
        info!("Git uninstaller not found, skipping");
    }
}

fn self_delete() {
    if !cfg!(windows) {
        return;
    }

    // Use cmd /c with ping delay trick to delete after we exit.
    // Pass exe path via environment variable to avoid cmd.exe injection.
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "ping 127.0.0.1 -n 3 > nul & del /f /q \"%WCLAUDE_EXE%\""])
            .env("WCLAUDE_EXE", &exe)
            .spawn();
        info!("Scheduled self-delete");
    }
}

/// Register with Add/Remove Programs so the app shows in Windows Settings > Apps.
/// Called during install.
pub fn register_arp() {
    if !cfg!(windows) {
        return;
    }

    let exe_path = std::env::current_exe().unwrap_or_default();
    let exe_escaped = exe_path.to_string_lossy().replace('\'', "''");

    let ps_script = format!(
        r#"
        $key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\WindowedClaude'
        New-Item -Path $key -Force | Out-Null
        Set-ItemProperty -Path $key -Name 'DisplayName' -Value 'WindowedClaude'
        Set-ItemProperty -Path $key -Name 'DisplayVersion' -Value '1.2.2'
        Set-ItemProperty -Path $key -Name 'Publisher' -Value 'WindowedClaude'
        Set-ItemProperty -Path $key -Name 'UninstallString' -Value '"{exe}" --uninstall'
        Set-ItemProperty -Path $key -Name 'DisplayIcon' -Value '"{exe}"'
        Set-ItemProperty -Path $key -Name 'NoModify' -Value 1 -Type DWord
        Set-ItemProperty -Path $key -Name 'NoRepair' -Value 1 -Type DWord
        "#,
        exe = exe_escaped,
    );

    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();

    info!("Registered with Add/Remove Programs");
}

/// Write a marker indicating we installed Claude CLI
pub fn mark_claude_installed_by_us() {
    let marker = super::data_dir().join(".claude_installed_by_us");
    let _ = std::fs::write(marker, "ok");
}

/// Write a marker indicating we installed Git for Windows
pub fn mark_git_installed_by_us() {
    let marker = super::data_dir().join(".git_installed_by_us");
    let _ = std::fs::write(marker, "ok");
}
