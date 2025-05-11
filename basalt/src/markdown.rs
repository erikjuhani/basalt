mod state;
mod view;

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
/// pub mod markdown;
mod parser;

pub use state::MarkdownViewState;
pub use view::MarkdownView;
