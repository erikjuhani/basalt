This page documents current limitations and features not yet implemented.

## Markdown Rendering

| Feature | Status |
|---------|--------|
| Images | Not rendered |
| Tables | Not rendered |
| Horizontal rules | Not rendered |
| Syntax highlighting | Not supported |
| Inline text styles (bold, italic, strikethrough) | Parsed but not styled |
| Math blocks (`$...$`, `$$...$$`) | Not supported |
| Footnotes | Not supported |
| HTML content | Not supported |
| External links | Not clickable |
| Code blocks | Rendered without syntax highlighting |
| Callouts | Parsed but types not visually differentiated |
| Task items | `- [ ]` and `- [x]` work; `- [?]` not supported |

## File Operations

| Operation | Status |
|-----------|--------|
| Create notes | Not supported |
| Create folders | Not supported |
| Delete notes or folders | Not supported |
| Move notes or folders | Not supported |
| Copy notes or folders | Not supported |
| Search notes | Not supported |

## Experimental Editor

The experimental editor is disabled by default and requires configuration to enable. When enabled, it provides a custom-built editor with limited capabilities.

| Feature | Status |
|---------|--------|
| Character insertion | Supported |
| Character deletion (backspace) | Supported |
| Cursor movement | Supported |
| Word jumping | Supported |
| Newline insertion | Supported |
| Undo/Redo | Not supported |
| Clipboard (copy/cut/paste) | Not supported |
| Text selection | Not supported |
| Multi-line deletion | Not supported |
| Line/word deletion commands | Not supported |
| Jump to start/end of line or document | Not supported |
| Edit mode key mapping customization | Not supported |

The editor works on individual blocks (paragraphs, headings, etc.) rather than the full document. See [[Editor (experimental)]] for details.

## Configuration

| Feature | Status |
|---------|--------|
| Edit view key mappings | Cannot be customized |
| Multiple config file merge | Not supported (first found is used) |
| Shell expansion in commands | Not supported |
| Piping in commands | Not supported |

## Obsidian Compatibility

[[Basalt]] aims to be compatible with [[Obsidian]] vaults but does not support all [[Obsidian]] features.

| Feature | Status |
|---------|--------|
| Obsidian plugins | Not supported |
| Some Obsidian-specific markdown | May not render |
| Graph view | Not available |
| Backlinks panel | Not available |
| Creating vaults | Not available |

## See Also

- [[Editing and Formatting]] - What is currently supported
- [[Editor (experimental)]] - Editor capabilities and limitations
