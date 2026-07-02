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

The [[Editor (experimental)|experimental editor]] is disabled by default and requires [[Configuration|configuration]] to enable. When enabled, it provides a custom-built editor with limited capabilities.

- No undo/redo
- No clipboard (copy/cut/paste)
- No text selection
- No multi-line deletion
- No line/word deletion commands
- No jumping to start/end of line
- The editor works on individual blocks (paragraphs, headings, etc.) rather than the full document

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
