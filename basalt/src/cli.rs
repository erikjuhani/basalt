use clap::Parser;

use crate::version;

const VERSION_INFO: version::VersionInfo = version::VersionInfo::from_env();

#[derive(Parser)]
#[command(name = "basalt", version = VERSION_INFO.to_string())]
pub struct Cli {}

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
                    short_hash: "abc123def",
                    date: "2026-05-15",
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
