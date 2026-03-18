mod config;
mod installer;
mod terminal;
mod ui;

use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    env_logger::init();
    info!("WindowedClaude starting up");

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let auto_accept = args.iter().any(|a| a == "--auto-accept");

    if auto_accept {
        info!("Auto-accept mode: Claude will run with --dangerously-skip-permissions");
    }

    // Load persisted config
    let mut config = config::Config::load();
    config.auto_accept = auto_accept;
    info!("Config loaded: theme={}, transparent={}, opacity={}", config.theme_id, config.transparent, config.opacity);

    // Phase 1: Check if first run — install Git + Claude CLI if needed
    if !installer::is_installed() {
        info!("First run detected — starting installer");
        installer::run_first_time_setup()?;
    }

    // Phase 2: Check if we need to show the welcome/shortcut screen
    let show_welcome = installer::needs_shortcut_prompt();

    // Phase 3: Launch the themed terminal (with welcome screen if first run)
    info!("Launching terminal (welcome={}, auto_accept={})", show_welcome, auto_accept);
    ui::run(config, show_welcome)
}
