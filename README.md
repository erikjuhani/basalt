<img align="left" width="125px" src="https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt.png?raw=true"><h3>Basalt&nbsp;&nbsp;</h3>
<p>TUI Application to manage Obsidian notes&nbsp;&nbsp;&nbsp;&nbsp;</p>

<hr>

TUI Application to manage Obsidian vaults and notes directly from the terminal ✨.

![Demo](https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/dark/demo.gif)

Basalt is a cross-platform TUI (Terminal User Interface) for managing Obsidian vaults and notes. It runs on Windows, macOS and Linux. Basalt is not a replacement for Obsidian. Instead, it provides a minimalist terminal interface with a [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) experience.

## Installation

- Using [Homebrew](https://brew.sh):
  ```sh
  brew install erikjuhani/tap/basalt
  ```

- Using [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):
  ```sh
  cargo install basalt-tui
  ```

- Using [aqua](https://aquaproj.github.io/docs/install):
  ```sh
  aqua g -i erikjuhani/basalt
  ```

Or download a pre-compiled binary from the [latest release](https://github.com/erikjuhani/basalt/releases/latest), extract it and move the `basalt` binary to a location in your `PATH`.

## Nightly builds

Nightly builds track the latest `main` commit. They are unstable and meant for testing. The [`nightly` release](https://github.com/erikjuhani/basalt/releases/tag/nightly) always holds the newest build. A downloaded nightly binary reports a `-nightly` suffix in `basalt --version`. A build from source reports the commit hash instead.

- Using [Homebrew](https://brew.sh) (builds from source):
  ```sh
  brew install --HEAD erikjuhani/tap/basalt
  ```

- Using [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (builds from source):
  ```sh
  cargo install --git https://github.com/erikjuhani/basalt --branch main basalt-tui
  ```

Or download a pre-compiled binary from the [nightly release](https://github.com/erikjuhani/basalt/releases/tag/nightly).

## Configuration

Basalt can be customized using a TOML configuration file. The file does not exist by default. Create it manually when you want to override the defaults.

**macOS and Linux:**

- `$HOME/.basalt.toml`
- `$XDG_CONFIG_HOME/basalt/config.toml`

**Windows:**

- `%USERPROFILE%\.basalt.toml`
- `%APPDATA%\basalt\config.toml`

If configuration files exist in multiple locations, only the first one found is used. The home directory configuration takes precedence.

> [!WARNING]
>
> This behavior may change in future versions to merge all found configurations instead.

See the [full configuration reference](https://basalt.page/configuration/configuration/) for key mappings, custom commands and defaults.

## Documentation

The docs live at **[basalt.page](https://basalt.page)**. The sources are in [`docs/`](docs), written as an Obsidian vault, so read them on the site rather than on GitHub, where wiki-links and image embeds do not resolve.

- [Getting started](https://basalt.page/getting-started/installation/): install Basalt and open your first vault
- [User interface](https://basalt.page/user-interface/user-interface/): tabs, panes, modals and navigation
- [Configuration](https://basalt.page/configuration/configuration/): key mappings, custom commands and symbols
- [Editing and Formatting](https://basalt.page/editing-and-formatting/): markdown support and rendering
- [Files and Folders](https://basalt.page/files-and-folders/): working with notes and directories
- [Known Limitations](https://basalt.page/known-limitations/): what is not supported yet

## Contributing

Contributions are welcome, primarily for bug fixes. Feature work is considered on a case-by-case basis. Please open an issue first to discuss.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style and contribution guidelines.

## License

Copyright (c) Erik Kinnunen.

Basalt spans three crates under two licenses:

- **`basalt-tui`** (the application) is [GPL-3.0-or-later](LICENSE-GPL). Distributed modifications must stay open under the same terms.
- **`basalt-core`** and **`basalt-widgets`** (the libraries) are [Apache-2.0](LICENSE-APACHE), so they stay reusable in other projects.

Contributions are accepted under a [Contributor License Agreement](CLA.md). See [CONTRIBUTING.md](CONTRIBUTING.md).
