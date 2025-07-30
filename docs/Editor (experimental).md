
> [!WARNING]
>
> The current implementation of the Basalt note editor is _experimental_ and _subject to change_.
>
> The key change I anticipate for the editor is a custom implementation built from scratch that will enable a better [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) experience.

To enable the experimental editor feature, you must add the following configuration to your Basalt configuration file:

```toml
experimental_editor = true
```

## Modes

> [!IMPORTANT]
>
> In the future, the modes will follow the Obsidian modes more strictly: Reading, Live Preview Editing, and Source Mode Editing.

### Read Mode

Renders the note without Markdown syntax, similar in function to Obsidian's Reading view.

#### Key Mappings

|Mapping|Description|
|---|---|
|`↑`|Move cursor up by one line|
|`↓`|Move cursor down by one line|
|`Ctrl+D`|Scroll down by half a page|
|`Ctrl+U`|Scroll up by half a page|

### View Mode

View mode displays the source of the Markdown node directly under the cursor, making editing easier. View is the default display mode.

#### Key Mappings

| Mapping  | Description                           |
| -------- | ------------------------------------- |
| `→`      | Move cursor forward by one character  |
| `←`      | Move cursor backward by one character |
| `↑`      | Move cursor up by one line            |
| `↓`      | Move cursor down by one line          |
| `Alt+→`  | Move cursor forward by word           |
| `Alt+←`  | Move cursor backward by word          |
| `Ctrl+D` | Scroll down by half a page            |
| `Ctrl+U` | Scroll up by half a page              |

### Edit Mode

Edit mode allows you to make changes to your note.

#### Key Mappings

> [!WARNING]
>
> Edit mode key mappings cannot be modified.

Edit mode uses a crate called [tui-textarea](https://github.com/rhysd/tui-textarea) and provides the following default key mappings:

|Mapping|Description|
|---|---|
|`Ctrl+H`, `Backspace`|Delete one character before cursor|
|`Ctrl+D`, `Delete`|Delete one character next to cursor|
|`Ctrl+M`, `Enter`|Insert newline|
|`Ctrl+K`|Delete from cursor to the end of line|
|`Ctrl+J`|Delete from cursor to the beginning of line|
|`Ctrl+W`, `Alt+H`, `Alt+Backspace`|Delete one word before cursor|
|`Alt+D`, `Alt+Delete`|Delete one word next to cursor|
|`Ctrl+U`|Undo|
|`Ctrl+R`|Redo|
|`Ctrl+C`, `Copy`|Copy selected text|
|`Ctrl+X`, `Cut`|Cut selected text|
|`Ctrl+Y`, `Paste`|Paste yanked text|
|`Ctrl+F`, `→`|Move cursor forward by one character|
|`Ctrl+B`, `←`|Move cursor backward by one character|
|`Ctrl+P`, `↑`|Move cursor up by one line|
|`Ctrl+N`, `↓`|Move cursor down by one line|
|`Alt+F`, `Ctrl+→`|Move cursor forward by word|
|`Alt+B`, `Ctrl+←`|Move cursor backward by word|
|`Alt+]`, `Alt+P`, `Ctrl+↑`|Move cursor up by paragraph|
|`Alt+[`, `Alt+N`, `Ctrl+↓`|Move cursor down by paragraph|
|`Ctrl+E`, `End`, `Ctrl+Alt+F`, `Ctrl+Alt+→`|Move cursor to the end of line|
|`Ctrl+A`, `Home`, `Ctrl+Alt+B`, `Ctrl+Alt+←`|Move cursor to the beginning of line|
|`Alt+<`, `Ctrl+Alt+P`, `Ctrl+Alt+↑`|Move cursor to top of document|
|`Alt+>`, `Ctrl+Alt+N`, `Ctrl+Alt+↓`|Move cursor to bottom of document|
|`Ctrl+V`, `PageDown`|Scroll down by page|
|`Alt+V`, `PageUp`|Scroll up by page|
