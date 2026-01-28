<img align="left" width="125px" src="https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt.png?raw=true"><h3>Basalt&nbsp;&nbsp;</h3>
<p>TUI Application to manage Obsidian notes&nbsp;&nbsp;&nbsp;&nbsp;</p>

<hr>

TUI Application to manage Obsidian vaults and notes directly from the terminal ✨.

![Demo](https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt_demo.gif)

Basalt is a cross-platform TUI (Terminal User Interface) for managing Obsidian vaults and notes. It runs on Windows, macOS, and Linux.

Basalt is not a replacement for Obsidian. Instead, it provides a minimalist terminal interface with readable markdown rendering and a [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) experience.

## Vision

- Basalt functions as a companion app for Obsidian that enables quick note editing without interrupting the terminal flow
- Basalt enables text editing in a familiar way (Obsidian, vim) without having to rely on external editors
- Basalt is a terminal based [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) markdown editor
- Basalt works as a CLI for finding / deleting / creating notes and works with the rest of the unix tooling
- Basalt is a standalone terminal note managing application that works seamlessly with Obsidian

## Installation

Install basalt using cargo:

```sh
cargo install basalt-tui
```

Or use the precompiled binaries from the latest basalt release.

## Background

Basalt was created to bridge the gap between terminal-based workflows and Obsidian's visual note-taking experience. While tools like [obsidian.nvim](https://github.com/epwalsh/obsidian.nvim) provide Obsidian integration within Neovim, Basalt takes a different approach: a dedicated TUI that renders markdown visually while keeping you in the terminal.

The goal is not to replace Obsidian, but to complement it. Basalt provides quick access to your notes from anywhere in the terminal without breaking your workflow.

## Configuration

Basalt can be customized using a configuration file located in one of the following directories:

**macOS and Unix:**

- `$HOME/.basalt.toml`
- `$XDG_CONFIG_HOME/basalt/config.toml`

**Windows:**

- `%USERPROFILE%\.basalt.toml`
- `%APPDATA%\basalt\config.toml`

If configuration files exist in multiple locations, only the first one found is used. The home directory configuration takes precedence.

> [!WARNING]
>
> This behavior may change in future versions to merge all found configurations instead.

### Key Mappings

Key mappings can be modified or extended in the configuration file.

Each key mapping is associated with a specific pane and becomes active when that pane has focus. The `[global]` section applies to all panes and is evaluated first.

### Default configuration

See [here](https://github.com/erikjuhani/basalt/blob/main/docs/Configuration.md#default-configuration).

## Contributing

Contributions are welcome, primarily for bug fixes. Feature work is considered on a case-by-case basis—please open an issue first to discuss.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and contribution guidelines.

