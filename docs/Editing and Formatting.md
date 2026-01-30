[[Basalt]] renders markdown by replacing syntax characters with visual styling. Heading `#` symbols are hidden and replaced with colored indicators, and block quotes display with a vertical bar instead of `>` characters.

[[Basalt]] supports CommonMark and GitHub Flavored Markdown (GFM) elements through the `pulldown_cmark` parser.

## Headings

All heading levels (H1-H6) are supported with visual indicators:

```markdown
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6
```

## Text content

- **Paragraphs**: Regular text content
- **Block quotes**: Rendered with a vertical bar indicator
- **Inline code**: Rendered with code styling

## Lists

Both ordered and unordered lists are supported:

```markdown
- Unordered item
- Another item

1. First item
2. Second item
```

## Task lists

Checkbox items are rendered with visual indicators:

```markdown
- [ ] Unchecked task
- [x] Completed task
```

## Code blocks

Fenced code blocks are rendered with a distinct background:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

> [!NOTE]
>
> Syntax highlighting is not yet implemented. Code blocks display with uniform styling regardless of language.

## Callouts

Obsidian-style callout blocks are recognized:

```markdown
> [!NOTE]
> This is a note callout.

> [!TIP]
> This is a tip callout.

> [!WARNING]
> This is a warning callout.

> [!CAUTION]
> This is a caution callout.

> [!IMPORTANT]
> This is an important callout.
```

## Links

Wiki-links and standard markdown links are parsed:

```markdown
[[Another Note]]
[[Note|Display Text]]
[[Note#Heading]]
[External Link](https://example.com)
```

## Text styling

While [[Basalt]] parses bold, italic, and strikethrough syntax, these styles are **not yet rendered visually** in the terminal:

```markdown
**bold text**      (parsed but not styled)
*italic text*      (parsed but not styled)
~~strikethrough~~  (parsed but not styled)
```

## Editing notes

By default, [[Basalt]] opens notes in read-only mode. To edit notes, you can either configure an external editor or enable the built-in [[Editor (experimental)|experimental editor]].

### External editor

Configure an external editor command in your [[Configuration]]:

```toml
[global]
key_bindings = [
  { key = "ctrl+alt+e", command = "exec:vi %note_path" },
]
```

### Experimental editor

Enable the built-in experimental editor in your configuration:

```toml
experimental_editor = true
```

See [[Editor (experimental)]] for details on the built-in editing capabilities and its [[Known Limitations|limitations]].
