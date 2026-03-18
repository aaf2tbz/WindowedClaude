mod config;
mod installer;
mod terminal;
mod ui;

use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    env_logger::init();
    info!("WindowedClaude starting up");

    // Load persisted config (theme, opacity, font, etc.)
    let config = config::Config::load();
    info!("Config loaded: theme={}, transparent={}, opacity={}", config.theme_id, config.transparent, config.opacity);

    // Phase 1: Check if first run — install Git + Claude CLI if needed
    if !installer::is_installed() {
        info!("First run detected — starting installer");
        installer::run_first_time_setup()?;
    }

    // Phase 2: Check if we need to show the welcome/shortcut screen
    let show_welcome = installer::needs_shortcut_prompt();

    // Phase 3: Launch the themed terminal (with welcome screen if first run)
    info!("Launching terminal (welcome={})", show_welcome);
    ui::run(config, show_welcome)
}
