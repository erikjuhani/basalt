[[Basalt]] works with your existing Obsidian vaults, displaying notes and folders in the [[Explorer]] pane.

## Vault detection

[[Basalt]] automatically detects Obsidian vaults by reading Obsidian's configuration file:

- **macOS**: `~/Library/Application Support/obsidian/obsidian.json`
- **Windows**: `%APPDATA%\Obsidian\obsidian.json`
- **Linux**: `~/.config/obsidian/obsidian.json`
- **Linux (Flatpak)**: `~/.var/app/md.obsidian.Obsidian/config/obsidian/obsidian.json`
- **Linux (Snap)**: `~/snap/obsidian/current/.config/obsidian/obsidian.json`

You can override the configuration directory by setting the `OBSIDIAN_CONFIG_DIR` environment variable.

## Switching vaults

Press `Ctrl+g` to open the vault selector and switch between vaults.

## File types

- **Notes**: Files with `.md` extension
- **Folders**: Directories containing notes or other folders

Hidden files and folders (names starting with `.`) are not displayed.

## Sorting

Press `s` in the [[Explorer]] to toggle between ascending and descending sort order. Folders always appear before files.

## Renaming

Select an item in the [[Explorer]] and press `r` to open the rename dialog. Modify the name and press `Enter` to confirm or `Esc` to cancel.

When renaming a note, all wiki-links referencing that note are automatically updated throughout the vault.

## Current limitations

The following file operations are not yet supported:

- Creating new notes or folders
- Deleting notes or folders
- Moving notes or folders
- Copying notes or folders
