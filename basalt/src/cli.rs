use clap::Parser;

use crate::{debug_log::LogLevel, version};

const VERSION_INFO: version::VersionInfo = version::VersionInfo::from_env();

#[derive(Parser)]
#[command(name = "basalt", version = VERSION_INFO.to_string())]
pub struct Cli {
    /// Open the debug log overlay on startup
    #[arg(long)]
    pub debug: bool,

    /// Minimum log level shown in the debug log overlay
    #[arg(long, value_enum, default_value_t = LogLevel::Trace)]
    pub log_level: LogLevel,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::{cli::Cli, version};

    #[test]
    fn version_output() {
        let help = Cli::command()
            .version(
                version::VersionInfo {
                    version: "0.12.5",
                    hash: Some("abc123def0123456789"),
                    short_hash: Some("abc123def"),
                    date: Some("2026-05-15"),
                }
                .to_string(),
            )
            .render_version()
            .to_string();
        insta::assert_snapshot!(help)
    }

    #[test]
    fn help_output() {
        let help = Cli::command().render_help().to_string();
        insta::assert_snapshot!(help)
    }
}
