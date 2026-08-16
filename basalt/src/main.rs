use clap::Parser;
use std::path::PathBuf;

use basalt_core::obsidian::{self, Error, Vault};
use basalt_tui::{app::App, cli::Cli, debug_log};
use ratatui::crossterm::{cursor::SetCursorStyle, execute};

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

    let terminal = ratatui::init();

    let result = App::start(
        terminal,
        vaults,
        initial_vault,
        cli.debug,
        cli.log_level,
        cli.theme,
    );

    let _ = execute!(std::io::stdout(), SetCursorStyle::DefaultUserShape);
    ratatui::restore();

    result?;

    Ok(())
}
