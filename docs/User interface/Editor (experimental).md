> [!WARNING]
>
> The editor is _experimental_ and _subject to change_. It is built from scratch with limited capabilities. More features will be added incrementally.

To enable the experimental editor, add the following to your [[Configuration]] file:

```toml
experimental_editor = true
```

![[note-editor.gif]]

## Views

[[Basalt]] follows Obsidian's view model with a reading view and an edit view.

### Reading view

Renders the note without markdown syntax, similar to Obsidian's reading view.

| Mapping    | Description                |
| ---------- | -------------------------- |
| `↑`        | Move cursor up             |
| `↓`        | Move cursor down           |
| `Ctrl+D`   | Scroll down half page      |
| `Ctrl+U`   | Scroll up half page        |
| `i`        | Switch to edit view        |
| `Ctrl+E`   | Toggle to edit view        |

### Edit view

Displays the raw markdown source and allows editing.

> [!WARNING]
>
> Edit view key mappings cannot be modified at the moment.

| Mapping     | Description                          |
| ----------- | ------------------------------------ |
| `Backspace` | Delete one character before cursor   |
| `Enter`     | Insert newline                       |
| `→`         | Move cursor forward                  |
| `←`         | Move cursor backward                 |
| `↑`         | Move cursor up                       |
| `↓`         | Move cursor down                     |
| `Alt+→`     | Move cursor forward by word          |
| `Alt+←`     | Move cursor backward by word         |
| `Ctrl+X`    | Save note                            |
| `Ctrl+E`    | Toggle to read view                  |
| `Shift+R`   | Switch to read view                  |
| `Esc`       | Exit edit mode                       |

### Limitations

The edit view edits the whole note line by line. With [[Configuration|vim mode]] it also supports motions, operators (delete, change, yank, paste), visual (line and block) selection, undo/redo and jumps to the start and end of the line and document.

- Pasting images from the clipboard is not supported
