This chapter covers installation and first steps with [[Basalt]].

## Prerequisites

[[Basalt]] works with existing [[Obsidian]] vaults. Before installing, ensure you have:

- [Obsidian](https://obsidian.md/) installed with at least one vault configured
- A terminal emulator with Unicode support, for example:
  - [Alacritty](https://alacritty.org/)
  - [kitty](https://sw.kovidgoyal.net/kitty/)
  - [WezTerm](https://wezfurlong.org/wezterm/)
  - [Ghostty](https://ghostty.org/)
  - [foot](https://codeberg.org/dnkl/foot)
  - [iTerm2](https://iterm2.com/) (macOS)
  - [Windows Terminal](https://github.com/microsoft/terminal) (Windows)

## Installation

### Using `cargo`

If you have Rust installed, you can install [[Basalt]] with:

```sh
cargo install basalt-tui
```

### Pre-compiled Binaries

Download pre-compiled binaries from the [GitHub releases page](https://github.com/erikjuhani/basalt/releases):

1. Download the archive for your platform and architecture
2. Extract the archive contents
3. Move the `basalt` binary to a location in your system PATH, or run it directly

## First Launch

Start Basalt by running:

```sh
basalt
```

On first launch, the **splash screen** displays all Obsidian vaults found on your system. Basalt automatically detects vaults from Obsidian's configuration.

### Selecting a Vault

Use <kbd>j</kbd>/<kbd>k</kbd> or arrow keys to navigate the vault list, then press <kbd>Enter</kbd> to open a vault.

## Basic Navigation

Once inside a vault, the interface is divided into three panes.

| Pane | Position | Purpose |
|------|----------|---------|
| Explorer | Left | Browse folders and notes |
| Note Editor | Center | View and edit notes |
| Outline | Right | Navigate by headings |

### Switching Panes

- <kbd>Tab</kbd> - Switch to next pane
- <kbd>Shift+Tab</kbd> - Switch to previous pane

### Within a Pane

- <kbd>j</kbd> / <kbd>k</kbd> - Move up/down
- <kbd>Enter</kbd> - Select/open item
- <kbd>?</kbd> - Open help modal

## Opening a Note

1. Navigate to a note in the Explorer pane
2. Press <kbd>Enter</kbd> to open it
3. The note content appears in the Note Editor pane
4. The Outline pane shows the document structure

## Common Keys

| Key | Action |
|-----|--------|
| <kbd>?</kbd> | Show help |
| <kbd>Ctrl+g</kbd> | Switch vault |
| <kbd>q</kbd> | Quit Basalt |
| <kbd>r</kbd> | Rename selected item (in Explorer) |

## Next Steps

- [[User Interface]] - Learn about all UI components
- [[Configuration]] - Customize key bindings
- [[Editing and Formatting]] - Understand markdown rendering
