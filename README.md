<img align="left" width="125px" src="https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt.png?raw=true"><h3>Basalt&nbsp;&nbsp;</h3>
<p>TUI Application to manage Obsidian notes&nbsp;&nbsp;&nbsp;&nbsp;</p>

<hr>

TUI Application to manage Obsidian vaults and notes directly from the terminal ✨.

![Demo](https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt_demo.gif)

Basalt is a cross-platform TUI (Terminal User Interface) for managing Obsidian vaults and notes. It runs on Windows, macOS, and Linux. Basalt is not a replacement for Obsidian. Instead, it provides a minimalist terminal interface with a [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) experience.

## Installation

Install Basalt using [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```sh
cargo install basalt-tui
```

Or download a pre-compiled binary from the [latest release](https://github.com/erikjuhani/basalt/releases/latest), extract it, and move the `basalt` binary to a location in your `PATH`.

## Configuration

Basalt can be customized using a TOML configuration file. The file does not exist by default — create it manually when you want to override the defaults.

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

See the [full configuration reference](docs/Configuration/Configuration.md) for key mappings, custom commands, and defaults.

## Documentation

- [Getting started](docs/Getting%20started/Installation.md)
- [User interface](docs/User%20interface/User%20interface.md)
- [Configuration](docs/Configuration/Configuration.md)
- [Editing and Formatting](docs/Editing%20and%20Formatting.md)
- [Files and Folders](docs/Files%20and%20Folders.md)
- [Known Limitations](docs/Known%20Limitations.md)

## Contributing

Contributions are welcome, primarily for bug fixes. Feature work is considered on a case-by-case basis — please open an issue first to discuss.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and contribution guidelines.
