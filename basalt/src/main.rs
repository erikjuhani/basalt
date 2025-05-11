use app::App;
use basalt_core::obsidian::ObsidianConfig;
use std::io;

pub mod actions;
pub mod app;
pub mod events;
pub mod main_view;
pub mod mode;
pub mod screen;
pub mod start_view;
pub mod text_counts;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let obsidian_config = ObsidianConfig::load().unwrap();
    let vaults = obsidian_config.vaults();

    App::start(terminal, vaults)?;
    ratatui::restore();

    Ok(())
}
