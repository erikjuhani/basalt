> [!WARNING]
>
> The current implementation of the Basalt note editor is _experimental_ and _subject to change_.
>
> The editor is built from scratch with limited capabilities. More features will be added incrementally.

To enable the experimental editor feature, you must add the following configuration to your [[Basalt]] [[configuration]] file:

```toml
experimental_editor = true
```

## View

[[Basalt]] follows loosely the Obsidian views and modes: [[Reading view]], [[Edit view]], and [[Source Mode editing]] when in [[Edit view]]. Live Preview editing is not yet supported.

### Reading View (READ)

Renders the note without Markdown syntax, similar in function to Obsidian's [[Reading view]].

#### Key Mappings

| Mapping  | Description                  |
| -------- | ---------------------------- |
| `↑`      | Move cursor up by one line   |
| `↓`      | Move cursor down by one line |
| `Ctrl+D` | Scroll down by half a page   |
| `Ctrl+U` | Scroll up by half a page     |
| `Ctrl+E` | Toggle to edit mode          |

### Edit View (EDIT)

[[Edit view]] displays the markdown source exactly as written, and allows you to make changes to the notes.

The editor uses a custom implementation with limited capabilities. Currently supported operations:

- Character insertion
- Character deletion (backspace)
- Navigation (arrow keys, word jumping)
- Newline insertion (Enter)

#### Key Mappings

> [!WARNING]
>
> Edit view key mappings cannot be modified at the moment.

| Mapping     | Description                          |
| ----------- | ------------------------------------ |
| `Backspace` | Delete one character before cursor   |
| `Enter`     | Insert newline                       |
| `→`         | Move cursor forward by one character |
| `←`         | Move cursor backward by one character|
| `↑`         | Move cursor up by one line           |
| `↓`         | Move cursor down by one line         |
| `Alt+→`     | Move cursor forward by word          |
| `Alt+←`     | Move cursor backward by word         |
| `Ctrl+E`    | Toggle to read mode                  |
| `Esc`       | Exit edit mode                       |

#### Limitations

The current editor implementation has the following limitations:

- No undo/redo support
- No clipboard operations (copy/paste/cut)
- No text selection
- No multi-line deletion
- No line/word deletion commands
- No jumping to start/end of line or document
- Single block editing only (navigating between blocks switches the active edit buffer)
