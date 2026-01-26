The Basalt user interface is built around a multi-pane layout optimized for keyboard navigation. When launched, Basalt opens the splash screen where you can select a vault to open.

## Layout

The main interface is divided into three primary panes arranged horizontally:

| Pane | Position | Purpose |
|------|----------|---------|
| Explorer | Left | Browse folders and notes in the vault |
| Note Editor | Center | View and edit the selected note |
| Outline | Right | Navigate the document by headings |

Only one pane is active at a time and receives keyboard input. Switch between panes using <kbd>Tab</kbd> (next) or <kbd>Shift+Tab</kbd> (previous), or use <kbd>h</kbd>/<kbd>l</kbd> for vim-style navigation.

## Panes

### Explorer (Sidebar)

The Explorer displays the folder structure and notes within the selected vault. It provides a tree-like view for navigating your notes.

**Key features:**
- Navigate with <kbd>j</kbd>/<kbd>k</kbd> or arrow keys
- Open a note by pressing <kbd>Enter</kbd>
- Expand/collapse folders with <kbd>Enter</kbd>
- Rename items with <kbd>r</kbd> (opens the input modal)
- Toggle visibility with <kbd>e</kbd>
- Sort items with <kbd>s</kbd>

When renaming a note, all wiki-links (`[[note-name]]`) referencing that note are automatically updated throughout the vault.

### Note Editor

The Note Editor is the central pane where you view and edit notes. It renders markdown with formatting applied, providing a readable view of your content.

**Key features:**
- Scroll with <kbd>j</kbd>/<kbd>k</kbd> or arrow keys
- Jump to top/bottom with <kbd>g</kbd>/<kbd>G</kbd>
- Page up/down with <kbd>Ctrl+u</kbd>/<kbd>Ctrl+d</kbd>
- Open the experimental editor with <kbd>i</kbd> (if enabled)
- Open in external editor with <kbd>o</kbd>

See [[Editor (experimental)]] for details on the built-in editing capabilities.

### Outline

The Outline pane displays the heading structure of the current note, allowing quick navigation to different sections.

**Key features:**
- Navigate headings with <kbd>j</kbd>/<kbd>k</kbd> or arrow keys
- Jump to a heading by pressing <kbd>Enter</kbd>
- Toggle visibility with <kbd>o</kbd>
- The outline automatically syncs with the cursor position in the editor

## Components

### Status Bar

The status bar runs along the bottom of the screen and displays contextual information:

- **Active pane**: Shows which pane currently has focus
- **Word count**: Number of words in the current note
- **Character count**: Number of characters in the current note
- **Mode indicator**: Shows the current editing mode (when using the experimental editor)

## Modals

Modals are overlay dialogs that appear on top of the main interface for specific interactions.

### Help Modal

Access the help modal by pressing <kbd>?</kbd> from any pane. It displays:

- Available key bindings for the current context
- Pane-specific commands
- Global shortcuts

Press <kbd>Esc</kbd> or <kbd>?</kbd> again to close.

### Vault Selector Modal

Press <kbd>Ctrl+g</kbd> to open the vault selector, which displays all available Obsidian vaults. Navigate with <kbd>j</kbd>/<kbd>k</kbd> and select with <kbd>Enter</kbd>. This allows switching between vaults without restarting Basalt.

### Input Modal

The input modal appears when performing text input operations, such as renaming files or folders. It provides a simple text editor with:

- Cursor navigation with arrow keys
- Word-based movement with <kbd>Ctrl+Left</kbd>/<kbd>Ctrl+Right</kbd>
- Delete word with <kbd>Ctrl+Backspace</kbd>
- Confirm with <kbd>Enter</kbd>
- Cancel with <kbd>Esc</kbd>

Access it by pressing <kbd>r</kbd> on a selected item in the explorer.

## Customization

All key bindings can be customized through the [[Configuration]] file. See the configuration documentation for details on remapping keys and adding custom commands.
