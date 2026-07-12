Basalt's interface is divided into a tab bar, panes, modals, and a status bar.

![[demo.gif]]

Only one pane has focus at a time. The active pane is indicated by a thicker border, and the status bar at the bottom shows which pane is active. Switch between panes with `Tab` and `Shift+Tab`.

## Tabs

The tab bar runs along the top and lists the notes you have open. Opening a note from the [[Explorer]] focuses its tab if it is already open, otherwise it opens a new one, so every note keeps its own cursor, scroll position and unsaved edits as you move between them.

Cycling to a tab also moves the [[Explorer]] selection to that note, expanding any collapsed folder in its path so the note stays visible. Tabs are sized uniformly and shrink as more open; tabs that share a name are disambiguated by their parent directory.

![[tabs.gif]]

| Mapping             | Description                    |
| ------------------- | ------------------------------ |
| `Ctrl+n` / `Ctrl+p` | Focus the next / previous tab  |
| `L` / `H`           | Focus the next / previous tab  |
| `]b` / `[b`         | Focus the next / previous tab  |
| `Ctrl+w`            | Close the active tab           |

## Panes

### Explorer

The [[Explorer]] is the sidebar on the left side. It displays folders and notes in the current vault.

![[explorer.gif]]

### Note editor

The [[Note editor]] is the main pane in the center. It displays the selected note with rendered markdown — headings, lists, code blocks, and other elements are rendered with WYSIWYG-style formatting.

![[note-editor.gif]]

### Outline

The [[Outline]] is the pane on the right side. It lists the headings in the current note and lets you jump to a specific section.

![[outline.gif]]

## Modals

Modals open on top of the interface. While a modal is open, key mappings for the underlying panes are inactive.

### Help modal

Press `?` to open the help modal. It shows the available key mappings for the currently active pane. Use `j`/`k` or arrow keys to scroll and `Esc` to close.

![[help-modal.gif]]

### Vault selector modal

Press `Ctrl+g` to open the vault selector. It lists all your Obsidian vaults and lets you switch between them. Use `j`/`k` or arrow keys to navigate, `Enter` to open, and `Esc` to close.

![[vault-selector.gif]]

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

### Debug log overlay

Press `g<` to toggle the debug log overlay. It docks to the lower half of the screen and shows the application's tracing output across all levels (trace, debug, info, warn and error), each colored by severity. Capture is always on, so the overlay reflects what has happened up to the moment you open it.

The title shows the active minimum level and the current process memory. The overlay is meant for debugging and troubleshooting. It does not interfere with normal use, so you can open it to inspect activity, then close it and carry on.

![[debug-log.gif]]

| Mapping               | Description                                     |
| --------------------- | ----------------------------------------------- |
| `g<`                  | Toggle the overlay                              |
| `j` / `k` / `↑` / `↓` | Scroll by one line                              |
| `Ctrl+u` / `Ctrl+d`   | Scroll up / down half a page                    |
| `l`                   | Cycle the minimum visible level (trace → error) |
| `c`                   | Clear the captured entries                      |
| `Esc`                 | Close the overlay                               |

The overlay can also be opened on startup with the `--debug` flag, and the initial minimum level set with `--log-level` (for example `basalt --debug --log-level warn`).

## Status bar

The status bar runs along the bottom of the screen and displays contextual information: the active pane, word and character counts for the current note, and the editing mode when using the [[Editor (experimental)|experimental editor]].
