This page documents current limitations and features not yet implemented.

## Markdown rendering

- Images are not rendered
- Horizontal rules are not rendered
- Syntax highlighting is not supported
- Inline text styles (bold, italic, strikethrough) are parsed but not styled
- Math blocks (`$...$`, `$$...$$`) are not supported
- Footnotes are not supported
- HTML content is not supported
- External links are not clickable
- Code blocks are rendered without syntax highlighting
- Callout folding (`> [!note]-`) is not interactive; folds render expanded
- Task items `- [ ]` and `- [x]` work; `- [?]` is not supported

## File operations

- Deleting notes or folders is not supported
- Moving notes or folders is not supported
- Copying notes or folders is not supported
- Searching notes is not supported

## Experimental editor

The [[Editor (experimental)|experimental editor]] is disabled by default and requires [[Configuration|configuration]] to enable. It edits the whole note line by line. With [[Configuration|vim mode]] it supports undo/redo, clipboard, visual (line and block) selection, line and word deletion and jumps to the start and end of the line and document.

- Pasting images from the clipboard is not supported

## Configuration

- Multiple config files are not merged (first found is used)
- Shell expansion in commands is not supported
- Piping in commands is not supported

## Obsidian compatibility

[[Basalt]] aims to be compatible with Obsidian vaults but does not support all Obsidian features.

- Obsidian plugins are not supported
- Some Obsidian-specific markdown may not render
- Graph view is not available
- Backlinks panel is not available
- Creating vaults is not available
