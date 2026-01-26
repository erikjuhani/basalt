<img align="left" width="125px" src="https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt.png?raw=true"><h3>Basalt&nbsp;&nbsp;</h3>
<p>TUI Application to manage Obsidian notes&nbsp;&nbsp;&nbsp;&nbsp;</p>

<hr>

TUI Application to manage Obsidian vaults and notes directly from the terminal ✨.

![Demo](https://raw.githubusercontent.com/erikjuhani/basalt/refs/heads/main/assets/basalt_demo.gif)

Basalt is a TUI (Terminal User Interface) application to manage Obsidian vaults and notes from the terminal. Basalt is cross-platform and can be installed and run in the major operating systems on Windows, macOS; and Linux.

Basalt is not a complete or comprehensive replacement for Obsidian, but instead a minimalist approach for note management in terminal with a readable markdown rendering and [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) experience.

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

This is something that has been brewing in my head for quite some time. There has been different incarnations over the years, however, nothing as substantial as this.

I have been using Neovim and the official Obsidian app. However, I wanted to have something dedicated that offers the same writing experience as Neovim, but has more WYSIWYG experience as in the official Obsidian app. I'm fully aware of [obsidian.nvim](https://github.com/epwalsh/obsidian.nvim), which many people use and find more than sufficient. However, I want to see images, beautified text, note graphs, etc. I want it to be a bit more.

The problem for me personally is that when I leave the terminal, my flow breaks, especially if I'm writing. Using an entirely different app disrupts that flow, and it _annoys_ me. So here I am, building a TUI for Obsidian.

The goal of basalt is not to replace the Obsidian app. Basalt is to fill and cater a need to have a terminal view to the selection of notes and vaults, providing quick access from anywhere in the terminal with a simple command.

## Configuration

[[Basalt]] features and functionality can be customized using a user-defined configuration file. The configuration file should be located in one of the following directories:

**macOS and Unix:**

- `$HOME/.basalt.toml`
- `$XDG_CONFIG_HOME/basalt/config.toml`

**Windows:**

- `%USERPROFILE%\.basalt.toml`
- `%APPDATA%\basalt\config.toml`

If configuration files exist in multiple locations, only the first one found will be used, with the home directory configuration taking precedence. 

> [!WARNING]
>
> This behavior may change in future versions to merge all found configurations instead.

### Key Mappings

Basalt key mappings can be modified or extended by defining key mappings in the user configuration file.

Each key mapping is associated with a specific 'pane' and becomes active when that pane has focus. The global section applies to all panes and is evaluated first.

### Default configuration

See [here](https://github.com/erikjuhani/basalt/blob/main/docs/Configuration.md#default-configuration).

## Contributing

Contributions are welcome, primarily for bug fixes. Feature work is considered on a case-by-case basis—please open an issue first to discuss.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and contribution guidelines.

