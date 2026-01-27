[[Basalt]] works with your existing Obsidian vaults, displaying notes and folders in the Explorer pane.

## Vault Detection

[[Basalt]] automatically detects [[Obsidian]] vaults by reading [[Obsidian|Obsidian's]] configuration file. The configuration is found at:

| Platform | Location |
|----------|----------|
| macOS | `~/Library/Application Support/obsidian/obsidian.json` |
| Windows | `%APPDATA%\Obsidian\obsidian.json` |
| Linux | `~/.config/obsidian/obsidian.json` |
| Linux (Flatpak) | `~/.var/app/md.obsidian.Obsidian/config/obsidian/obsidian.json` |
| Linux (Snap) | `~/snap/obsidian/current/.config/obsidian/obsidian.json` |

You can override the configuration directory by setting the `OBSIDIAN_CONFIG_DIR` environment variable.

## Switching Vaults

Press <kbd>Ctrl+g</kbd> to open the vault selector and switch between vaults.

## Explorer Navigation

The Explorer pane displays the folder structure of the current vault.

### Key Bindings

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>k</kbd> | Move down/up |
| <kbd>Enter</kbd> | Open note or expand/collapse folder |
| <kbd>h</kbd> | Hide explorer pane |
| <kbd>l</kbd> | Expand explorer pane |
| <kbd>s</kbd> | Toggle sort order |
| <kbd>r</kbd> | Rename selected item |
| <kbd>t</kbd> | Toggle explorer visibility |
| <kbd>Ctrl+b</kbd> | Toggle explorer visibility |

### Sorting

Press <kbd>s</kbd> to toggle between ascending and descending sort order. Folders always appear before files.

## File Types

[[Basalt]] recognizes the following:

- **Notes**: Files with `.md` extension
- **Folders**: Directories containing notes or other folders

Hidden files and folders (names starting with `.`) are not displayed.

## Renaming

To rename a note or folder:

1. Select the item in the Explorer
2. Press <kbd>r</kbd> to open the rename dialog
3. Edit the name
4. Press <kbd>Enter</kbd> to confirm or <kbd>Esc</kbd> to cancel

When renaming a note, all [[Links|wiki-links]] referencing that note are automatically updated throughout the vault.

### Rename Dialog Keys

| Key | Action |
|-----|--------|
| <kbd>Enter</kbd> | Confirm rename |
| <kbd>Esc</kbd> | Cancel |
| <kbd>←</kbd> / <kbd>→</kbd> | Move cursor |
| <kbd>Ctrl+←</kbd> / <kbd>Ctrl+→</kbd> | Move by word |
| <kbd>Ctrl+Backspace</kbd> | Delete word |

## Current Limitations

The following operations are not yet supported:

- Creating new notes or folders
- Deleting notes or folders
- Moving notes or folders
- Copying notes or folders

See [[Known Limitations]] for a complete list.
