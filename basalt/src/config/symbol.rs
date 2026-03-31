use ratatui::widgets;
use serde::Deserialize;

use crate::{
    config::env::{self, Env},
    stylized_text::FontStyle,
};

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    #[default]
    Auto,
    Unicode,
    Ascii,
    NerdFont,
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Deserialize)]
pub enum BorderType {
    #[default]
    Plain,
    Double,
    Rounded,
    Thick,
}

impl From<BorderType> for widgets::BorderType {
    fn from(value: BorderType) -> Self {
        match value {
            BorderType::Plain => Self::Plain,
            BorderType::Double => Self::Double,
            BorderType::Rounded => Self::Rounded,
            BorderType::Thick => Self::Thick,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Symbols {
    pub preset: Preset,
    pub border_active: BorderType,
    pub border_inactive: BorderType,
    pub border_modal: BorderType,
    pub wrap_marker: String,
    pub tree_indent: String,
    pub tree_expanded: String,
    pub tree_collapsed: String,
    pub selected: String,
    pub unselected: String,
    pub vault_active: String,
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
    pub toast_success: String,
    pub toast_info: String,
    pub toast_error: String,
    pub toast_warning: String,
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
            border_active,
            border_inactive,
            border_modal,
            wrap_marker,
            tree_indent,
            tree_expanded,
            tree_collapsed,
            selected,
            unselected,
            vault_active,
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
            outline_heading_collapsed,
            toast_success,
            toast_info,
            toast_error,
            toast_warning
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
            border_active: BorderType::Double,
            border_inactive: BorderType::Plain,
            border_modal: BorderType::Plain,
            wrap_marker: "".into(),
            tree_indent: "|".into(),
            tree_expanded: "v".into(),
            tree_collapsed: ">".into(),
            selected: "*".into(),
            unselected: ".".into(),
            vault_active: "*".into(),
            pane_open: ">".into(),
            pane_close: "<".into(),
            pane_full: "=>".into(),
            sort_asc: "^=".into(),
            sort_desc: "v=".into(),
            h1_underline: "═".into(),
            h2_underline: "─".into(),
            h3_marker: "###".into(),
            h4_marker: "####".into(),
            h5_marker: "#####".into(),
            h6_marker: "######".into(),
            task_unchecked: "[ ]".into(),
            task_checked: "[x]".into(),
            blockquote_border: "|".into(),
            horizontal_rule: "═".into(),
            folder_expanded_collapsed: "+".into(),
            folder_collapsed_collapsed: "-".into(),
            heading_collapsed_dot: ".".into(),
            outline_indent: "|".into(),
            outline_expanded: "v".into(),
            outline_collapsed: ">".into(),
            outline_heading_dot: ".".into(),
            outline_heading_expanded: "#".into(),
            outline_heading_collapsed: ">".into(),
            toast_success: "+".into(),
            toast_info: "i".into(),
            toast_error: "x".into(),
            toast_warning: "!".into(),
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
            border_active: BorderType::Thick,
            border_inactive: BorderType::Rounded,
            border_modal: BorderType::Rounded,
            wrap_marker: "⤷ ".into(),
            tree_indent: "│".into(),
            tree_expanded: "▾".into(),
            tree_collapsed: "▸".into(),
            selected: "◆".into(),
            unselected: "◦".into(),
            vault_active: "◆".into(),
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
            toast_success: "✓".into(),
            toast_info: "ⓘ".into(),
            toast_error: "✗".into(),
            toast_warning: "⚠".into(),
            list_markers: vec!["●".into(), "○".into(), "◆".into(), "◇".into()],
            title_font_style: Some(FontStyle::BlackBoardBold),
            h5_font_style: Some(FontStyle::Script),
            h6_font_style: Some(FontStyle::Script),
        }
    }

    pub fn nerd_font() -> Self {
        Self {
            preset: Preset::NerdFont,
            border_active: BorderType::Thick,
            border_inactive: BorderType::Rounded,
            border_modal: BorderType::Rounded,
            wrap_marker: "⤷ ".into(),
            tree_indent: "│".into(),
            tree_expanded: "\u{f07c}".into(),
            tree_collapsed: "\u{f07b}".into(),
            selected: "\u{f15b}".into(),
            unselected: "\u{ea7b}".into(),
            vault_active: "◆".into(),
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
            toast_success: "\u{f00c}".into(),
            toast_info: "\u{f05a}".into(),
            toast_error: "\u{f00d}".into(),
            toast_warning: "\u{f071}".into(),
            list_markers: vec!["●".into(), "○".into(), "◆".into(), "◇".into()],
            title_font_style: Some(FontStyle::BlackBoardBold),
            h5_font_style: Some(FontStyle::Script),
            h6_font_style: Some(FontStyle::Script),
        }
    }

    pub fn from_preset(preset: &Preset) -> Self {
        match preset {
            Preset::Auto => Self::from_preset(&detect_preset(env::SystemEnv)),
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
    border_active: Option<BorderType>,
    border_inactive: Option<BorderType>,
    border_modal: Option<BorderType>,
    wrap_marker: Option<String>,
    tree_indent: Option<String>,
    tree_expanded: Option<String>,
    tree_collapsed: Option<String>,
    selected: Option<String>,
    unselected: Option<String>,
    vault_active: Option<String>,
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
    toast_success: Option<String>,
    toast_info: Option<String>,
    toast_error: Option<String>,
    toast_warning: Option<String>,
    list_markers: Option<Vec<String>>,
    title_font_style: Option<FontStyle>,
    h5_font_style: Option<FontStyle>,
    h6_font_style: Option<FontStyle>,
}

pub fn detect_preset(env: impl Env) -> Preset {
    let is_dumb_terminal = || -> bool {
        env.var("TERM")
            .map(|value| value == "dumb")
            .unwrap_or(false)
    };

    let is_utf8_locale = || -> bool {
        env.var("LC_ALL")
            .or_else(|| env.var("LC_CTYPE"))
            .or_else(|| env.var("LANG"))
            .map(|locale| {
                let locale = locale.to_ascii_lowercase();
                locale.contains("utf-8") || locale.contains("utf8")
            })
            .unwrap_or(false)
    };

    let is_linux_framebuffer = || -> bool {
        env.var("TERM")
            .map(|value| value == "linux")
            .unwrap_or(false)
    };

    if is_dumb_terminal() || !is_utf8_locale() || is_linux_framebuffer() {
        Preset::Ascii
    } else {
        Preset::Unicode
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    struct TestEnv(HashMap<&'static str, &'static str>);

    impl Env for TestEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.get(key).map(|v| v.to_string())
        }
    }

    fn env_from(pairs: &[(&'static str, &'static str)]) -> TestEnv {
        TestEnv(pairs.iter().copied().collect())
    }

    #[test]
    fn dumb_terminal_returns_ascii() {
        let env = env_from(&[("TERM", "dumb"), ("LANG", "en_US.UTF-8")]);
        assert_eq!(detect_preset(env), Preset::Ascii);
    }

    #[test]
    fn linux_framebuffer_returns_ascii() {
        let env = env_from(&[("TERM", "linux"), ("LANG", "en_US.UTF-8")]);
        assert_eq!(detect_preset(env), Preset::Ascii);
    }

    #[test]
    fn no_utf8_locale_returns_ascii() {
        let env = env_from(&[("TERM", "xterm-256color"), ("LANG", "C")]);
        assert_eq!(detect_preset(env), Preset::Ascii);
    }

    #[test]
    fn no_locale_vars_returns_ascii() {
        let env = env_from(&[("TERM", "xterm-256color")]);
        assert_eq!(detect_preset(env), Preset::Ascii);
    }

    #[test]
    fn utf8_locale_returns_unicode() {
        let env = env_from(&[("TERM", "xterm-256color"), ("LANG", "en_US.UTF-8")]);
        assert_eq!(detect_preset(env), Preset::Unicode);
    }

    #[test]
    fn lc_all_takes_priority_over_lang() {
        let env = env_from(&[("TERM", "xterm"), ("LC_ALL", "en_US.UTF-8"), ("LANG", "C")]);
        assert_eq!(detect_preset(env), Preset::Unicode);
    }

    #[test]
    fn lc_ctype_takes_priority_over_lang() {
        let env = env_from(&[
            ("TERM", "xterm"),
            ("LC_CTYPE", "en_US.UTF-8"),
            ("LANG", "C"),
        ]);
        assert_eq!(detect_preset(env), Preset::Unicode);
    }

    #[test]
    fn lc_all_non_utf8_overrides_utf8_lang() {
        let env = env_from(&[("TERM", "xterm"), ("LC_ALL", "C"), ("LANG", "en_US.UTF-8")]);
        assert_eq!(detect_preset(env), Preset::Ascii);
    }

    #[test]
    fn utf8_lowercase_detected() {
        let env = env_from(&[("TERM", "xterm"), ("LANG", "en_US.utf8")]);
        assert_eq!(detect_preset(env), Preset::Unicode);
    }

    #[test]
    fn no_env_vars_returns_ascii() {
        let env = env_from(&[]);
        assert_eq!(detect_preset(env), Preset::Ascii);
    }
}
