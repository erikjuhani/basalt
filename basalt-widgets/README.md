# basalt-widgets

This crate provides the custom ratatui widgets used in `basalt` app.

## markdown

Markdown module provides a Markdown View Widget that renders markdown nodes
provided by [`basalt_core::markdown`] parser.

Example of rendered output:

```
██ Headings

█ This is a heading 1

██ This is a heading 2

▓▓▓ This is a heading 3

▓▓▓▓ This is a heading 4

▓▓▓▓▓ This is a heading 5

░░░░░░ This is a heading 6

██ Quotes

You can quote text by adding a > symbols before the text.

┃ Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.
┃
┃ - Doug Engelbart, 1961

██ Bold, italics, highlights

This line will not be bold

\*\*This line will not be bold\*\*
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Copyright (c) Erik Kinnunen.
