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
pub enum ItemKind {
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
            nodes,
            source_range,
        } => {
            format!(
                "{:indent$}(blockquote {:?} @{:?}\n{})",
                "",
                kind,
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
    }
}

pub fn rich_text_to_sexp(rich_text: &RichText, indent_level: usize) -> String {
    rich_text
        .segments()
        .iter()
        .map(|segment| match &segment.style {
            Some(style) => format!(
                "{:indent$}({} \"{}\")",
                "",
                style,
                segment,
                indent = indent_level
            ),
            None => format!("{:indent$}\"{}\"", "", segment, indent = indent_level),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
