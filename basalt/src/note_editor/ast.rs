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

/// The Obsidian callout types. GitHub's `important`/`caution` are aliases of
/// `tip`/`warning`, matching Obsidian.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockQuoteKind {
    Note,
    Abstract,
    Info,
    Todo,
    Tip,
    Success,
    Question,
    Warning,
    Failure,
    Danger,
    Bug,
    Example,
    Quote,
}

impl From<pulldown_cmark::BlockQuoteKind> for BlockQuoteKind {
    fn from(value: pulldown_cmark::BlockQuoteKind) -> Self {
        use pulldown_cmark::BlockQuoteKind as Gfm;
        match value {
            Gfm::Note => BlockQuoteKind::Note,
            Gfm::Tip | Gfm::Important => BlockQuoteKind::Tip,
            Gfm::Warning | Gfm::Caution => BlockQuoteKind::Warning,
        }
    }
}

impl BlockQuoteKind {
    /// Resolves a callout type name (including Obsidian aliases). Unknown types
    /// fall back to [`Note`](BlockQuoteKind::Note), as in Obsidian.
    fn from_name(name: &str) -> Self {
        use BlockQuoteKind::*;
        match name.trim().to_ascii_lowercase().as_str() {
            "abstract" | "summary" | "tldr" => Abstract,
            "info" => Info,
            "todo" => Todo,
            "tip" | "hint" | "important" => Tip,
            "success" | "check" | "done" => Success,
            "question" | "help" | "faq" => Question,
            "warning" | "caution" | "attention" => Warning,
            "failure" | "fail" | "missing" => Failure,
            "danger" | "error" => Danger,
            "bug" => Bug,
            "example" => Example,
            "quote" | "cite" => Quote,
            _ => Note,
        }
    }
}

pub struct CalloutMarker {
    pub kind: BlockQuoteKind,
    pub title: Option<String>,
}

/// Parses a callout marker (`[!note]`, `[!note]- Title`) from a quote's first
/// line. Covers Obsidian's fold markers (`-`/`+`, accepted but not yet acted on)
/// and custom titles, which `pulldown_cmark` does not recognise.
pub fn parse_callout_marker(line: &str) -> Option<CalloutMarker> {
    let rest = line.trim_start().strip_prefix("[!")?;
    let close = rest.find(']')?;
    let kind = BlockQuoteKind::from_name(&rest[..close]);
    let tail = &rest[close + 1..];
    let title = tail.strip_prefix(['-', '+']).unwrap_or(tail).trim();
    Some(CalloutMarker {
        kind,
        title: (!title.is_empty()).then(|| title.to_string()),
    })
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

/// Column alignment for a table, derived from the delimiter row (e.g. `:---:`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alignment {
    /// No explicit alignment; rendered left aligned.
    None,
    /// Left aligned (`:---`).
    Left,
    /// Centered (`:---:`).
    Center,
    /// Right aligned (`---:`).
    Right,
}

impl From<pulldown_cmark::Alignment> for Alignment {
    fn from(value: pulldown_cmark::Alignment) -> Self {
        match value {
            pulldown_cmark::Alignment::None => Alignment::None,
            pulldown_cmark::Alignment::Left => Alignment::Left,
            pulldown_cmark::Alignment::Center => Alignment::Center,
            pulldown_cmark::Alignment::Right => Alignment::Right,
        }
    }
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
        /// Custom callout title (Obsidian `> [!note] Title`); `None` falls back
        /// to the kind's own label.
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
    /// A GFM table with a header row and zero or more body rows. Each cell is the [`RichText`]
    /// between two pipes; `alignments` holds one [`Alignment`] per column.
    Table {
        alignments: Vec<Alignment>,
        head: Vec<RichText>,
        rows: Vec<Vec<RichText>>,
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
            | Self::Table { source_range, .. } => source_range,
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
            | Self::Table { source_range, .. } => *source_range = new_range,
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

    pub fn children_as_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::List { nodes, .. }
            | Self::Item { nodes, .. }
            | Self::Task { nodes, .. }
            | Self::BlockQuote { nodes, .. } => Some(nodes),
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
                "{:indent$}(blockquote {:?} {:?} @{:?}\n{})",
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
            head,
            rows,
            source_range,
        } => {
            let cells = |row: &[RichText]| {
                row.iter()
                    .map(|cell| format!("\"{cell}\""))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let inner_indent = indent_level + indent_increment;
            let body = rows
                .iter()
                .map(|row| format!("{:inner_indent$}(row {})", "", cells(row)))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{:indent$}(table {:?} @{:?}\n{:inner_indent$}(head {})\n{})",
                "",
                alignments,
                source_range,
                "",
                cells(head),
                body,
                indent = indent_level,
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
