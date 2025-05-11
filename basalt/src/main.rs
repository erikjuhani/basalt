use std::io;

use app::App;
use basalt_core::obsidian::ObsidianConfig;

pub mod app;
pub mod help_modal;
pub mod markdown;
pub mod sidepanel;
pub mod start;
pub mod statusbar;
pub mod text_counts;
pub mod vault_selector;
pub mod vault_selector_modal;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let obsidian_config = ObsidianConfig::load().unwrap();
    let vaults = obsidian_config.vaults();

    App::start(terminal, vaults)?;
    ratatui::restore();

    Ok(())
}
