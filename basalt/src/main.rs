use clap::Parser;
use std::path::PathBuf;

use basalt_core::obsidian::{self, Error, Vault};
use basalt_tui::{
    app::{App, Startup},
    cli::Cli,
    debug_log,
};
use ratatui::crossterm::{cursor::SetCursorStyle, execute};
use ratatui_image::picker::Picker;

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    debug_log::init();

    let obsidian_config = obsidian::config::load();
    let obsidian_config_error = obsidian_config.as_ref().err().map(ToString::to_string);
    let obsidian_config = obsidian_config.unwrap_or_default();
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

    let initial_file = match cli.path {
        Some(path) => {
            if !path.exists() {
                if let Some(parent) = path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, "")?;
            }
            Some(path.canonicalize()?)
        }
        _ => None,
    };

    // `BASALT_EXP_IMAGE_HALFBLOCKS` forces half-blocks (e.g. for VHS).
    let picker = match std::env::var_os("BASALT_EXP_IMAGE_HALFBLOCKS") {
        Some(_) => Picker::halfblocks(),
        None => Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()),
    };

    let terminal = ratatui::init();

    // Only a main-thread panic restores the terminal; other threads catch theirs.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() == Some("main") {
            default_hook(info);
        } else {
            tracing::error!("background thread panicked: {info}");
        }
    }));

    let result = App::start(Startup {
        terminal,
        vaults,
        initial_vault,
        initial_file,
        debug: cli.debug,
        log_level: cli.log_level,
        theme_override: cli.theme,
        obsidian_config_error,
        picker,
    });

    let _ = execute!(std::io::stdout(), SetCursorStyle::DefaultUserShape);
    ratatui::restore();

    result?;

    Ok(())
}
