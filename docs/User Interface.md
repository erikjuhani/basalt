Basalt uses a multi-pane layout with keyboard-driven navigation. On launch, the splash screen appears where you select a vault to open.

## Splash Screen

On startup, you can select the vault you want to view. Any open vaults are shown with a **◆** symbol marker.

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>↓</kbd> | Move selection down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Move selection up |
| <kbd>Enter</kbd> | Open selected vault |
| <kbd>q</kbd> | Quit |
| <kbd>?</kbd> | Show help |

The vault selection can be brought up as a modal by pressing <kbd>Ctrl+g</kbd> after the startup screen.

## Layout

The main interface is divided into three primary panes arranged horizontally:

| Pane | Position | Purpose |
|------|----------|---------|
| Explorer | Left | Browse folders and notes in the vault |
| Note Editor | Center | View and edit the selected note |
| Outline | Right | Navigate the document by headings |

Only one pane is active at a time and receives keyboard input. The currently active pane is highlighted and its name is displayed in the lower left corner of the application.

### Switching Panes

| Key | Action |
|-----|--------|
| <kbd>Tab</kbd> | Switch to next pane |
| <kbd>Shift+Tab</kbd> | Switch to previous pane |

## Panes

### Explorer

The Explorer shows all notes and folders in your vault. Navigate the list with <kbd>j</kbd>/<kbd>k</kbd> and press <kbd>Enter</kbd> to open a note. The pane can be toggled to give more space to the note editor.

The Explorer pane can be expanded and hidden. When expanded, it displays a **⟹** symbol indicator.

When renaming a note, all wiki-links referencing that note are automatically updated throughout the vault.

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>↓</kbd> | Move selection down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Move selection up |
| <kbd>Enter</kbd> | Open note or expand/collapse folder |
| <kbd>s</kbd> | Toggle note sorting |
| <kbd>r</kbd> | Rename selected note or directory |
| <kbd>t</kbd> | Toggle explorer panel visibility |
| <kbd>h</kbd> / <kbd>←</kbd> | Hide explorer panel |
| <kbd>l</kbd> / <kbd>→</kbd> | Expand explorer panel |
| <kbd>Ctrl+b</kbd> | Toggle explorer panel visibility |
| <kbd>Ctrl+u</kbd> | Scroll up half a page |
| <kbd>Ctrl+d</kbd> | Scroll down half a page |
| <kbd>Ctrl+o</kbd> | Toggle outline pane |
| <kbd>Ctrl+g</kbd> | Toggle vault selector modal |
| <kbd>q</kbd> | Quit |
| <kbd>?</kbd> | Show help |

### Note Editor

View and navigate through your notes. When the experimental editor is enabled, the note editor supports multiple modes.

#### View Mode (Default)

View mode displays the rendered markdown content with navigation support. This is the default mode for reading notes.

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>↓</kbd> | Move cursor down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Move cursor up |
| <kbd>t</kbd> | Toggle explorer panel visibility |
| <kbd>Ctrl+b</kbd> | Toggle explorer panel visibility |
| <kbd>Ctrl+u</kbd> | Scroll up half a page |
| <kbd>Ctrl+d</kbd> | Scroll down half a page |
| <kbd>Ctrl+o</kbd> | Toggle outline pane |
| <kbd>Ctrl+g</kbd> | Toggle vault selector modal |
| <kbd>q</kbd> | Quit |
| <kbd>?</kbd> | Show help |

#### Experimental Editor Keys

When the experimental editor is enabled:

| Key | Action |
|-----|--------|
| <kbd>i</kbd> | Enter edit mode |
| <kbd>Shift+r</kbd> | Enter read mode |
| <kbd>Ctrl+x</kbd> | Save note |
| <kbd>Esc</kbd> | Exit current mode |
| <kbd>h</kbd> / <kbd>←</kbd> | Move cursor left |
| <kbd>l</kbd> / <kbd>→</kbd> | Move cursor right |
| <kbd>Alt+f</kbd> / <kbd>Alt+→</kbd> | Move cursor forward by word |
| <kbd>Alt+b</kbd> / <kbd>Alt+←</kbd> | Move cursor backward by word |

See [[Editor (experimental)]] for details on the built-in editing capabilities.

### Outline

The Outline shows all headings in the current note. Navigate with <kbd>j</kbd>/<kbd>k</kbd> and press <kbd>Enter</kbd> to expand or collapse a heading. Press <kbd>g</kbd> to jump to the selected heading in the document. The pane can be toggled to give more space to the note editor.

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>↓</kbd> | Move selection down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Move selection up |
| <kbd>g</kbd> | Move editor cursor to selected heading |
| <kbd>Enter</kbd> | Expand or collapse heading |
| <kbd>Ctrl+o</kbd> | Toggle outline pane visibility |
| <kbd>Ctrl+b</kbd> | Toggle explorer pane visibility |
| <kbd>Ctrl+g</kbd> | Toggle vault selector modal |
| <kbd>q</kbd> | Quit |
| <kbd>?</kbd> | Show help |

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

Press <kbd>?</kbd> from any pane to open the help modal. It displays available key bindings, pane-specific commands, and global shortcuts.

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>↓</kbd> | Scroll down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Scroll up |
| <kbd>Ctrl+u</kbd> | Scroll up half a page |
| <kbd>Ctrl+d</kbd> | Scroll down half a page |
| <kbd>?</kbd> / <kbd>Esc</kbd> | Close help |

### Vault Selector Modal

Press <kbd>Ctrl+g</kbd> to open the vault selector, which displays all available Obsidian vaults.

| Key | Action |
|-----|--------|
| <kbd>j</kbd> / <kbd>↓</kbd> | Move selection down |
| <kbd>k</kbd> / <kbd>↑</kbd> | Move selection up |
| <kbd>Enter</kbd> | Open selected vault |
| <kbd>Esc</kbd> | Close modal |

### Input Modal

The input modal appears when renaming notes or directories. It provides a text input field with basic editing capabilities.

| Key | Action |
|-----|--------|
| <kbd>i</kbd> | Enter edit mode for typing |
| <kbd>h</kbd> / <kbd>←</kbd> | Move cursor left |
| <kbd>l</kbd> / <kbd>→</kbd> | Move cursor right |
| <kbd>Alt+f</kbd> | Move cursor forward by word |
| <kbd>Alt+b</kbd> | Move cursor backward by word |
| <kbd>Backspace</kbd> | Delete character before cursor |
| <kbd>Enter</kbd> | Accept changes and save |
| <kbd>Esc</kbd> | Cancel and close without saving |

#### Usage

When renaming a note or directory from the explorer:

1. Press <kbd>r</kbd> on the selected item
2. The input modal will appear with the current name
3. Press <kbd>i</kbd> to enter edit mode
4. Modify the name as needed
5. Press <kbd>Enter</kbd> to save or <kbd>Esc</kbd> to cancel

## Customization

Key bindings can be modified or extended in the [[Configuration]] file. Each key mapping is associated with a specific pane and becomes active when that pane has focus. The global section applies to all panes and is evaluated first.
