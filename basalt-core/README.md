# Basalt Core: basalt-core

This crate provides the core functionality for Basalt TUI application and
interoperability layer to Obsidian vaults and notes.

## Obsidian

Obsidian module provides functionality operating with Obsidian. It lets you
read and manipulate Obsidian's configuration, vaults, and notes.

Currently supports reading vaults, notes, and writing to note path.

### Example

```rust
use basalt_core::obsidian::{ObsidianConfig, Error, Vault};

let config = ObsidianConfig::from([
  ("Obsidian", Vault::default()),
  ("My Vault", Vault::default()),
]);

_ = config.get_vault_by_name("Obsidian");
```

## Markdown


Markdown module provides a markdown parser that produces a custom AST using the
`pulldown_cmark::Parser`. The "AST" acts as an intermediate layer. This enables
segregation of the parsing logic into it's own module under basalt-core lib.

### Example

```rust
use basalt_core::markdown::{from_str, Node, HeadingLevel, Text};

let markdown = "# My Heading\n\nSome text.";
let nodes = from_str(markdown);

assert_eq!(nodes, vec![
  Node::Heading {
    level: HeadingLevel::H1,
    text: Text::from("My Heading"),
  },
  Node::Paragraph {
    text: Text::from("Some text."),
  },
])
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Copyright (c) Erik Kinnunen.
