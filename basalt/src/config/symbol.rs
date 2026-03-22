use serde::Deserialize;

use crate::stylized_text::FontStyle;

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    #[default]
    Unicode,
    Ascii,
    NerdFont,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Symbols {
    preset: Preset,
    pub wrap_marker: String,
    pub tree_indent: String,
    pub tree_expanded: String,
    pub tree_collapsed: String,
    pub selected: String,
    pub unselected: String,
    pub pane_open: String,
    pub pane_close: String,
    pub pane_full: String,
    pub sort_asc: String,
    pub sort_desc: String,
    pub h1_underline: String,
    pub h2_underline: String,
    pub h3_marker: String,
    pub h4_marker: String,
    pub h5_marker: String,
    pub h6_marker: String,
    pub task_unchecked: String,
    pub task_checked: String,
    pub blockquote_border: String,
    pub horizontal_rule: String,
    pub folder_expanded_collapsed: String,
    pub folder_collapsed_collapsed: String,
    pub heading_collapsed_dot: String,
    pub outline_indent: String,
    pub outline_expanded: String,
    pub outline_collapsed: String,
    pub outline_heading_dot: String,
    pub outline_heading_expanded: String,
    pub outline_heading_collapsed: String,
    pub list_markers: Vec<String>,
    pub title_font_style: Option<FontStyle>,
    pub h5_font_style: Option<FontStyle>,
    pub h6_font_style: Option<FontStyle>,
}

impl From<TomlSymbols> for Symbols {
    fn from(value: TomlSymbols) -> Self {
        let mut symbols = Symbols::from_preset(&value.preset);

        macro_rules! override_if_set {
          ($($field:ident),*) => {
              $(if let Some(v) = value.$field { symbols.$field = v; })*
          };
        }

        override_if_set!(
            wrap_marker,
            tree_indent,
            tree_expanded,
            tree_collapsed,
            selected,
            unselected,
            pane_open,
            pane_close,
            pane_full,
            sort_asc,
            sort_desc,
            h1_underline,
            h2_underline,
            h3_marker,
            h4_marker,
            h5_marker,
            h6_marker,
            task_unchecked,
            task_checked,
            blockquote_border,
            horizontal_rule,
            folder_expanded_collapsed,
            folder_collapsed_collapsed,
            heading_collapsed_dot,
            outline_indent,
            outline_expanded,
            outline_collapsed,
            outline_heading_dot,
            outline_heading_expanded,
            outline_heading_collapsed
        );

        macro_rules! override_option_if_set {
            ($($field:ident),*) => {
                $(if value.$field.is_some() { symbols.$field = value.$field; })*
            };
        }

        override_option_if_set!(title_font_style, h5_font_style, h6_font_style);

        if let Some(v) = value.list_markers {
            symbols.list_markers = v;
        }

        symbols
    }
}

impl Default for Symbols {
    fn default() -> Self {
        Self {
            preset: Preset::Ascii,
            wrap_marker: "".into(),
            tree_indent: "|".into(),
            tree_expanded: "v".into(),
            tree_collapsed: ">".into(),
            selected: "*".into(),
            unselected: ".".into(),
            pane_open: ">".into(),
            pane_close: "<".into(),
            pane_full: "=>".into(),
            sort_asc: "^=".into(),
            sort_desc: "v=".into(),
            h1_underline: "=".into(),
            h2_underline: "-".into(),
            h3_marker: "###".into(),
            h4_marker: "####".into(),
            h5_marker: "#####".into(),
            h6_marker: "######".into(),
            task_unchecked: "[ ]".into(),
            task_checked: "[x]".into(),
            blockquote_border: "|".into(),
            horizontal_rule: "=".into(),
            folder_expanded_collapsed: "+".into(),
            folder_collapsed_collapsed: "-".into(),
            heading_collapsed_dot: ".".into(),
            outline_indent: "|".into(),
            outline_expanded: "v".into(),
            outline_collapsed: ">".into(),
            outline_heading_dot: ".".into(),
            outline_heading_expanded: "v".into(),
            outline_heading_collapsed: ">".into(),
            list_markers: vec!["-".into(), "*".into(), "+".into()],
            title_font_style: None,
            h5_font_style: None,
            h6_font_style: None,
        }
    }
}

impl Symbols {
    pub fn ascii() -> Self {
        Self::default()
    }

    pub fn unicode() -> Self {
        Self {
            preset: Preset::Unicode,
            wrap_marker: "⤷ ".into(),
            tree_indent: "│".into(),
            tree_expanded: "▾".into(),
            tree_collapsed: "▸".into(),
            selected: "◆".into(),
            unselected: "◦".into(),
            pane_open: "▶".into(),
            pane_close: "◀".into(),
            pane_full: "⟹ ".into(),
            sort_asc: "↑≡".into(),
            sort_desc: "↓≡".into(),
            h1_underline: "═".into(),
            h2_underline: "─".into(),
            h3_marker: "◉".into(),
            h4_marker: "◎".into(),
            h5_marker: "◈".into(),
            h6_marker: "✦".into(),
            task_unchecked: "□".into(),
            task_checked: "■".into(),
            blockquote_border: "┃".into(),
            horizontal_rule: "═".into(),
            folder_expanded_collapsed: "▪".into(),
            folder_collapsed_collapsed: "▫".into(),
            heading_collapsed_dot: "·".into(),
            outline_indent: "│".into(),
            outline_expanded: "▾".into(),
            outline_collapsed: "▸".into(),
            outline_heading_dot: "·".into(),
            outline_heading_expanded: "✺".into(),
            outline_heading_collapsed: "◦".into(),
            list_markers: vec!["●".into(), "○".into(), "◆".into(), "◇".into()],
            title_font_style: Some(FontStyle::BlackBoardBold),
            h5_font_style: Some(FontStyle::Script),
            h6_font_style: Some(FontStyle::Script),
        }
    }

    pub fn nerd_font() -> Self {
        Self {
            preset: Preset::NerdFont,
            wrap_marker: "⤷ ".into(),
            tree_indent: "│".into(),
            tree_expanded: "\u{f07c}".into(),
            tree_collapsed: "\u{f07b}".into(),
            selected: "\u{f15b}".into(),
            unselected: "\u{ea7b}".into(),
            pane_open: "▶".into(),
            pane_close: "◀".into(),
            pane_full: "⟹ ".into(),
            sort_asc: "\u{f15d}".into(),
            sort_desc: "\u{f15e}".into(),
            h1_underline: "═".into(),
            h2_underline: "─".into(),
            h3_marker: "◉".into(),
            h4_marker: "◎".into(),
            h5_marker: "◈".into(),
            h6_marker: "✦".into(),
            task_unchecked: "󰄱".into(),
            task_checked: "󰄲".into(),
            blockquote_border: "┃".into(),
            horizontal_rule: "═".into(),
            folder_expanded_collapsed: "\u{f07c}".into(),
            folder_collapsed_collapsed: "\u{f07b}".into(),
            heading_collapsed_dot: "·".into(),
            outline_indent: "│".into(),
            outline_expanded: "▾".into(),
            outline_collapsed: "▸".into(),
            outline_heading_dot: "·".into(),
            outline_heading_expanded: "✺".into(),
            outline_heading_collapsed: "◦".into(),
            list_markers: vec!["●".into(), "○".into(), "◆".into(), "◇".into()],
            title_font_style: Some(FontStyle::BlackBoardBold),
            h5_font_style: Some(FontStyle::Script),
            h6_font_style: Some(FontStyle::Script),
        }
    }

    pub fn from_preset(preset: &Preset) -> Self {
        match preset {
            Preset::Ascii => Self::ascii(),
            Preset::Unicode => Self::unicode(),
            Preset::NerdFont => Self::nerd_font(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct TomlSymbols {
    #[serde(default)]
    preset: Preset,
    wrap_marker: Option<String>,
    tree_indent: Option<String>,
    tree_expanded: Option<String>,
    tree_collapsed: Option<String>,
    selected: Option<String>,
    unselected: Option<String>,
    pane_open: Option<String>,
    pane_close: Option<String>,
    pane_full: Option<String>,
    sort_asc: Option<String>,
    sort_desc: Option<String>,
    h1_underline: Option<String>,
    h2_underline: Option<String>,
    h3_marker: Option<String>,
    h4_marker: Option<String>,
    h5_marker: Option<String>,
    h6_marker: Option<String>,
    task_unchecked: Option<String>,
    task_checked: Option<String>,
    blockquote_border: Option<String>,
    horizontal_rule: Option<String>,
    folder_expanded_collapsed: Option<String>,
    folder_collapsed_collapsed: Option<String>,
    heading_collapsed_dot: Option<String>,
    outline_indent: Option<String>,
    outline_expanded: Option<String>,
    outline_collapsed: Option<String>,
    outline_heading_dot: Option<String>,
    outline_heading_expanded: Option<String>,
    outline_heading_collapsed: Option<String>,
    list_markers: Option<Vec<String>>,
    title_font_style: Option<FontStyle>,
    h5_font_style: Option<FontStyle>,
    h6_font_style: Option<FontStyle>,
}
