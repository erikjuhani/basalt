use crate::note_editor::rich_text::RichText;

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

/// Represents the variant of a list or task item (checked, unchecked, etc.).
#[derive(Clone, Debug, PartialEq)]
pub enum TaskKind {
    /// A checkbox item that is marked as done using `- [x]`.
    Checked,
    /// A checkbox item that is unchecked using `- [ ]`.
    Unchecked,
    /// A checkbox item that is checked, but not explicitly recognized as
    /// `Checked` (e.g., `- [?]`).
    LooselyChecked,
}

pub type SourceRange<Idx> = std::ops::Range<Idx>;

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
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    List {
        kind: ListKind,
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    Item {
        nodes: Vec<Node>,
        source_range: SourceRange<usize>,
    },
    Task {
        kind: TaskKind,
        nodes: Vec<Node>,
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
            | Self::Task { source_range, .. } => source_range,
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
            | Self::Task { source_range, .. } => *source_range = new_range,
        }
    }
}

pub fn nodes_to_sexp(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(node_to_sexp)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn node_to_sexp(node: &Node) -> String {
    match node {
        Node::Heading {
            level,
            text,
            source_range,
        } => {
            format!(
                "(heading {:?} @{:?}\n  {})",
                level,
                source_range,
                rich_text_to_sexp(text)
            )
        }
        Node::Paragraph { text, source_range } => {
            format!(
                "(paragraph @{:?}\n  {})",
                source_range,
                rich_text_to_sexp(text)
            )
        }
        Node::BlockQuote {
            kind,
            nodes,
            source_range,
        } => {
            format!(
                "(blockquote {:?} @{:?}\n  {})",
                kind,
                source_range,
                nodes_to_sexp(nodes)
            )
        }
        Node::CodeBlock {
            lang,
            text,
            source_range,
        } => {
            format!(
                "(codeblock {} @{:?}\n  {})",
                lang.clone().unwrap_or_default(),
                source_range,
                rich_text_to_sexp(text)
            )
        }
        Node::List {
            kind,
            nodes,
            source_range,
        } => {
            format!(
                "(list {:?} @{:?}\n  {})",
                kind,
                source_range,
                nodes_to_sexp(nodes)
            )
        }
        Node::Item {
            nodes,
            source_range,
        } => {
            format!("(item @{:?}\n  {})", source_range, nodes_to_sexp(nodes))
        }
        Node::Task {
            kind,
            nodes,
            source_range,
        } => {
            format!(
                "(item {:?} @{:?}\n  {})",
                kind,
                source_range,
                nodes_to_sexp(nodes)
            )
        }
    }
}

pub fn rich_text_to_sexp(rich_text: &RichText) -> String {
    rich_text
        .segments()
        .iter()
        .map(|segment| match &segment.style {
            Some(style) => format!("({} \"{}\")", style, segment),
            None => format!("\"{}\"", segment),
        })
        .collect::<Vec<_>>()
        .join("\n  ")
}

// impl Node {
//     pub fn push_text_segment(&mut self, text_segment: TextSegment) {
//         match self {
//             Node::Heading { text, .. } => text.push_text_segment(text_segment),
//             Node::BlockQuote { nodes, .. } => {
//                 if let Some(last_node) = nodes.last_mut() {
//                     last_node.push_text_segment(text_segment);
//                 }
//             }
//             _ => {}
//         }
//     }
// }
