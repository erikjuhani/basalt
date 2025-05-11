//! A Markdown parser that transforms Markdown input into a custom abstract syntax tree (AST)
//! intented to be rendered with [basalt](https://github.com/erikjuhani/basalt)—a TUI application
//! for Obsidian.
//!
//! This module provides a [`Parser`] type, which processes raw Markdown input into a [`Vec`] of
//! [`Node`]s. These [`Node`]s represent semantic elements such as headings, paragraphs, block
//! quotes, and code blocks.
//!
//! The parser is built on top of [`pulldown_cmark`].
//!
//! ## Simple usage
//!
//! At the simplest level, you can parse a Markdown string by calling the [`from_str`] function:
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
//!     source_range: Range { start: 14, end: 24 }
//!   },
//! ])
//! ```
//!
//! ## Implementation details
//!
//! The [`Parser`] processes [`pulldown_cmark::Event`]s one by one, building up the current
//! [`Node`] in `current_node`. When an event indicates the start of a new structure (e.g.,
//! `Event::Start(Tag::Heading {..})`), the [`Parser`] pushes or replaces the current node
//! with a new one. When an event indicates the end of that structure, the node is finalized
//! and pushed into [`Parser::output`].
//!
//! Unrecognized events (such as [`InlineHtml`](pulldown_cmark::Event::InlineHtml)) are simply
//! ignored for the time being.
//!
//! ## Not yet implemented
//!
//! - Handling of inline HTML, math blocks, etc.
//! - Tracking code block language (`lang`) properly (currently set to [`None`]).
use std::vec::IntoIter;

use pulldown_cmark::{Event, Options, Tag, TagEnd};

/// A style that can be applied to [`TextNode`] (code, emphasis, strikethrough, strong).
#[derive(Clone, Debug, PartialEq)]
pub enum Style {
    /// Inline code style (e.g. `code`).
    Code,
    /// Italic/emphasis style (e.g. `*emphasis*`).
    Emphasis,
    /// Strikethrough style (e.g. `~~strikethrough~~`).
    Strikethrough,
    /// Bold/strong style (e.g. `**strong**`).
    Strong,
}

/// Represents the variant of a list or task item (checked, unchecked, etc.).
#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    /// A checkbox item that is marked as done using `- [x]`.
    HardChecked,
    /// A checkbox item that is checked, but not explicitly recognized as
    /// `HardChecked` (e.g., `- [?]`).
    Checked,
    /// A checkbox item that is unchecked using `- [ ]`.
    Unchecked,
    // TODO: Remove in favor of using List node that has children of nodes
    /// An ordered list item (e.g., `1. item`), storing the numeric index.
    Ordered(u64),
    /// An unordered list item (e.g., `- item`).
    Unordered,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum HeadingLevel {
    H1 = 1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl From<pulldown_cmark::HeadingLevel> for HeadingLevel {
    fn from(value: pulldown_cmark::HeadingLevel) -> Self {
        match value {
            pulldown_cmark::HeadingLevel::H1 => HeadingLevel::H1,
            pulldown_cmark::HeadingLevel::H2 => HeadingLevel::H2,
            pulldown_cmark::HeadingLevel::H3 => HeadingLevel::H3,
            pulldown_cmark::HeadingLevel::H4 => HeadingLevel::H4,
            pulldown_cmark::HeadingLevel::H5 => HeadingLevel::H5,
            pulldown_cmark::HeadingLevel::H6 => HeadingLevel::H6,
        }
    }
}

/// Represents specialized block quote kind variants (tip, note, warning, etc.).
///
/// Currently, the underlying [`pulldown_cmark`] parser distinguishes these via syntax like `">
/// [!NOTE] Some note"`.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum BlockQuoteKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl From<pulldown_cmark::BlockQuoteKind> for BlockQuoteKind {
    fn from(value: pulldown_cmark::BlockQuoteKind) -> Self {
        match value {
            pulldown_cmark::BlockQuoteKind::Tip => BlockQuoteKind::Tip,
            pulldown_cmark::BlockQuoteKind::Note => BlockQuoteKind::Note,
            pulldown_cmark::BlockQuoteKind::Warning => BlockQuoteKind::Warning,
            pulldown_cmark::BlockQuoteKind::Caution => BlockQuoteKind::Caution,
            pulldown_cmark::BlockQuoteKind::Important => BlockQuoteKind::Important,
        }
    }
}

/// Denotes whether a list is ordered or unordered.
#[derive(Clone, Debug, PartialEq)]
pub enum ListKind {
    /// An ordered list item (e.g., `1. item`), storing the numeric index.
    Ordered(u64),
    /// An unordered list item (e.g., `- item`).
    Unordered,
}

/// A single unit of text that is optionally styled (e.g., code).
///
/// [`TextNode`] can be any combination of sentence, words or characters.
///
/// Usually styled text will be contained in a single [`TextNode`] with the given [`Style`]
/// property.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TextNode {
    /// The literal text content.
    pub content: String,
    /// Optional inline style of the text.
    pub style: Option<Style>,
}

impl From<&str> for TextNode {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<String> for TextNode {
    fn from(value: String) -> Self {
        Self {
            content: value,
            ..Default::default()
        }
    }
}

impl TextNode {
    /// Creates a new [`TextNode`] from `content` and optional [`Style`].
    pub fn new(content: String, style: Option<Style>) -> Self {
        Self { content, style }
    }
}

/// A wrapper type holding a list of [`TextNode`]s.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Text(Vec<TextNode>);

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        TextNode::from(value).into()
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        TextNode::from(value).into()
    }
}

impl From<TextNode> for Text {
    fn from(value: TextNode) -> Self {
        Self([value].to_vec())
    }
}

impl From<Vec<TextNode>> for Text {
    fn from(value: Vec<TextNode>) -> Self {
        Self(value)
    }
}

impl From<&[TextNode]> for Text {
    fn from(value: &[TextNode]) -> Self {
        Self(value.to_vec())
    }
}

impl IntoIterator for Text {
    type Item = TextNode;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Text {
    /// Appends a [`TextNode`] to the inner text list.
    fn push(&mut self, node: TextNode) {
        self.0.push(node);
    }
}

/// A [`std::ops::Range`] type for depicting range in [`crate::markdown`].
///
/// # Examples
///
/// ```
/// use basalt_core::markdown::{Node, MarkdownNode, Range, Text};
///
/// let node = Node {
///   markdown_node: MarkdownNode::Paragraph {
///     text: Text::default(),
///   },
///   source_range: Range::default(),
/// };
/// ```
pub type Range<Idx> = std::ops::Range<Idx>;

/// A node in the Markdown AST.
///
/// Each `Node` contains a [`MarkdownNode`] variant representing a specific kind of Markdown
/// element (paragraph, heading, code block, etc.), along with a `source_range` indicating where in
/// the source text this node occurs.
///
/// # Examples
///
/// ```
/// use basalt_core::markdown::{Node, MarkdownNode, Range, Text};
///
/// let node = Node::new(
///   MarkdownNode::Paragraph {
///     text: Text::default(),
///   },
///   0..10,
/// );
///
/// assert_eq!(node.markdown_node, MarkdownNode::Paragraph { text: Text::default() });
/// assert_eq!(node.source_range, Range { start: 0, end: 10 });
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The specific Markdown node represented by this node.
    pub markdown_node: MarkdownNode,

    /// The range in the original source text that this node covers.
    pub source_range: Range<usize>,
}

impl Node {
    /// Creates a new `Node` from the provided [`MarkdownNode`] and source range.
    pub fn new(markdown_node: MarkdownNode, source_range: Range<usize>) -> Self {
        Self {
            markdown_node,
            source_range,
        }
    }

    /// Pushes a [`TextNode`] into the markdown node, if it contains a text buffer.
    ///
    /// If the markdown node is a [`MarkdownNode::BlockQuote`], the [`TextNode`] will be pushed
    /// into the last child [`Node`], if any.
    /// ```
    pub(crate) fn push_text_node(&mut self, node: TextNode) {
        match &mut self.markdown_node {
            MarkdownNode::Paragraph { text, .. }
            | MarkdownNode::Heading { text, .. }
            | MarkdownNode::CodeBlock { text, .. }
            | MarkdownNode::Item { text, .. } => text.push(node),
            MarkdownNode::BlockQuote { nodes, .. } => {
                if let Some(last_node) = nodes.last_mut() {
                    last_node.push_text_node(node);
                }
            }
        }
    }
}

/// The Markdown AST node enumeration.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum MarkdownNode {
    /// A heading node that represents different heading levels.
    ///
    /// The level is controlled with the [`HeadingLevel`] definition.
    Heading {
        level: HeadingLevel,
        text: Text,
    },
    Paragraph {
        text: Text,
    },
    /// A block quote node that represents different quote block variants including callout blocks.
    ///
    /// The variant is controlled with the [`BlockQuoteKind`] definition. When [`BlockQuoteKind`]
    /// is [`None`] the block quote should be interpreted as a regular block quote:
    /// `"> Block quote"`.
    BlockQuote {
        kind: Option<BlockQuoteKind>,
        nodes: Vec<Node>,
    },
    /// A fenced code block, optionally with a language identifier.
    CodeBlock {
        lang: Option<String>,
        text: Text,
    },
    /// A list item node that represents different list item variants including task items.
    ///
    /// The variant is controlled with the [`ItemKind`] definition. When [`ItemKind`] is [`None`]
    /// the item should be interpreted as unordered list item: `"- Item"`.
    Item {
        kind: Option<ItemKind>,
        text: Text,
    },
}

/// Returns `true` if the [`MarkdownNode`] should be closed upon encountering the given [`TagEnd`].
fn matches_tag_end(node: &Node, tag_end: &TagEnd) -> bool {
    matches!(
        (&node.markdown_node, tag_end),
        (MarkdownNode::Paragraph { .. }, TagEnd::Paragraph)
            | (MarkdownNode::Heading { .. }, TagEnd::Heading(..))
            | (MarkdownNode::BlockQuote { .. }, TagEnd::BlockQuote(..))
            | (MarkdownNode::CodeBlock { .. }, TagEnd::CodeBlock)
            | (MarkdownNode::Item { .. }, TagEnd::Item)
    )
}

/// Parses the given Markdown input into a list of [`Node`]s.
///
/// This is a convenience function for constructing a [`Parser`] and calling [`Parser::parse`].  
///
/// # Examples
///
/// ```
/// use basalt_core::markdown::{from_str, Range, Node, MarkdownNode, HeadingLevel, Text};
///
/// let markdown = "# My Heading\n\nSome text.";
/// let nodes = from_str(markdown);
///
/// assert_eq!(nodes, vec![
///   Node {
///     markdown_node: MarkdownNode::Heading {
///       level: HeadingLevel::H1,
///       text: Text::from("My Heading"),
///     },
///     source_range: Range { start: 0, end: 13 },
///   },
///   Node {
///     markdown_node: MarkdownNode::Paragraph {
///       text: Text::from("Some text."),
///     },
///     source_range: Range { start: 14, end: 24 },
///   },
/// ])
/// ```
pub fn from_str(text: &str) -> Vec<Node> {
    Parser::new(text).parse()
}

/// A parser that consumes [`pulldown_cmark::Event`]s and produces a [`Vec`] of [`Node`].
///
/// # Examples
///
/// ```
/// use basalt_core::markdown::{Parser, Range, Node, MarkdownNode, HeadingLevel, Text};
///
/// let markdown = "# My Heading\n\nSome text.";
/// let parser = Parser::new(markdown);
/// let nodes = parser.parse();
///
/// assert_eq!(nodes, vec![
///   Node {
///     markdown_node: MarkdownNode::Heading {
///       level: HeadingLevel::H1,
///       text: Text::from("My Heading"),
///     },
///     source_range: Range { start: 0, end: 13 },
///   },
///   Node {
///     markdown_node: MarkdownNode::Paragraph {
///       text: Text::from("Some text."),
///     },
///     source_range: Range { start: 14, end: 24 },
///   },
/// ])
/// ```
pub struct Parser<'a> {
    /// Contains the completed AST [`Node`]s.
    pub output: Vec<Node>,
    inner: pulldown_cmark::TextMergeWithOffset<'a, pulldown_cmark::OffsetIter<'a>>,
    current_node: Option<Node>,
}

impl<'a> Iterator for Parser<'a> {
    type Item = (Event<'a>, Range<usize>);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> Parser<'a> {
    /// Creates a new [`Parser`] from a Markdown input string.
    ///
    /// The parser uses [`pulldown_cmark::Parser::new_ext`] with [`Options::all()`] and
    /// [`pulldown_cmark::TextMergeWithOffset`] internally.
    ///
    /// The offset is required to know where the node appears in the provided source text.
    pub fn new(text: &'a str) -> Self {
        let parser = pulldown_cmark::TextMergeWithOffset::new(
            pulldown_cmark::Parser::new_ext(text, Options::all()).into_offset_iter(),
        );

        Self {
            inner: parser,
            output: vec![],
            current_node: None,
        }
    }

    /// Pushes a [`Node`] as a child if the current node is a [`BlockQuote`], otherwise sets it as
    /// the `current_node`.
    fn push_node(&mut self, node: Node) {
        if let Some(Node {
            markdown_node: MarkdownNode::BlockQuote { nodes, .. },
            ..
        }) = &mut self.current_node
        {
            nodes.push(node);
        } else {
            self.set_node(&node);
        }
    }

    /// Pushes a [`TextNode`] into the `current_node` if it exists.
    fn push_text_node(&mut self, node: TextNode) {
        if let Some(ref mut current) = self.current_node {
            current.push_text_node(node);
        }
    }

    /// Sets (or replaces) the `current_node` with a new one, discarding any old node.
    fn set_node(&mut self, block: &Node) {
        self.current_node.replace(block.clone());
    }

    /// Handles the start of a [`Tag`]. Pushes the matching semantic node to be processed.
    fn tag(&mut self, tag: Tag<'a>, range: Range<usize>) {
        match tag {
            Tag::Paragraph => self.push_node(Node::new(
                MarkdownNode::Paragraph {
                    text: Text::default(),
                },
                range,
            )),
            Tag::Heading { level, .. } => self.push_node(Node::new(
                MarkdownNode::Heading {
                    level: level.into(),
                    text: Text::default(),
                },
                range,
            )),
            Tag::BlockQuote(kind) => self.push_node(Node::new(
                MarkdownNode::BlockQuote {
                    kind: kind.map(|kind| kind.into()),
                    nodes: vec![],
                },
                range,
            )),
            Tag::CodeBlock(_) => self.push_node(Node::new(
                MarkdownNode::CodeBlock {
                    lang: None,
                    text: Text::default(),
                },
                range,
            )),
            Tag::Item => self.push_node(Node::new(
                MarkdownNode::Item {
                    kind: None,
                    text: Text::default(),
                },
                range,
            )),
            // For now everything below this comment are defined as paragraph nodes
            Tag::HtmlBlock
            | Tag::List(_)
            | Tag::FootnoteDefinition(_)
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Link { .. }
            | Tag::Image { .. }
            | Tag::MetadataBlock(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => {}
        }
    }

    /// Handles the end of a [`Tag`], finalizing a node if matching.
    fn tag_end(&mut self, tag_end: TagEnd) {
        let Some(node) = self.current_node.take() else {
            return;
        };

        if matches_tag_end(&node, &tag_end) {
            self.output.push(node);
        } else {
            self.set_node(&node);
        }
    }

    /// Processes a single [`Event`] from the underlying [`pulldown_cmark::Parser`] iterator.
    fn handle_event(&mut self, event: Event<'a>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.tag(tag, range),
            Event::End(tag_end) => self.tag_end(tag_end),
            Event::Text(text) => self.push_text_node(TextNode::new(text.to_string(), None)),
            Event::Code(text) => {
                self.push_text_node(TextNode::new(text.to_string(), Some(Style::Code)))
            }
            Event::TaskListMarker(checked) => {
                // The range for these markdown items only applies to the `[ ]` portion.
                // TODO: Add implementation for ListBlock, which will retain the complete source
                // range.
                if checked {
                    self.set_node(&Node::new(
                        MarkdownNode::Item {
                            kind: Some(ItemKind::HardChecked),
                            text: Text::default(),
                        },
                        range,
                    ));
                } else {
                    self.set_node(&Node::new(
                        MarkdownNode::Item {
                            kind: Some(ItemKind::Unchecked),
                            text: Text::default(),
                        },
                        range,
                    ));
                }
            }
            Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Rule
            | Event::FootnoteReference(_) => {
                // TODO: Not yet implemented
            }
        }
    }

    /// Consumes the parser, processing all remaining events from the stream into a list of
    /// [`Node`]s.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basalt_core::markdown::{Parser, Node, MarkdownNode, Range, Text};
    /// let parser = Parser::new("Hello world");
    ///
    /// let nodes = parser.parse();
    ///
    /// assert_eq!(nodes, vec![
    ///   Node {
    ///     markdown_node: MarkdownNode::Paragraph {
    ///       text: Text::from("Hello world"),
    ///     },
    ///     source_range: Range { start: 0, end: 11 },
    ///   },
    /// ]);
    /// ```
    pub fn parse(mut self) -> Vec<Node> {
        while let Some((event, range)) = self.next() {
            self.handle_event(event, range);
        }

        if let Some(node) = self.current_node.take() {
            self.output.push(node);
        }

        self.output
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    fn p(str: &str, range: Range<usize>) -> Node {
        Node::new(MarkdownNode::Paragraph { text: str.into() }, range)
    }

    fn blockquote(nodes: Vec<Node>, range: Range<usize>) -> Node {
        Node::new(MarkdownNode::BlockQuote { kind: None, nodes }, range)
    }

    fn item(str: &str, range: Range<usize>) -> Node {
        Node::new(
            MarkdownNode::Item {
                kind: None,
                text: str.into(),
            },
            range,
        )
    }

    fn task(str: &str, range: Range<usize>) -> Node {
        Node::new(
            MarkdownNode::Item {
                kind: Some(ItemKind::Unchecked),
                text: str.into(),
            },
            range,
        )
    }

    fn completed_task(str: &str, range: Range<usize>) -> Node {
        Node::new(
            MarkdownNode::Item {
                kind: Some(ItemKind::HardChecked),
                text: str.into(),
            },
            range,
        )
    }

    fn heading(level: HeadingLevel, str: &str, range: Range<usize>) -> Node {
        Node::new(
            MarkdownNode::Heading {
                level,
                text: str.into(),
            },
            range,
        )
    }

    fn h1(str: &str, range: Range<usize>) -> Node {
        heading(HeadingLevel::H1, str, range)
    }

    fn h2(str: &str, range: Range<usize>) -> Node {
        heading(HeadingLevel::H2, str, range)
    }

    fn h3(str: &str, range: Range<usize>) -> Node {
        heading(HeadingLevel::H3, str, range)
    }

    fn h4(str: &str, range: Range<usize>) -> Node {
        heading(HeadingLevel::H4, str, range)
    }

    fn h5(str: &str, range: Range<usize>) -> Node {
        heading(HeadingLevel::H5, str, range)
    }

    fn h6(str: &str, range: Range<usize>) -> Node {
        heading(HeadingLevel::H6, str, range)
    }

    use super::*;

    #[test]
    fn test_parse() {
        let tests = [
            (
                indoc! {r#"# Heading 1

                ## Heading 2

                ### Heading 3

                #### Heading 4

                ##### Heading 5

                ###### Heading 6
                "#},
                vec![
                    h1("Heading 1", 0..12),
                    h2("Heading 2", 13..26),
                    h3("Heading 3", 27..41),
                    h4("Heading 4", 42..57),
                    h5("Heading 5", 58..74),
                    h6("Heading 6", 75..92),
                ],
            ),
            // TODO: Implement correct test case when `- [?] ` task item syntax is supported
            // Now we interpret it as a regular paragraph
            (
                indoc! { r#"## Tasks

                - [ ] Task

                - [x] Completed task

                - [?] Completed task
                "#},
                vec![
                    h2("Tasks", 0..9),
                    task("Task", 12..15),
                    completed_task("Completed task", 24..27),
                    p("[?] Completed task", 46..65),
                ],
            ),
            (
                indoc! {r#"## Quotes

                You _can_ quote text by adding a `>` symbols before the text.

                > Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.
                >
                >- Doug Engelbart, 1961
                "#},
                vec![
                    h2("Quotes", 0..10),
                    Node::new(MarkdownNode::Paragraph {
                        text: vec![
                            TextNode::new("You ".into(), None),
                            TextNode::new("can".into(),None),
                            TextNode::new(" quote text by adding a ".into(), None),
                            TextNode::new(">".into(), Some(Style::Code)),
                            TextNode::new(" symbols before the text.".into(), None),
                        ]
                        .into(),
                    }, 11..73),
                    blockquote(vec![
                        p("Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.", 76..269),
                        item("Doug Engelbart, 1961", 272..295)
                    ], 74..295),
                ],
            ),
        ];

        tests
            .iter()
            .for_each(|test| assert_eq!(from_str(test.0), test.1));
    }
}
