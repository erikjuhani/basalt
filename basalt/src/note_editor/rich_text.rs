use std::{fmt, slice::Iter, vec::IntoIter};

use pulldown_cmark::CowStr;

/// A style that can be applied to [`TextSegment`] (code, emphasis, strikethrough, strong).
#[derive(Clone, Debug, PartialEq)]
pub enum Style {
    Code,
    Emphasis,
    Strong,
    Strikethrough,
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Style::Code => write!(f, "Code"),
            Style::Emphasis => write!(f, "Emphasis"),
            Style::Strong => write!(f, "Strong"),
            Style::Strikethrough => write!(f, "Strikethrough"),
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
        // TODO: Control tab length with config value.
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
        write!(
            f,
            "{}",
            self.segments
                .iter()
                .map(|segment| segment.to_string())
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

impl RichText {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> Iter<'_, TextSegment> {
        self.segments.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn push_text_segment(&mut self, text_segment: TextSegment) {
        self.segments.push(text_segment);
    }

    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
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
