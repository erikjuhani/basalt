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

## Contributing to Basalt

[[Basalt]] is ***open for code contributions***, but _primarily_ for bug fixes. Why? Feature work can bring long-term maintenance overhead, and I'd like to keep that to a minimum. One big reason for limiting feature work is that I want to build features myself, as this is a _fun_ side project alongside work, and I would like to keep it that way—to an extent.

However, I do realize that open source projects usually flourish with multiple contributors. Thus, I won't say no if you would like to contribute feature work, but please open an issue first so we can discuss it. This way we can avoid unnecessary effort or bikeshedding over architectural or stylistic choices. I have my own opinions and ideas on how certain things should be written in this project.

> [!info]
> I want this project to feel low-barrier, so don't be discouraged from opening an issue, whether it's about existing features, ideas, or anything else!

### What you can do right now

#### Found a typo?

Open a PR directly with the correction!

#### Found a bug and know how to fix it?

Open a PR with the fix!

#### Found a bug but not sure how to fix it or don't want to do it yourself?

Open an issue with steps to reproduce!

#### Want to contribute a feature?

Open an issue first so we can chat about the feature work or claim an existing issue for yourself!

### How to make your contribution

1. Fork the `basalt` repository
2. Create a branch
3. Open a pull request against basalt's main branch with your changes
4. I'll review your pull request as soon as possible and either leave comments or merge it

If you find mistakes in the documentation or need simple code fixes, please go ahead and open a pull request with the changes!

### Git Pre-push Hook

There's a useful pre-push git hook under `scripts`, which you can enable by running the following command:

```sh
cp scripts/pre-push .git/hooks/
```

The script runs the same test commands as in the `test.yml` workflow.

### CI

> [!missing]
>
> This section is unfinished. It should explain roughly what is being run in the CI and what is required for CI to actually run on a PR opened from a fork.

---

_I will create proper contribution guidelines later, with more details on certain operational aspects of this project._
