> [!WARNING]
>
> The current implementation of the Basalt note editor is _experimental_ and _subject to change_.
>
> The editor is built from scratch with limited capabilities. More features will be added incrementally.

The experimental editor is disabled by default. To enable it, add the following to your [[Configuration]] file:

```toml
experimental_editor = true
```

## Views

Basalt follows Obsidian's view model: Reading view and Edit view. Live Preview is not yet supported.

### Reading View (READ)

Renders the note without Markdown syntax, similar in function to Obsidian's Reading view. This is the default mode for viewing notes.

| Key | Action |
|-----|--------|
| <kbd>↑</kbd> / <kbd>k</kbd> | Move cursor up by one line |
| <kbd>↓</kbd> / <kbd>j</kbd> | Move cursor down by one line |
| <kbd>Ctrl+u</kbd> | Scroll up by half a page |
| <kbd>Ctrl+d</kbd> | Scroll down by half a page |
| <kbd>i</kbd> | Enter edit mode |
| <kbd>Ctrl+e</kbd> | Toggle to edit mode |

### Edit View (EDIT)

Edit view displays the markdown source and allows you to make changes.

The editor uses a custom implementation with limited capabilities.

#### Supported Operations

- Character insertion
- Character deletion (backspace only)
- Navigation (arrow keys, word jumping)
- Newline insertion (Enter)

#### Cursor Movement

| Key | Action |
|-----|--------|
| <kbd>→</kbd> | Move forward one character |
| <kbd>←</kbd> | Move backward one character |
| <kbd>↑</kbd> | Move up one line |
| <kbd>↓</kbd> | Move down one line |
| <kbd>Alt+→</kbd> | Move forward by word |
| <kbd>Alt+←</kbd> | Move backward by word |

#### Editing Commands

| Key | Action |
|-----|--------|
| <kbd>Backspace</kbd> | Delete character before cursor |
| <kbd>Enter</kbd> | Insert newline |
| <kbd>Ctrl+x</kbd> | Save note |
| <kbd>Ctrl+e</kbd> | Toggle to read mode |
| <kbd>Shift+r</kbd> | Switch to read mode |
| <kbd>Esc</kbd> | Exit edit mode |

> [!WARNING]
>
> Edit view key mappings cannot be modified through configuration.

#### Limitations

The current editor implementation has the following limitations:

- No undo/redo support
- No clipboard operations (copy/paste/cut)
- No text selection
- No multi-line deletion
- No line/word deletion commands
- No jumping to start/end of line or document
- Single block editing only (navigating between blocks switches the active edit buffer)

## See Also

- [[User Interface]] - Complete key binding reference
- [[Known Limitations]] - Full list of current limitations
