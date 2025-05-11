use ratatui::widgets::ScrollbarState;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Scrollbar {
    pub state: ScrollbarState,
    pub position: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MarkdownViewState {
    pub(crate) text: String,
    pub(crate) scrollbar: Scrollbar,
}

impl MarkdownViewState {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn get_lines(&self) -> Vec<&str> {
        self.text.lines().collect()
    }

    pub fn scroll_up(self, amount: usize) -> Self {
        let new_position = self.scrollbar.position.saturating_sub(amount);
        let new_state = self.scrollbar.state.position(new_position);

        Self {
            scrollbar: Scrollbar {
                state: new_state,
                position: new_position,
            },
            ..self
        }
    }

    pub fn scroll_down(self, amount: usize) -> Self {
        let new_position = self.scrollbar.position.saturating_add(amount);
        let new_state = self.scrollbar.state.position(new_position);

        Self {
            scrollbar: Scrollbar {
                state: new_state,
                position: new_position,
            },
            ..self
        }
    }

    pub fn set_text(self, text: String) -> Self {
        Self { text, ..self }
    }

    pub fn reset_scrollbar(self) -> Self {
        Self {
            scrollbar: Scrollbar::default(),
            ..self
        }
    }
}
