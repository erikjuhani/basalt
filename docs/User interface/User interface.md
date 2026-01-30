Basalt's interface is divided into panes, modals, and a status bar.

![[screenshot-ui-overview.png]]

Only one pane has focus at a time. The active pane is indicated by a thicker border, and the status bar at the bottom shows which pane is active. Switch between panes with `Tab` and `Shift+Tab`.

## Panes

### Explorer

The [[Explorer]] is the sidebar on the left side. It displays folders and notes in the current vault.

![[screenshot-explorer.png]]

### Note editor

The [[Note editor]] is the main pane in the center. It displays the selected note with rendered markdown — headings, lists, code blocks, and other elements are rendered with WYSIWYG-style formatting.

![[screenshot-note-editor.png]]

### Outline

The [[Outline]] is the pane on the right side. It lists the headings in the current note and lets you jump to a specific section.

![[screenshot-outline.png]]

## Modals

Modals open on top of the interface. While a modal is open, key mappings for the underlying panes are inactive.

### Help modal

Press `?` to open the help modal. It shows the available key mappings for the currently active pane. Use `j`/`k` or arrow keys to scroll and `Esc` to close.

![[screenshot-help-modal.png]]

### Vault selector modal

Press `Ctrl+g` to open the vault selector. It lists all your Obsidian vaults and lets you switch between them. Use `j`/`k` or arrow keys to navigate, `Enter` to open, and `Esc` to close.

![[screenshot-vault-selector.png]]

### Input modal

The input modal provides text input for operations like renaming. Press `r` in the [[Explorer]] to rename the selected note or directory. The modal opens with the current name — modify it and press `Enter` to confirm or `Esc` to cancel.

When renaming a note, all wiki-links referencing that note are automatically updated across the vault.

| Mapping       | Description                          |
| ------------- | ------------------------------------ |
| `i`           | Enter edit mode for typing           |
| `h` / `←`    | Move cursor left                     |
| `l` / `→`    | Move cursor right                    |
| `Alt+f`       | Move cursor forward by word          |
| `Alt+b`       | Move cursor backward by word         |
| `Backspace`   | Delete character before cursor       |
| `Enter`       | Accept changes and save              |
| `Esc`         | Cancel and close without saving      |

## Status bar

The status bar runs along the bottom of the screen and displays contextual information: the active pane, word and character counts for the current note, and the editing mode when using the [[Editor (experimental)|experimental editor]].
