use std::path::PathBuf;

use basalt_core::obsidian::{self, Error, Vault};
use basalt_tui::app::App;

fn main() -> Result<(), Error> {
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

    let mut terminal = ratatui::init();
    terminal.show_cursor()?;

    App::start(terminal, vaults, initial_vault)?;

    ratatui::restore();

    Ok(())
}
