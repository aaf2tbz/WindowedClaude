// Suppress the console window on Windows when double-clicking the exe
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod config;
mod installer;
mod session;
mod terminal;
mod ui;

use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    env_logger::init();
    info!("WindowedClaude starting up");

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();

    // Handle --uninstall
    if args.iter().any(|a| a == "--uninstall") {
        info!("Uninstall mode requested");
        return installer::uninstall::run_uninstall();
    }

    let auto_accept = args.iter().any(|a| a == "--auto-accept");

    if auto_accept {
        info!("Auto-accept mode: Claude will run with --dangerously-skip-permissions");
    }

    // Load persisted config
    let mut config = config::Config::load();
    config.auto_accept = auto_accept;
    info!("Config loaded: theme={}, transparent={}, opacity={}", config.theme_id, config.transparent, config.opacity);

    // Determine what phase to start in
    let needs_install = !installer::is_installed();
    let needs_welcome = installer::needs_shortcut_prompt();

    info!(
        "Launching (needs_install={}, needs_welcome={}, auto_accept={})",
        needs_install, needs_welcome, auto_accept
    );

    // Window opens FIRST — installer runs inside the window with visual progress
    ui::run(config, needs_install, needs_welcome)
}
