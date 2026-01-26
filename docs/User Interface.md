
> [!CAUTION]
> 
> This documentation is still work in progress and is missing a lot of information and screenshots of the UI panes and components.

Basalt is always booted up in the 'splash' screen, where users can pick a vault from a list of available vaults to be opened.

## Panes

Basalt user interface is divided into different panes; modals and components.

### Explorer (Sidebar)

Explorer is shown on the left side and displays the folders and notes under the selected vault.

The explorer supports renaming both notes and directories using the input modal. Select an item and press <kbd>r</kbd> to open the rename dialog.

### Note editor

Note editor is the 'main' pane that is used to view and modify the selected note.

### Outline

The Outline is the rightmost pane that allows navigation using the headings of the document.

## Components

### Status bar

The status bar shows bits of helpful information at the bottom of the screen, which includes the currently selected pane; and amount of words and characters.

## Modals

Modals are UI components that can be opened on top of existing active panes or other components.

### Help Modal

Help modal can be accessed by pressing <kbd>?</kbd>. Help modal contains the essential information of each pane and key mappings.

### Vault Selector Modal

Vault selector modal can be accessed by pressing <kbd>Ctrl+g</kbd>, which lets you select another vault from the list of available vaults.

### Input Modal

The input modal provides text input capabilities for interactive operations like renaming notes and directories. It features a custom text editor with cursor navigation and word-based movement. Access it by pressing <kbd>r</kbd> on a selected item in the explorer. When renaming a note, all wiki-links referencing that note are automatically updated across the vault.
