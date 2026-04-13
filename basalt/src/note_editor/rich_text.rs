use std::{fmt, vec::IntoIter};

use pulldown_cmark::CowStr;

/// A style that can be applied to [`TextSegment`] (code, emphasis, strikethrough, strong).
#[derive(Clone, Debug, PartialEq)]
pub enum Style {
    Code,
    Emphasis,
    Strong,
    Strikethrough,
    /// Inline math style (e.g. `$formula$`).
    InlineMath,
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Style::Code => write!(f, "Code"),
            Style::Emphasis => write!(f, "Emphasis"),
            Style::Strong => write!(f, "Strong"),
            Style::Strikethrough => write!(f, "Strikethrough"),
            Style::InlineMath => write!(f, "InlineMath"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSegment {
    pub content: String,
    pub style: Option<Style>,
}

impl TextSegment {
    pub fn new(content: &str, style: Option<Style>) -> Self {
        Self {
            content: content.to_string(),
            style,
        }
    }

    pub fn add_style(&mut self, style: &Style) {
        self.style = Some(style.clone());
    }

    pub fn empty_line() -> Self {
        Self {
            content: '\n'.to_string(),
            style: None,
        }
    }

    pub fn plain(content: &str) -> Self {
        Self {
            content: content.to_string(),
            style: None,
        }
    }

    pub fn styled(content: &str, style: Style) -> Self {
        Self {
            content: content.to_string(),
            style: Some(style),
        }
    }
}

impl fmt::Display for TextSegment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let content = &self.content;
        write!(f, "{content}")
    }
}

impl From<CowStr<'_>> for TextSegment {
    fn from(content: CowStr<'_>) -> Self {
        content.to_string().into()
    }
}

impl From<String> for TextSegment {
    fn from(content: String) -> Self {
        content.as_str().into()
    }
}

impl From<&str> for TextSegment {
    fn from(content: &str) -> Self {
        // Replace tab character with spaces.
        // Tab character can break the rendered TUI.
        // FIXME: Control tab length with config value.
        Self::plain(&content.replace("\t", "  "))
    }
}

/// Target of a hyperlink or navigation action.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkTarget {
    /// An external URL (e.g. https://example.com). Phase 7: OSC 8 hyperlink.
    External(String),
    /// A footnote reference label (e.g. "1"). Future: scroll to definition.
    FootnoteRef(String),
}

/// A typed inline node within a RichText sequence.
#[derive(Clone, Debug, PartialEq)]
pub enum InlineNode {
    /// Existing styled text segment.
    Text(TextSegment),
    /// A hyperlink with display text and a target.
    Link { text: String, target: LinkTarget },
    /// An inline footnote reference marker (example: 1 for footnote reference syntax).
    FootnoteRef(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RichText {
    nodes: Vec<InlineNode>,
}

impl IntoIterator for RichText {
    type Item = InlineNode;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl fmt::Display for RichText {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.nodes
                .iter()
                .map(|node| match node {
                    InlineNode::Text(seg) => seg.to_string(),
                    InlineNode::Link { text, .. } => text.clone(),
                    InlineNode::FootnoteRef(label) => format!("[{label}]"),
                })
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

impl RichText {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn nodes(&self) -> &[InlineNode] {
        &self.nodes
    }
}

impl From<Vec<TextSegment>> for RichText {
    fn from(segments: Vec<TextSegment>) -> Self {
        Self {
            nodes: segments.into_iter().map(InlineNode::Text).collect(),
        }
    }
}

impl<const N: usize> From<[TextSegment; N]> for RichText {
    fn from(segments: [TextSegment; N]) -> Self {
        Self {
            nodes: segments.into_iter().map(InlineNode::Text).collect(),
        }
    }
}

impl From<Vec<InlineNode>> for RichText {
    fn from(nodes: Vec<InlineNode>) -> Self {
        Self { nodes }
    }
}

/// An entry in the link map, recording the screen position of a rendered link.
/// Used by the draw loop to emit OSC 8 hyperlink sequences at the correct position.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkMapEntry {
    /// Virtual line index in the document (0-based, relative to content lines, not meta).
    pub line_idx: usize,
    /// Display column start (inclusive) within the line.
    pub col_start: usize,
    /// Display column end (exclusive) within the line.
    pub col_end: usize,
    /// The link text (needed for OSC 8 re-emission).
    pub text: String,
    /// The link target.
    pub target: LinkTarget,
}
