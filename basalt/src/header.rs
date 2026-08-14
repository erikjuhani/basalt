use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::{symbol::Preset, Symbols, Theme},
    tabs::Tabs,
};

pub struct Header<'a, 'b> {
    symbols: &'a Symbols,
    theme: &'a Theme,
    tabs: &'a Tabs<'b>,
}

const MAX_TAB_WIDTH: usize = 24;
const MIN_TAB_WIDTH: usize = 12;

fn truncate(label: &str, max_width: usize, ellipsis: &str) -> String {
    if label.width() <= max_width {
        return label.to_string();
    }

    // Leave room for the ellipsis
    let max_width = max_width.saturating_sub(ellipsis.width());

    let mut truncated = label
        .char_indices()
        .take_while(|(i, _)| *i < max_width)
        .map(|(_, c)| c)
        .collect::<String>();

    truncated.push_str(ellipsis);
    truncated
}

impl<'a, 'b> Header<'a, 'b> {
    pub fn new(symbols: &'a Symbols, theme: &'a Theme, tabs: &'a Tabs<'b>) -> Self {
        Self {
            symbols,
            theme,
            tabs,
        }
    }
}

impl Widget for Header<'_, '_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Block::new()
            .style(Style::new().bg(self.theme.background))
            .render(area, buf);

        let brand = Line::from(Span::from(" ⋅𝕭𝖆𝖘𝖆𝖑𝖙⋅ ").fg(self.theme.accent).bold());

        let brand_width = (brand.width() as u16).min(area.width);
        let [brand_area, tabs_area] =
            Layout::horizontal([Constraint::Length(brand_width), Constraint::Fill(1)]).areas(area);
        brand.render(brand_area, buf);

        // A theme that owns a real background (any colour scheme) gets the
        // bufferline style: an accent left bar and a raised surface highlight.
        // The default theme keeps the terminal palette, so it renders the
        // classic reversed tabs instead.
        let themed = self.theme.background != Color::Reset;

        let unicode = self.symbols.preset != Preset::Ascii;
        let (separator, active_bar, modified_glyph, ellipsis) = if unicode {
            ("▍", "▎", "●", "…")
        } else {
            ("|", "|", "*", "...")
        };

        let titles = self.tabs.titles();
        if titles.is_empty() {
            return;
        }

        let available = tabs_area.width as usize;
        let tab_width = (available / titles.len()).clamp(MIN_TAB_WIDTH, MAX_TAB_WIDTH);
        let inner = tab_width.saturating_sub(1);

        let background = self.theme.background;
        let muted = self.theme.muted;
        let text = self.theme.text;
        let accent = self.theme.accent;
        let highlight = self.theme.code_bg;

        let mut tabs: Vec<Vec<Span>> = Vec::new();
        let mut active_index = 0;
        for (index, (name, active, modified)) in titles.iter().enumerate() {
            let suffix = if *modified {
                format!(" {modified_glyph}")
            } else {
                String::new()
            };
            let name = truncate(name, inner.saturating_sub(suffix.width()), ellipsis);
            let padding = inner.saturating_sub(name.width() + suffix.width());
            let left = padding / 2;
            let right = padding - left;
            let lead = if index > 0 { separator } else { " " };

            if *active {
                active_index = index;
            }

            let tab = match (themed, *active) {
                // Bufferline: accent left bar + raised surface + brighter text.
                (true, true) => vec![
                    Span::from(active_bar).fg(accent).bg(highlight),
                    Span::from(" ".repeat(left)).bg(highlight),
                    Span::from(name).fg(text).bg(highlight).bold(),
                    Span::from(suffix).fg(accent).bg(highlight),
                    Span::from(" ".repeat(right)).bg(highlight),
                ],
                (true, false) => vec![
                    Span::from(lead).fg(muted),
                    Span::from(" ".repeat(left)),
                    Span::from(name).fg(muted),
                    Span::from(suffix).fg(muted),
                    Span::from(" ".repeat(right)),
                ],
                // Original reversed look for the terminal-default theme.
                (false, true) => vec![
                    Span::from(lead).bg(background).reversed(),
                    Span::from(" ".repeat(left)).reversed(),
                    Span::from(name).bold().reversed(),
                    Span::from(suffix).reversed(),
                    Span::from(" ".repeat(right)).reversed(),
                ],
                (false, false) => vec![
                    Span::from(lead).fg(muted).reversed(),
                    Span::from(" ".repeat(left)).bg(muted),
                    Span::from(name).fg(text).reversed().fg(muted),
                    Span::from(suffix).fg(text).reversed().fg(muted),
                    Span::from(" ".repeat(right)).bg(muted),
                ],
            };
            tabs.push(tab);
        }

        let fit_count = (available / tab_width).clamp(1, tabs.len());
        let start = active_index.saturating_sub(fit_count / 2);
        let end = (start + fit_count).min(tabs.len());
        let start = end.saturating_sub(fit_count);

        let visible: Vec<Span> = tabs[start..end]
            .iter()
            .flat_map(|spans| spans.iter().cloned())
            .collect();
        Line::from(visible).render(tabs_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        app::SelectedNote, config::Symbols, note_editor::state::NoteEditorState, tabs::Tab,
    };

    fn tab(name: &str) -> Tab<'static> {
        tab_in("", name)
    }

    fn tab_in(dir: &str, name: &str) -> Tab<'static> {
        let path = PathBuf::from(format!("/vault/{dir}/{name}.md"));
        let editor = NoteEditorState::new("", name, &path, &Symbols::unicode());
        Tab {
            note: SelectedNote::new(name, &path, ""),
            editor,
        }
    }

    #[test]
    fn header_highlights_active_tab() {
        use ratatui::{
            backend::TestBackend,
            style::{Color, Modifier},
            Terminal,
        };

        let mut tabs = Tabs::default();
        tabs.open(tab("alpha"));
        tabs.open(tab("beta")); // active

        let symbols = Symbols::unicode();
        let mut terminal = Terminal::new(TestBackend::new(60, 1)).unwrap();
        terminal
            .draw(|frame| {
                Header::new(&symbols, &Theme::default(), &tabs)
                    .render(frame.area(), frame.buffer_mut())
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        type CellStyle = (Color, Color, Modifier);
        let cells: Vec<(String, CellStyle)> = (0..60)
            .map(|x| {
                let cell = buffer.cell((x, 0)).unwrap();
                (cell.symbol().to_string(), (cell.fg, cell.bg, cell.modifier))
            })
            .collect();
        let row: String = cells.iter().map(|(symbol, _)| symbol.as_str()).collect();
        assert!(row.contains("alpha") && row.contains("beta"), "got {row:?}");

        // The active tab renders in a visibly different style from the inactive
        // ones. Names are ASCII: one cell per char.
        let style_of = |needle: &str| {
            let start = cells
                .windows(needle.len())
                .position(|window| {
                    window
                        .iter()
                        .map(|(symbol, _)| symbol.as_str())
                        .collect::<String>()
                        == needle
                })
                .expect("tab name present");
            cells[start].1
        };
        assert_ne!(
            style_of("beta"),
            style_of("alpha"),
            "active tab must render distinctly from inactive tabs"
        );
    }

    #[test]
    fn header_scrolls_active_tab_into_view() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut tabs = Tabs::default();
        for index in 0..8 {
            tabs.open(tab(&format!("note{index}"))); // last opened stays active
        }

        let symbols = Symbols::unicode();
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).unwrap();
        terminal
            .draw(|frame| {
                Header::new(&symbols, &Theme::default(), &tabs)
                    .render(frame.area(), frame.buffer_mut())
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let row: String = (0..30)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol())
            .collect();
        assert!(
            row.contains("note7"),
            "active tab scrolled into view, got {row:?}"
        );
        assert!(
            !row.contains("note0"),
            "earlier tabs scrolled off, got {row:?}"
        );
    }

    #[test]
    fn header_keeps_middle_active_tab_whole() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut tabs = Tabs::default();
        for index in 0..8 {
            tabs.open(tab(&format!("note{index}")));
        }
        // Focus a middle tab so tabs overflow on both sides.
        tabs.next(); // wraps 7 -> 0
        for _ in 0..4 {
            tabs.next(); // 0 -> 4
        }

        let symbols = Symbols::unicode();
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).unwrap();
        terminal
            .draw(|frame| {
                Header::new(&symbols, &Theme::default(), &tabs)
                    .render(frame.area(), frame.buffer_mut())
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let row: String = (0..30)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol())
            .collect();
        assert!(
            row.contains("note4"),
            "active tab is shown whole, got {row:?}"
        );
    }

    #[test]
    fn tabs_render_at_uniform_width() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut tabs = Tabs::default();
        tabs.open(tab("x"));
        tabs.open(tab("a-much-longer-name"));
        tabs.open(tab("y"));

        let symbols = Symbols::unicode();
        let mut terminal = Terminal::new(TestBackend::new(120, 1)).unwrap();
        terminal
            .draw(|frame| {
                Header::new(&symbols, &Theme::default(), &tabs)
                    .render(frame.area(), frame.buffer_mut())
            })
            .unwrap();

        // Separators sit at the start of every tab but the first, so the gap
        // between them is exactly one tab width — the same for every tab.
        let buffer = terminal.backend().buffer();
        let separators: Vec<usize> = (0..120)
            .filter(|&x| buffer.cell((x, 0)).unwrap().symbol() == "▍")
            .map(|x| x as usize)
            .collect();
        assert_eq!(separators.len(), 2, "separators between the three tabs");
        assert_eq!(
            separators[1] - separators[0],
            MAX_TAB_WIDTH,
            "tabs are the same (max) width regardless of name length"
        );
    }

    #[test]
    fn themed_active_tab_uses_highlight_background() {
        use ratatui::{backend::TestBackend, style::Color, Terminal};

        // A theme that owns a background switches the tabs to the bufferline
        // style: the active tab paints its raised `code_bg` surface.
        let highlight = Color::Rgb(10, 20, 30);
        let theme = Theme {
            background: Color::Rgb(1, 2, 3),
            code_bg: highlight,
            accent: Color::Rgb(200, 100, 50),
            ..Theme::default()
        };

        let mut tabs = Tabs::default();
        tabs.open(tab("alpha"));
        tabs.open(tab("beta")); // active

        let symbols = Symbols::unicode();
        let mut terminal = Terminal::new(TestBackend::new(60, 1)).unwrap();
        terminal
            .draw(|frame| {
                Header::new(&symbols, &theme, &tabs).render(frame.area(), frame.buffer_mut())
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let active_on_highlight = (0..60).any(|x| {
            let cell = buffer.cell((x, 0)).unwrap();
            cell.symbol() == "b" && cell.bg == highlight
        });
        assert!(
            active_on_highlight,
            "themed active tab renders on the code_bg highlight"
        );
    }

    #[test]
    fn truncate_reserves_room_for_the_ellipsis() {
        assert_eq!(truncate("short", 10, "…"), "short");
        // The unicode ellipsis is one column; the ASCII fallback is three.
        assert_eq!(truncate("a-long-name", 6, "…"), "a-lon…");
        assert_eq!(truncate("a-long-name", 6, "..."), "a-l...");
    }
}
