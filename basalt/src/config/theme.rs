use std::{collections::HashMap, fs::read_to_string, str::FromStr};

use etcetera::{choose_base_strategy, BaseStrategy};
use ratatui::{style::Color, widgets, widgets::Borders};
use serde::Deserialize;

/// Semantic colour roles for the whole UI. Every hard-coded colour in the
/// renderer resolves through one of these, so swapping a [`Theme`] re-skins
/// the application. [`Theme::default`] reproduces the original palette, so the
/// default appearance is unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Primary foreground (terminal default).
    pub text: Color,
    /// Base background painted across the whole UI (terminal default by default).
    pub background: Color,
    /// Secondary text: markers, indentation, bullets, badges.
    pub muted: Color,
    /// Accent: raw-mode markers and other highlights.
    pub accent: Color,
    /// Default border colours and line type for every pane (each pane may
    /// override its own in a `[pane]` section).
    pub border: Color,
    pub border_active: Color,
    pub border_type: Option<BorderKind>,
    pub border_edges: Edges,
    pub heading_1: Color,
    pub heading_2: Color,
    pub heading_3: Color,
    pub heading_4: Color,
    pub heading_5: Color,
    pub heading_6: Color,
    /// Background for fenced code blocks.
    pub code_bg: Color,
    /// Block-quote bar and text.
    pub blockquote: Color,
    /// List bullets and ordered-list numbers.
    pub list_marker: Color,
    /// Task check-box marker.
    pub task: Color,
    /// Editor mode indicator: insert / edit (writing).
    pub mode_insert: Color,
    /// Editor mode indicator: vim normal.
    pub mode_normal: Color,
    /// Editor mode indicator: read-only.
    pub mode_read: Color,
    pub success: Color,
    pub info: Color,
    pub warning: Color,
    pub error: Color,
    /// Per-pane background and border overrides.
    pub explorer: Pane,
    pub note_editor: Pane,
    pub outline: Pane,
    pub status_bar: StatusBar,
}

/// Background and border styling for a single bordered pane. Colours default to
/// the theme's global background / terminal default; `border_type` defers to the
/// [`Symbols`](super::Symbols) preset when unset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pane {
    pub background: Color,
    pub border: Color,
    pub border_active: Color,
    pub border_type: Option<BorderKind>,
    pub border_edges: Edges,
}

/// Which sides of a pane draw a border. Lets a theme keep only the dividers
/// between panes (e.g. explorer `right`, outline `left`, editor `none`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Edges {
    #[default]
    All,
    None,
    Top,
    Bottom,
    Left,
    Right,
    /// Left and right.
    Vertical,
    /// Top and bottom.
    Horizontal,
}

impl Edges {
    pub fn to_borders(self) -> Borders {
        match self {
            Edges::All => Borders::ALL,
            Edges::None => Borders::NONE,
            Edges::Top => Borders::TOP,
            Edges::Bottom => Borders::BOTTOM,
            Edges::Left => Borders::LEFT,
            Edges::Right => Borders::RIGHT,
            Edges::Vertical => Borders::LEFT | Borders::RIGHT,
            Edges::Horizontal => Borders::TOP | Borders::BOTTOM,
        }
    }
}

impl Pane {
    /// Border colour by focus state.
    pub fn border(&self, active: bool) -> Color {
        if active {
            self.border_active
        } else {
            self.border
        }
    }

    /// Effective border line: the theme's `border-type` when set, otherwise the
    /// given fallback (from the [`Symbols`](super::Symbols) preset). `None` means
    /// the border is removed.
    pub fn border_line(&self, fallback: widgets::BorderType) -> Option<widgets::BorderType> {
        match self.border_type {
            Some(kind) => kind.line(),
            None => Some(fallback),
        }
    }

    /// Borders for a folded pane: a full-box theme keeps the 3-sided `strip`; a
    /// divider theme keeps only its own edge so the folded pane connects to its
    /// neighbour instead of floating as a box.
    pub fn collapsed_borders(&self, strip: Borders) -> Borders {
        match self.border_edges {
            Edges::All => strip,
            edges => edges.to_borders(),
        }
    }
}

/// Background and foreground for the status bar (which has no border).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusBar {
    pub background: Color,
    pub foreground: Color,
}

/// Border line style for a pane. `None` removes the border entirely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BorderKind {
    None,
    Plain,
    Rounded,
    Thick,
    Double,
}

impl BorderKind {
    /// Ratatui line type, or `None` when the border should not be drawn.
    pub fn line(self) -> Option<widgets::BorderType> {
        match self {
            BorderKind::None => None,
            BorderKind::Plain => Some(widgets::BorderType::Plain),
            BorderKind::Rounded => Some(widgets::BorderType::Rounded),
            BorderKind::Thick => Some(widgets::BorderType::Thick),
            BorderKind::Double => Some(widgets::BorderType::Double),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        let pane = Pane {
            background: Color::Reset,
            border: Color::Reset,
            border_active: Color::Reset,
            border_type: None,
            border_edges: Edges::All,
        };
        Self {
            text: Color::Reset,
            background: Color::Reset,
            muted: Color::DarkGray,
            accent: Color::Magenta,
            border: Color::Reset,
            border_active: Color::Reset,
            border_type: None,
            border_edges: Edges::All,
            heading_1: Color::Reset,
            heading_2: Color::Yellow,
            heading_3: Color::Cyan,
            heading_4: Color::Magenta,
            heading_5: Color::Reset,
            heading_6: Color::Reset,
            code_bg: Color::Black,
            blockquote: Color::Magenta,
            list_marker: Color::DarkGray,
            task: Color::Magenta,
            mode_insert: Color::Green,
            mode_normal: Color::Gray,
            mode_read: Color::Gray,
            success: Color::Green,
            info: Color::Blue,
            warning: Color::Yellow,
            error: Color::Red,
            explorer: pane,
            note_editor: pane,
            outline: pane,
            status_bar: StatusBar {
                background: Color::Reset,
                foreground: Color::Reset,
            },
        }
    }
}

impl Theme {
    /// Heading colour for a level (`1..=6`); out-of-range falls back to [`text`].
    pub fn heading(&self, level: usize) -> Color {
        match level {
            1 => self.heading_1,
            2 => self.heading_2,
            3 => self.heading_3,
            4 => self.heading_4,
            5 => self.heading_5,
            6 => self.heading_6,
            _ => self.text,
        }
    }
}

/// A theme as written in a TOML file: a `[palette]` of named colours, a value
/// per global role, and optional `[explorer]`, `[note-editor]`, `[outline]`
/// and `[status-bar]` tables. A colour value is either a palette key or a
/// literal (`#rrggbb` or an ANSI name). Unset roles keep the [`Theme::default`]
/// colour; unset pane backgrounds inherit the theme background.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlTheme {
    #[serde(default)]
    palette: HashMap<String, String>,
    text: Option<String>,
    background: Option<String>,
    muted: Option<String>,
    accent: Option<String>,
    border: Option<String>,
    border_active: Option<String>,
    border_type: Option<BorderKind>,
    border_edges: Option<Edges>,
    heading_1: Option<String>,
    heading_2: Option<String>,
    heading_3: Option<String>,
    heading_4: Option<String>,
    heading_5: Option<String>,
    heading_6: Option<String>,
    code_bg: Option<String>,
    blockquote: Option<String>,
    list_marker: Option<String>,
    task: Option<String>,
    mode_insert: Option<String>,
    mode_normal: Option<String>,
    mode_read: Option<String>,
    success: Option<String>,
    info: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    #[serde(default)]
    explorer: TomlPane,
    #[serde(default)]
    note_editor: TomlPane,
    #[serde(default)]
    outline: TomlPane,
    #[serde(default)]
    status_bar: TomlStatusBar,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlPane {
    background: Option<String>,
    border: Option<String>,
    border_active: Option<String>,
    border_type: Option<BorderKind>,
    border_edges: Option<Edges>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlStatusBar {
    background: Option<String>,
    foreground: Option<String>,
}

/// Resolves a role value to a colour: a `[palette]` key, else a literal
/// (`#rrggbb` or ANSI name), else the fallback.
fn resolve(palette: &HashMap<String, String>, role: Option<String>, fallback: Color) -> Color {
    role.map(|color| {
        let literal = palette.get(&color).unwrap_or(&color);
        Color::from_str(literal).unwrap_or(fallback)
    })
    .unwrap_or(fallback)
}

fn resolve_pane(palette: &HashMap<String, String>, toml: TomlPane, default: Pane) -> Pane {
    Pane {
        background: resolve(palette, toml.background, default.background),
        border: resolve(palette, toml.border, default.border),
        border_active: resolve(palette, toml.border_active, default.border_active),
        border_type: toml.border_type.or(default.border_type),
        border_edges: toml.border_edges.unwrap_or(default.border_edges),
    }
}

impl From<TomlTheme> for Theme {
    fn from(value: TomlTheme) -> Self {
        let default = Theme::default();
        let palette = &value.palette;
        let color = |role, fallback| resolve(palette, role, fallback);

        // Panes and the status bar inherit the global background unless they set
        // their own, so a single `background` tints the whole UI consistently.
        let background = color(value.background, default.background);
        let pane_default = Pane {
            background,
            border: color(value.border, default.border),
            border_active: color(value.border_active, default.border_active),
            border_type: value.border_type.or(default.border_type),
            border_edges: value.border_edges.unwrap_or(default.border_edges),
        };

        Self {
            text: color(value.text, default.text),
            background,
            muted: color(value.muted, default.muted),
            accent: color(value.accent, default.accent),
            border: pane_default.border,
            border_active: pane_default.border_active,
            border_type: pane_default.border_type,
            border_edges: pane_default.border_edges,
            heading_1: color(value.heading_1, default.heading_1),
            heading_2: color(value.heading_2, default.heading_2),
            heading_3: color(value.heading_3, default.heading_3),
            heading_4: color(value.heading_4, default.heading_4),
            heading_5: color(value.heading_5, default.heading_5),
            heading_6: color(value.heading_6, default.heading_6),
            code_bg: color(value.code_bg, default.code_bg),
            blockquote: color(value.blockquote, default.blockquote),
            list_marker: color(value.list_marker, default.list_marker),
            task: color(value.task, default.task),
            mode_insert: color(value.mode_insert, default.mode_insert),
            mode_normal: color(value.mode_normal, default.mode_normal),
            mode_read: color(value.mode_read, default.mode_read),
            success: color(value.success, default.success),
            info: color(value.info, default.info),
            warning: color(value.warning, default.warning),
            error: color(value.error, default.error),
            explorer: resolve_pane(palette, value.explorer, pane_default),
            note_editor: resolve_pane(palette, value.note_editor, pane_default),
            outline: resolve_pane(palette, value.outline, pane_default),
            status_bar: StatusBar {
                background: resolve(palette, value.status_bar.background, background),
                foreground: resolve(
                    palette,
                    value.status_bar.foreground,
                    default.status_bar.foreground,
                ),
            },
        }
    }
}

fn parse_theme(toml: &str) -> Theme {
    toml::from_str::<TomlTheme>(toml)
        .map(Theme::from)
        .unwrap_or_default()
}

/// Built-in themes embedded at compile time, in display order.
const BUILTIN_THEMES: &[(&str, &str)] = &[
    (
        "default",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/themes/default.toml")),
    ),
    (
        "causeway-dark",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/causeway-dark.toml"
        )),
    ),
    (
        "causeway-light",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/causeway-light.toml"
        )),
    ),
    (
        "gruvbox-dark",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/gruvbox-dark.toml"
        )),
    ),
    (
        "gruvbox-light",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/gruvbox-light.toml"
        )),
    ),
    (
        "nord",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/themes/nord.toml")),
    ),
    (
        "dracula",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/themes/dracula.toml")),
    ),
    (
        "catppuccin-latte",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/catppuccin-latte.toml"
        )),
    ),
    (
        "catppuccin-frappe",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/catppuccin-frappe.toml"
        )),
    ),
    (
        "catppuccin-macchiato",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/catppuccin-macchiato.toml"
        )),
    ),
    (
        "catppuccin-mocha",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/catppuccin-mocha.toml"
        )),
    ),
    (
        "everforest-dark",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/everforest-dark.toml"
        )),
    ),
    (
        "everforest-light",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/themes/everforest-light.toml"
        )),
    ),
    (
        "minimal",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/themes/minimal.toml")),
    ),
];

/// User themes live in `$config/basalt/themes/*.toml`.
fn user_themes_dir() -> Option<std::path::PathBuf> {
    choose_base_strategy()
        .ok()
        .map(|strategy| strategy.config_dir().join("basalt/themes"))
}

fn user_themes() -> Vec<(String, Theme)> {
    let Some(dir) = user_themes_dir() else {
        return vec![];
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let theme = parse_theme(&read_to_string(&path).ok()?);
            Some((name, theme))
        })
        .collect()
}

/// All available themes, in picker order: built-ins first, then user themes
/// from `$config/basalt/themes/`. A user theme with a built-in's name overrides it.
pub fn load_themes() -> Vec<(String, Theme)> {
    let mut themes: Vec<(String, Theme)> = BUILTIN_THEMES
        .iter()
        .map(|(name, toml)| (name.to_string(), parse_theme(toml)))
        .collect();

    for (name, theme) in user_themes() {
        match themes.iter_mut().find(|(existing, _)| *existing == name) {
            Some((_, existing)) => *existing = theme,
            None => themes.push((name, theme)),
        }
    }

    themes
}

/// Resolves a theme by name, falling back to the default theme when unknown.
pub fn theme_by_name(name: &str) -> Theme {
    load_themes()
        .into_iter()
        .find(|(theme_name, _)| theme_name == name)
        .map(|(_, theme)| theme)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_default_matches_struct_default() {
        let (_, default) = load_themes()
            .into_iter()
            .find(|(name, _)| name == "default")
            .unwrap();
        assert_eq!(default, Theme::default());
    }

    #[test]
    fn resolves_palette_and_literals() {
        let toml = r##"
            accent = "red"
            muted = "#102030"
            error = "green"

            [palette]
            red = "#ff0000"
        "##;
        let theme = parse_theme(toml);
        assert_eq!(theme.accent, Color::Rgb(255, 0, 0));
        assert_eq!(theme.muted, Color::Rgb(16, 32, 48));
        assert_eq!(theme.error, Color::Green);
    }

    #[test]
    fn unset_roles_fall_back_to_default() {
        let theme = parse_theme("accent = \"#abcdef\"");
        assert_eq!(theme.accent, Color::Rgb(0xab, 0xcd, 0xef));
        assert_eq!(theme.muted, Theme::default().muted);
        assert_eq!(theme.heading_2, Theme::default().heading_2);
    }

    #[test]
    fn resolves_pane_sections() {
        let theme = parse_theme(
            r##"
            background = "#000000"

            [explorer]
            background = "surface"
            border = "#111111"
            border-active = "#00ff00"
            border-type = "none"

            [palette]
            surface = "#101010"
        "##,
        );
        assert_eq!(theme.explorer.background, Color::Rgb(16, 16, 16));
        assert_eq!(theme.explorer.border, Color::Rgb(0x11, 0x11, 0x11));
        assert_eq!(theme.explorer.border(true), Color::Rgb(0, 255, 0));
        assert_eq!(theme.explorer.border_type, Some(BorderKind::None));
        // Unset pane inherits the global background and defers its border type.
        assert_eq!(theme.note_editor.background, Color::Rgb(0, 0, 0));
        assert_eq!(theme.note_editor.border_type, None);
    }

    #[test]
    fn resolves_border_edges() {
        let theme = parse_theme(
            r##"
            [explorer]
            border-edges = "right"

            [outline]
            border-edges = "left"
        "##,
        );
        assert_eq!(theme.explorer.border_edges, Edges::Right);
        assert_eq!(theme.outline.border_edges, Edges::Left);
        assert_eq!(theme.explorer.border_edges.to_borders(), Borders::RIGHT);
        // Unset panes keep the default (all edges).
        assert_eq!(theme.note_editor.border_edges, Edges::All);
    }

    #[test]
    fn status_bar_section() {
        let theme = parse_theme(
            r##"
            [status-bar]
            background = "#222222"
            foreground = "#eeeeee"
        "##,
        );
        assert_eq!(theme.status_bar.background, Color::Rgb(0x22, 0x22, 0x22));
        assert_eq!(theme.status_bar.foreground, Color::Rgb(0xee, 0xee, 0xee));
    }

    #[test]
    fn all_builtins_parse() {
        let themes = load_themes();
        for name in [
            "causeway-dark",
            "causeway-light",
            "gruvbox-dark",
            "gruvbox-light",
            "nord",
            "dracula",
            "catppuccin-latte",
            "catppuccin-frappe",
            "catppuccin-macchiato",
            "catppuccin-mocha",
            "everforest-dark",
            "everforest-light",
            "minimal",
        ] {
            assert!(
                themes.iter().any(|(theme, _)| theme == name),
                "missing {name}"
            );
        }
    }
}
