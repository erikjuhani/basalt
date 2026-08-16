## Installation

[[Basalt]] is available to install via [Homebrew](https://brew.sh), [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html), [aqua](https://aquaproj.github.io/docs/install) and as pre-compiled binaries from [GitHub releases](https://github.com/erikjuhani/basalt/releases).

### Homebrew

```
brew install erikjuhani/tap/basalt
```

### Cargo

```
cargo install basalt-tui
```

### aqua

```
aqua g -i erikjuhani/basalt
```

### Pre-compiled binaries

Download the appropriate archive for your system and architecture from [GitHub releases](https://github.com/erikjuhani/basalt/releases), extract it and move the `basalt` binary to a location in your `PATH`.

## Nightly builds

Nightly builds track the latest `main` commit. They are unstable and meant for testing. The [nightly release](https://github.com/erikjuhani/basalt/releases/tag/nightly) always holds the newest build. A downloaded nightly binary reports a `-nightly` suffix in `basalt --version`. A build from source reports the commit hash instead.

### Homebrew

Build the latest `main` from source:

```
brew install --HEAD erikjuhani/tap/basalt
```

### Cargo

Build the latest `main` from source:

```
cargo install --git https://github.com/erikjuhani/basalt --branch main basalt-tui
```

### Pre-compiled nightly binaries

Download the archive for your system and architecture from the [nightly release](https://github.com/erikjuhani/basalt/releases/tag/nightly), extract it and move the `basalt` binary to a location in your `PATH`.

## Starting Basalt

Once installed, launch `basalt` from your terminal:

```
basalt
```

[[Basalt]] opens in the splash screen, showing a list of your Obsidian vaults discovered automatically from Obsidian's configuration. Use `j`/`k` or arrow keys to navigate and `Enter` to open a vault.

![[vault-selector.gif]]

## Navigating a vault

Once inside a vault, the interface is divided into three panes:

- **[[Explorer]]** on the left — browse folders and notes
- **[[Note editor]]** in the center — view the selected note with rendered markdown
- **[[Outline]]** on the right — navigate headings in the current note

Use `Tab` and `Shift+Tab` to move focus between panes. The status bar at the bottom shows which pane is active.

## Opening a note

In the [[Explorer]], use `j`/`k` or arrow keys to move through the file list. Press `Enter` to open a note in the [[Note editor]].

## Switching vaults

Press `Space` then `v` to open the vault selector and switch to a different vault.

## Getting help

Press `?` to open the help modal. It shows the available key mappings for the currently active pane.

For more on the interface, see [[User interface]].
