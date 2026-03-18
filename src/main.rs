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

    // Phase 2: Launch the themed terminal with Claude
    info!("Launching terminal");
    ui::run(config)
}
