use basalt_tui as basalt;

use std::io;

use basalt::app::App;
use basalt_core::obsidian::ObsidianConfig;

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let obsidian_config = ObsidianConfig::load().unwrap();
    let vaults = obsidian_config.vaults();

    App::start(terminal, vaults)?;
    ratatui::restore();

    Ok(())
}
