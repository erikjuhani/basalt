use std::{borrow::Cow, fmt, ops::Range, vec::IntoIter};

use pulldown_cmark::CowStr;

/// A style that can be applied to [`TextSegment`] (code, emphasis, strikethrough, strong).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Code,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Style::Code => write!(f, "Code"),
            Style::Emphasis => write!(f, "Emphasis"),
            Style::Strong => write!(f, "Strong"),
            Style::Strikethrough => write!(f, "Strikethrough"),
            Style::Link => write!(f, "Link"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSegment {
    pub content: String,
    /// The inline marks covering this segment, outermost first. Several apply at
    /// once for nested markup, so `**bold _italic_**` yields `[Strong, Emphasis]`.
    pub styles: Vec<Style>,
}

impl TextSegment {
    pub fn new(content: &str, styles: Vec<Style>) -> Self {
        Self {
            content: content.to_string(),
            styles,
        }
    }

    pub fn has_style(&self, style: Style) -> bool {
        self.styles.contains(&style)
    }

    pub fn empty_line() -> Self {
        Self::new("\n", Vec::new())
    }

    pub fn plain(content: &str) -> Self {
        Self::new(content, Vec::new())
    }

    pub fn styled(content: &str, style: Style) -> Self {
        Self::new(content, vec![style])
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RichText {
    segments: Vec<TextSegment>,
}

impl IntoIterator for RichText {
    type Item = TextSegment;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl fmt::Display for RichText {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.segments
            .iter()
            .try_for_each(|segment| write!(f, "{segment}"))
    }
}

impl RichText {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
    }

    /// The text with all styling dropped. Borrows when the text is exactly one
    /// segment, and joins the segments otherwise.
    pub fn plain_text(&self) -> Cow<'_, str> {
        match self.segments.as_slice() {
            [segment] => Cow::Borrowed(&segment.content),
            _ => Cow::Owned(self.to_string()),
        }
    }

    /// Each segment paired with its byte range in the flattened text.
    pub fn segment_ranges(&self) -> impl Iterator<Item = (Range<usize>, &TextSegment)> {
        self.segments.iter().scan(0, |start, segment| {
            let range = *start..*start + segment.content.len();
            *start = range.end;
            Some((range, segment))
        })
    }

    pub fn link_ranges(&self) -> Vec<Range<usize>> {
        self.segment_ranges()
            .filter(|(_, segment)| segment.has_style(Style::Link))
            .map(|(range, _)| range)
            .collect()
    }
}

impl From<Vec<TextSegment>> for RichText {
    fn from(segments: Vec<TextSegment>) -> Self {
        Self { segments }
    }
}

impl<const N: usize> From<[TextSegment; N]> for RichText {
    fn from(segments: [TextSegment; N]) -> Self {
        Self {
            segments: segments.to_vec(),
        }
    }
}
