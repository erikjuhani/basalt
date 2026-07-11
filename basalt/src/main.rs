use clap::Parser;
use std::path::PathBuf;

use basalt_core::obsidian::{self, Error, Vault};
use basalt_tui::{app::App, cli::Cli, debug_log};
use ratatui_image::picker::Picker;

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    debug_log::init();

    let obsidian_config = obsidian::config::load().unwrap();
    let vaults = obsidian_config.vaults();

    let initial_vault = match std::env::var("BASALT_EXP_VAULT_PATH") {
        Ok(path) => {
            let path = PathBuf::from(&path).canonicalize()?;
            let name = path
                .file_name()
                .and_then(|os_str| os_str.to_str().map(|str| str.to_string()))
                .ok_or_else(|| Error::InvalidPathName(path.to_path_buf()))?;

            Some(Vault {
                name,
                path,
                open: false,
                ts: 0,
            })
        }
        Err(_) => None,
    };

    // Query graphics support before the alternate screen, falling back to
    // half-blocks. `BASALT_EXP_IMAGE_HALFBLOCKS` forces them (e.g. for VHS).
    let picker = Some(match std::env::var_os("BASALT_EXP_IMAGE_HALFBLOCKS") {
        Some(_) => Picker::halfblocks(),
        None => Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()),
    });

    let mut terminal = ratatui::init();
    terminal.show_cursor()?;

    App::start(
        terminal,
        vaults,
        initial_vault,
        cli.debug,
        cli.log_level,
        picker,
    )?;

    ratatui::restore();

    Ok(())
}
