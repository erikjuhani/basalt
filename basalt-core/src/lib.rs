//! This crate provides the core functionality for Basalt, a TUI application for Obsidian.
//! It lets you read and manipulate Obsidian's configuration, vaults, and notes.
//!
//! # Example
//!
//! ```
//! use basalt_core::obsidian;
//!
//! let config = obsidian::config::load();
//! ```
//!
//! This crate also provides a markdown parser that produces a custom AST using the
//! [`pulldown_cmark::Parser`]. The "AST" acts as an intermediate layer. This enables segregation
//! of the parsing logic into a module under basalt-core lib.
//!
//! # Example
//!
//! ```
//! use basalt_core::markdown::{from_str, Range, Node, MarkdownNode, HeadingLevel, Text};
//!
//! let markdown = "# My Heading\n\nSome text.";
//! let nodes = from_str(markdown);
//!
//! assert_eq!(nodes, vec![
//!   Node {
//!     markdown_node: MarkdownNode::Heading {
//!       level: HeadingLevel::H1,
//!       text: Text::from("My Heading"),
//!     },
//!     source_range: Range { start: 0, end: 13 },
//!   },
//!   Node {
//!     markdown_node: MarkdownNode::Paragraph {
//!       text: Text::from("Some text."),
//!     },
//!     source_range: Range { start: 14, end: 24 },
//!   },
//! ])
//! ```

#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/basalt-core")]

/// Provides Markdown parser that supports Obsidian flavor.
/// Obsidian flavor is a combination of different flavors and a few differences.
///
/// Namely `CommonMark` and `GitHub Flavored Markdown`. More info
/// [here](https://help.obsidian.md/Editing+and+formatting/Obsidian+Flavored+Markdown).
///
/// NOTE: Current iteration does not handle Obsidian flavor, unless it is covered by
/// pulldown-cmark. Part of Obsidian flavor is for example use of any character inside tasks to
/// mark them as completed `- [?] Completed`.
///
/// This crate uses [`pulldown_cmark`] to parse the markdown and enable the applicable features. This
/// crate uses own intermediate types to provide the parsed markdown nodes.
pub mod markdown;

/// Provides Obsidian interoperability operations
pub mod obsidian;
