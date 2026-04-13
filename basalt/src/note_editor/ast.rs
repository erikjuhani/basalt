use crate::note_editor::rich_text::{InlineNode, LinkTarget, RichText};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum BlockQuoteKind {
    // Standard GitHub Alert (pulldown-cmark native)
    Note,
    Tip,
    Important,
    Warning,
    Caution,
    // ITS Theme Extended (post-processing detection)
    Aside,
    Blank,
    Caption,
    Cards,
    Checks,
    Column,
    Grid,
    Infobox,
    Kanban,
    Kith,
    Metadata,
    Quote,
    Recite,
    Statblocks,
    Timeline,
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
pub enum ItemKind {
    /// An ordered list item (e.g., `1. item`), storing the numeric index.
    Ordered(u64),
    /// An unordered list item (e.g., `- item`).
    Unordered,
}

/// Represents the variant of a list or task item (checked, unchecked, or an ITS Theme marker).
#[derive(Clone, Debug, PartialEq)]
pub enum TaskKind {
    // Standard markers
    /// A checkbox item that is marked as done using `- [x]` or `- [X]`.
    Checked,
    /// A checkbox item that is unchecked using `- [ ]`.
    Unchecked,

    // ITS Theme markers (35), in ITS Theme source order
    /// `- [-]` dropped
    Dropped,
    /// `- [>]` forwarded / scheduled
    Forward,
    /// `- [<]` migrated
    Migrated,
    /// `- [D]` date
    Date,
    /// `- [?]` question
    Question,
    /// `- [/]` half done
    HalfDone,
    /// `- [+]` add
    Add,
    /// `- [R]` research
    Research,
    /// `- [!]` important
    Important,
    /// `- [i]` idea
    Idea,
    /// `- [B]` brainstorm
    Brainstorm,
    /// `- [P]` pro
    Pro,
    /// `- [C]` con
    Con,
    /// `- [Q]` quote
    Quote,
    /// `- [N]` note
    Note,
    /// `- [b]` bookmark
    Bookmark,
    /// `- [I]` information
    Information,
    /// `- [p]` paraphrase
    Paraphrase,
    /// `- [L]` location
    Location,
    /// `- [E]` example
    Example,
    /// `- [A]` answer
    Answer,
    /// `- [r]` reward
    Reward,
    /// `- [c]` choice
    Choice,
    /// `- [d]` doing
    Doing,
    /// `- [T]` time
    Time,
    /// `- [@]` character / person
    Character,
    /// `- [t]` talk
    Talk,
    /// `- [O]` outline / plot
    Outline,
    /// `- [~]` conflict
    Conflict,
    /// `- [W]` world
    World,
    /// `- [f]` clue / find
    Clue,
    /// `- [F]` foreshadow
    Foreshadow,
    /// `- [H]` favorite / health
    Favorite,
    /// `- [&]` symbolism
    Symbolism,
    /// `- [s]` secret
    Secret,

    // Fallback
    /// Any `[char]` not matching the 35 ITS Theme markers above.
    /// Stores the original character so the renderer can display `[char]` as-is.
    LooselyChecked(char),
}

pub type SourceRange<Idx> = std::ops::Range<Idx>;

/// Column alignment extracted from the `---` separator row of a GFM table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alignment {
    /// `:---` — left-aligned
    Left,
    /// `:---:` — center-aligned
    Center,
    /// `---:` — right-aligned
    Right,
    /// `---` — no colons, treated as Left at render time
    None,
}

impl From<pulldown_cmark::Alignment> for Alignment {
    fn from(value: pulldown_cmark::Alignment) -> Self {
        match value {
            pulldown_cmark::Alignment::Left => Alignment::Left,
            pulldown_cmark::Alignment::Center => Alignment::Center,
            pulldown_cmark::Alignment::Right => Alignment::Right,
            pulldown_cmark::Alignment::None => Alignment::None,
        }
    }
}

/// The Markdown AST node enumeration.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Heading {
        level: HeadingLevel,
        text: RichText,
        source_range: SourceRange<usize>,
    },
    Paragraph {
        text: RichText,
        source_range: SourceRange<usize>,
    },
    CodeBlock {
        lang: Option<String>,
        text: RichText,
        source_range: SourceRange<usize>,
    },
    BlockQuote {
        kind: Option<BlockQuoteKind>,
        title: Option<String>,
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    List {
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    Item {
        kind: ItemKind,
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    Task {
        kind: TaskKind,
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    Table {
        alignments: Vec<Alignment>,
        header: Vec<RichText>,
        rows: Vec<Vec<RichText>>,
        source_range: SourceRange<usize>,
    },
    /// Collected footnote definitions at the end of the document.
    FootnoteSection {
        defs: indexmap::IndexMap<String, RichText>,
        source_range: SourceRange<usize>,
    },
}

impl Node {
    pub fn source_range(&self) -> &SourceRange<usize> {
        match self {
            Self::Heading { source_range, .. }
            | Self::CodeBlock { source_range, .. }
            | Self::Paragraph { source_range, .. }
            | Self::List { source_range, .. }
            | Self::BlockQuote { source_range, .. }
            | Self::Item { source_range, .. }
            | Self::Task { source_range, .. }
            | Self::Table { source_range, .. }
            | Self::FootnoteSection { source_range, .. } => source_range,
        }
    }

    pub fn set_source_range(&mut self, new_range: SourceRange<usize>) {
        match self {
            Self::Heading { source_range, .. }
            | Self::CodeBlock { source_range, .. }
            | Self::Paragraph { source_range, .. }
            | Self::List { source_range, .. }
            | Self::BlockQuote { source_range, .. }
            | Self::Item { source_range, .. }
            | Self::Task { source_range, .. }
            | Self::Table { source_range, .. }
            | Self::FootnoteSection { source_range, .. } => *source_range = new_range,
        }
    }

    pub fn rich_text(&self) -> Option<&RichText> {
        match self {
            Self::Heading { text, .. }
            | Self::Paragraph { text, .. }
            | Self::CodeBlock { text, .. } => Some(text),
            _ => None,
        }
    }
}

pub fn nodes_to_sexp(nodes: &[Node], indent_level: usize) -> String {
    nodes
        .iter()
        .map(|node| node_to_sexp(node, indent_level))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn node_to_sexp(node: &Node, indent_level: usize) -> String {
    let indent_increment = 2;

    match node {
        Node::Heading {
            level,
            text,
            source_range,
        } => {
            format!(
                "{:indent$}(heading {:?} @{:?}\n{})",
                "",
                level,
                source_range,
                rich_text_to_sexp(text, indent_level + indent_increment),
                indent = indent_level
            )
        }
        Node::Paragraph { text, source_range } => {
            format!(
                "{:indent$}(paragraph @{:?}\n{})",
                "",
                source_range,
                rich_text_to_sexp(text, indent_level + indent_increment),
                indent = indent_level
            )
        }
        Node::BlockQuote {
            kind,
            title,
            nodes,
            source_range,
        } => {
            format!(
                "{:indent$}(blockquote {:?} title={:?} @{:?}\n{})",
                "",
                kind,
                title,
                source_range,
                nodes_to_sexp(nodes, indent_level + indent_increment),
                indent = indent_level
            )
        }
        Node::CodeBlock {
            lang,
            text,
            source_range,
        } => {
            format!(
                "{:indent$}(codeblock {} @{:?}\n{})",
                "",
                lang.clone().unwrap_or(String::new()),
                source_range,
                rich_text_to_sexp(text, indent_level + indent_increment),
                indent = indent_level,
            )
        }
        Node::List {
            nodes,
            source_range,
        } => {
            format!(
                "{:indent$}(list @{:?}\n{})",
                "",
                source_range,
                nodes_to_sexp(nodes, indent_level + indent_increment),
                indent = indent_level
            )
        }
        Node::Item {
            kind,
            nodes,
            source_range,
        } => {
            format!(
                "{:indent$}(item {:?} @{:?}\n{})",
                "",
                kind,
                source_range,
                nodes_to_sexp(nodes, indent_level + indent_increment),
                indent = indent_level
            )
        }
        Node::Task {
            kind,
            nodes,
            source_range,
        } => {
            format!(
                "{:indent$}(task {:?} @{:?}\n{})",
                "",
                kind,
                source_range,
                nodes_to_sexp(nodes, indent_level + indent_increment),
                indent = indent_level
            )
        }
        Node::Table {
            alignments,
            header,
            rows,
            source_range,
        } => {
            format!(
                "{:indent$}(table alignments={:?} cols={} rows={} @{:?})",
                "",
                alignments,
                header.len(),
                rows.len(),
                source_range,
                indent = indent_level
            )
        }
        Node::FootnoteSection { defs, source_range } => {
            let entries: Vec<String> = defs
                .iter()
                .map(|(label, rt)| {
                    format!(
                        "{:indent$}(footnote \"{}\" {})",
                        "",
                        label,
                        rich_text_to_sexp(rt, indent_level + indent_increment + 2),
                        indent = indent_level + indent_increment
                    )
                })
                .collect();
            format!(
                "{:indent$}(footnote-section @{:?}\n{})",
                "",
                source_range,
                entries.join("\n"),
                indent = indent_level
            )
        }
    }
}

pub fn rich_text_to_sexp(rich_text: &RichText, indent_level: usize) -> String {
    rich_text
        .nodes()
        .iter()
        .map(|node| match node {
            InlineNode::Text(segment) => match &segment.style {
                Some(style) => format!(
                    "{:indent$}({} \"{}\")",
                    "",
                    style,
                    segment,
                    indent = indent_level
                ),
                None => format!("{:indent$}\"{}\"", "", segment, indent = indent_level),
            },
            InlineNode::Link { text, target } => {
                let target_str = match target {
                    LinkTarget::External(url) => format!("url={}", url),
                    LinkTarget::FootnoteRef(label) => format!("fnref={}", label),
                };
                format!(
                    "{:indent$}(link \"{}\" {})",
                    "",
                    text,
                    target_str,
                    indent = indent_level
                )
            }
            InlineNode::FootnoteRef(label) => {
                format!(
                    "{:indent$}(footnote-ref \"{}\")",
                    "",
                    label,
                    indent = indent_level
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
