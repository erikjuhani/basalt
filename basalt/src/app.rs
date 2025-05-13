use super::markdown::{MarkdownView, MarkdownViewState};
use basalt_core::obsidian::{Note, Vault};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect, Size},
    widgets::{StatefulWidget, StatefulWidgetRef},
    DefaultTerminal,
};

use std::{cell::RefCell, io::Result, marker::PhantomData};

use crate::{
    help_modal::{HelpModal, HelpModalState},
    sidepanel::{SidePanel, SidePanelState},
    start::{StartScreen, StartState},
    statusbar::{StatusBar, StatusBarState},
    text_counts::{CharCount, WordCount},
    vault_selector_modal::{VaultSelectorModal, VaultSelectorModalState},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_TEXT: &str = include_str!("./help.txt");

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Mode {
    #[default]
    Select,
    Normal,
    Insert,
}

impl Mode {
    fn as_str(&self) -> &'static str {
        match self {
            Mode::Select => "Select",
            Mode::Normal => "Normal",
            Mode::Insert => "Insert",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum ScrollAmount {
    #[default]
    One,
    HalfPage,
}

fn calc_scroll_amount(scroll_amount: ScrollAmount, size: Size) -> usize {
    match scroll_amount {
        ScrollAmount::One => 1,
        ScrollAmount::HalfPage => (size.height / 3).into(),
    }
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Select,
    Next,
    Prev,
    Insert,
    Resize(Size),
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
    ToggleMode,
    ToggleHelp,
    ToggleVaultSelector,
    Quit,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Start<'a> {
    pub start_state: StartState<'a>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Main<'a> {
    pub sidepanel_state: SidePanelState<'a>,
    pub selected_note: Option<SelectedNote>,
    pub markdown_view_state: MarkdownViewState,
    pub notes: Vec<Note>,
    pub vaults: Vec<&'a Vault>,
    pub size: Size,
    pub mode: Mode,
}

impl<'a> Main<'a> {
    fn new(vault_name: &'a str, notes: Vec<Note>, size: Size, vaults: Vec<&'a Vault>) -> Self {
        Self {
            notes: notes.clone(),
            sidepanel_state: SidePanelState::new(vault_name, notes),
            vaults,
            size,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen<'a> {
    Start(Start<'a>),
    Main(Main<'a>),
}

impl Default for Screen<'_> {
    fn default() -> Self {
        Screen::Start(Start::default())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AppState<'a> {
    pub help_modal: Option<HelpModalState>,
    pub vault_selector_modal: Option<VaultSelectorModalState<'a>>,
    pub size: Size,
    pub is_running: bool,
    pub screen: Screen<'a>,
    _lifetime: PhantomData<&'a ()>,
}

pub struct App<'a> {
    pub state: AppState<'a>,
    terminal: RefCell<DefaultTerminal>,
}

impl<'a> StatefulWidgetRef for App<'a> {
    type State = AppState<'a>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let screen = state.screen.clone();

        match screen {
            Screen::Start(mut state) => {
                StartScreen::default().render_ref(area, buf, &mut state.start_state)
            }
            Screen::Main(mut state) => {
                let [content, statusbar] =
                    Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
                        .horizontal_margin(1)
                        .areas(area);

                let (left, right) = if state.mode == Mode::Select {
                    (Constraint::Length(35), Constraint::Fill(1))
                } else {
                    (Constraint::Length(5), Constraint::Fill(1))
                };

                let [sidepanel, note] = Layout::horizontal([left, right]).areas(content);

                SidePanel::default().render_ref(sidepanel, buf, &mut state.sidepanel_state);

                MarkdownView.render_ref(note, buf, &mut state.markdown_view_state);

                let mode = state.mode.as_str().to_uppercase();
                let (name, counts) = state
                    .selected_note
                    .clone()
                    .map(|note| {
                        let content = note.content.as_str();
                        (
                            note.name,
                            (WordCount::from(content), CharCount::from(content)),
                        )
                    })
                    .unzip();

                let (word_count, char_count) = counts.unwrap_or_default();

                let mut status_bar_state = StatusBarState::new(
                    &mode,
                    name.as_deref(),
                    word_count.into(),
                    char_count.into(),
                );

                StatusBar::default().render_ref(statusbar, buf, &mut status_bar_state);
            }
        }

        if let Some(mut vault_selector_modal_state) = state.vault_selector_modal.clone() {
            VaultSelectorModal::default().render(area, buf, &mut vault_selector_modal_state)
        }

        if let Some(mut help_modal_state) = state.help_modal.clone() {
            HelpModal.render(area, buf, &mut help_modal_state)
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SelectedNote {
    name: String,
    path: String,
    content: String,
}

impl From<&Note> for SelectedNote {
    fn from(value: &Note) -> Self {
        Self {
            name: value.name.clone(),
            path: value.path.to_string_lossy().to_string(),
            content: Note::read_to_string(value).unwrap(),
        }
    }
}

fn help_text() -> String {
    let version = format!("{VERSION}~alpha");
    HELP_TEXT.replace(
        "%version-notice",
        format!("This is the read-only release of Basalt ({version})").as_str(),
    )
}

impl<'a> App<'a> {
    pub fn start(terminal: DefaultTerminal, vaults: Vec<&Vault>) -> Result<()> {
        let version = format!("{VERSION}~alpha");
        let size = terminal.size()?;

        let state = AppState {
            screen: Screen::Start(Start {
                start_state: StartState::new(&version, size, vaults),
            }),
            size,
            is_running: true,
            _lifetime: PhantomData,
            ..Default::default()
        };

        App {
            state: state.clone(),
            terminal: RefCell::new(terminal),
        }
        .run(state)
    }

    fn run(&mut self, state: AppState<'a>) -> Result<()> {
        self.draw(&state)?;
        let event = &event::read()?;
        match self.update(&state, self.handle_event(event)) {
            state if state.is_running => self.run(state),
            _ => Ok(()),
        }
    }

    fn update_help_modal(
        &self,
        state: AppState<'a>,
        inner: HelpModalState,
        action: Action,
    ) -> AppState<'a> {
        match action {
            Action::ScrollUp(amount) => AppState {
                help_modal: Some(inner.scroll_up(calc_scroll_amount(amount, state.size))),
                ..state
            },
            Action::ScrollDown(amount) => AppState {
                help_modal: Some(inner.scroll_down(calc_scroll_amount(amount, state.size))),
                ..state
            },
            Action::Next => AppState {
                help_modal: Some(inner.scroll_down(1)),
                ..state
            },
            Action::Prev => AppState {
                help_modal: Some(inner.scroll_up(1)),
                ..state
            },
            _ => state,
        }
    }

    fn update_vault_selector_modal(
        &self,
        state: AppState<'a>,
        inner: VaultSelectorModalState<'a>,
        action: Action,
    ) -> AppState<'a> {
        match action {
            Action::ToggleVaultSelector => AppState {
                vault_selector_modal: None,
                ..state
            },
            Action::Select => {
                // TODO: Add logic to not load the vault again if the same vault was picked in the
                // selector.
                let alphabetically =
                    |a: &Note, b: &Note| a.name.to_lowercase().cmp(&b.name.to_lowercase());

                let vault_selector_state = inner.vault_selector_state.select();

                let vault_with_notes = vault_selector_state
                    .selected()
                    .and_then(|index| inner.vault_selector_state.get_item(index))
                    .map(|vault| (vault, vault.notes_sorted_by(alphabetically)));

                if let Some((vault, notes)) = vault_with_notes {
                    AppState {
                        screen: Screen::Main(Main::new(
                            &vault.name,
                            notes,
                            state.size,
                            vault_selector_state.items(),
                        )),
                        vault_selector_modal: None,
                        ..state
                    }
                } else {
                    state
                }
            }
            Action::Next => AppState {
                vault_selector_modal: Some(VaultSelectorModalState {
                    vault_selector_state: inner.vault_selector_state.next(),
                }),
                ..state
            },
            Action::Prev => AppState {
                vault_selector_modal: Some(VaultSelectorModalState {
                    vault_selector_state: inner.vault_selector_state.previous(),
                }),
                ..state
            },
            _ => state,
        }
    }

    fn update_select_mode(
        &self,
        state: AppState<'a>,
        inner: Main<'a>,
        action: Action,
    ) -> AppState<'a> {
        match action {
            Action::ToggleMode => AppState {
                screen: Screen::Main(Main {
                    mode: Mode::Normal,
                    sidepanel_state: inner.sidepanel_state.close(),
                    ..inner
                }),
                ..state
            },
            Action::ScrollUp(amount) => AppState {
                screen: Screen::Main(Main {
                    markdown_view_state: inner
                        .markdown_view_state
                        .scroll_up(calc_scroll_amount(amount, state.size)),
                    ..inner
                }),
                ..state
            },
            Action::ScrollDown(amount) => AppState {
                screen: Screen::Main(Main {
                    markdown_view_state: inner
                        .markdown_view_state
                        .scroll_down(calc_scroll_amount(amount, state.size)),
                    ..inner
                }),
                ..state
            },
            Action::Select => {
                let sidepanel_state = inner.sidepanel_state.select();

                let selected_note = inner
                    .notes
                    .get(sidepanel_state.selected().unwrap_or_default())
                    .map(SelectedNote::from);

                AppState {
                    screen: Screen::Main(Main {
                        sidepanel_state,
                        selected_note: selected_note.clone(),
                        markdown_view_state: inner
                            .markdown_view_state
                            .set_text(selected_note.map(|note| note.content).unwrap_or_default())
                            .reset_scrollbar(),
                        ..inner
                    }),
                    ..state
                }
            }
            Action::Next => AppState {
                screen: Screen::Main(Main {
                    sidepanel_state: inner.sidepanel_state.next(),
                    ..inner
                }),
                ..state
            },
            Action::Prev => AppState {
                screen: Screen::Main(Main {
                    sidepanel_state: inner.sidepanel_state.previous(),
                    ..inner
                }),
                ..state
            },
            _ => state,
        }
    }

    fn update_normal_mode(
        &self,
        state: AppState<'a>,
        inner: Main<'a>,
        action: Action,
    ) -> AppState<'a> {
        match action {
            Action::ToggleMode => AppState {
                screen: Screen::Main(Main {
                    mode: Mode::Select,
                    sidepanel_state: inner.sidepanel_state.open(),
                    ..inner
                }),
                ..state
            },
            Action::ScrollUp(amount) => AppState {
                screen: Screen::Main(Main {
                    markdown_view_state: inner
                        .markdown_view_state
                        .scroll_up(calc_scroll_amount(amount, state.size)),
                    ..inner
                }),
                ..state
            },
            Action::ScrollDown(amount) => AppState {
                screen: Screen::Main(Main {
                    markdown_view_state: inner
                        .markdown_view_state
                        .scroll_down(calc_scroll_amount(amount, state.size)),
                    ..inner
                }),
                ..state
            },
            Action::Next => AppState {
                screen: Screen::Main(Main {
                    markdown_view_state: inner.markdown_view_state.scroll_down(1),
                    ..inner
                }),
                ..state
            },
            Action::Prev => AppState {
                screen: Screen::Main(Main {
                    markdown_view_state: inner.markdown_view_state.scroll_up(1),
                    ..inner
                }),
                ..state
            },
            _ => state,
        }
    }

    fn update_main_state(
        &self,
        state: AppState<'a>,
        inner: Main<'a>,
        action: Action,
    ) -> AppState<'a> {
        if let Action::ToggleVaultSelector = action {
            return AppState {
                vault_selector_modal: if state.vault_selector_modal.is_some() {
                    None
                } else {
                    Some(VaultSelectorModalState::new(inner.vaults.clone()))
                },
                ..state
            };
        }

        match inner.mode {
            Mode::Select => self.update_select_mode(state, inner, action),
            Mode::Normal => self.update_normal_mode(state, inner, action),
            Mode::Insert => state,
        }
    }

    fn update_start_state(
        &self,
        state: AppState<'a>,
        inner: Start<'a>,
        action: Action,
    ) -> AppState<'a> {
        match action {
            Action::Select => {
                let alphabetically =
                    |a: &Note, b: &Note| a.name.to_lowercase().cmp(&b.name.to_lowercase());

                let splash_state = inner.start_state.select();

                let vault_with_notes = splash_state
                    .selected()
                    .and_then(|index| splash_state.get_item(index))
                    .map(|vault| (vault, vault.notes_sorted_by(alphabetically)));

                if let Some((vault, notes)) = vault_with_notes {
                    AppState {
                        screen: Screen::Main(Main::new(
                            &vault.name,
                            notes,
                            state.size,
                            inner.start_state.items(),
                        )),
                        ..state
                    }
                } else {
                    state
                }
            }
            Action::Next => AppState {
                screen: Screen::Start(Start {
                    start_state: inner.start_state.next(),
                }),
                ..state
            },
            Action::Prev => AppState {
                screen: Screen::Start(Start {
                    start_state: inner.start_state.previous(),
                }),
                ..state
            },
            _ => state,
        }
    }

    fn update(&self, state: &AppState<'a>, action: Option<Action>) -> AppState<'a> {
        let state = state.clone();
        let screen = state.screen.clone();

        let Some(action) = action else {
            return state;
        };

        match action {
            Action::Quit => AppState {
                is_running: false,
                ..state
            },
            Action::ToggleHelp => AppState {
                help_modal: if state.help_modal.is_some() {
                    None
                } else {
                    Some(HelpModalState::new(&help_text()))
                },
                ..state
            },
            Action::Resize(size) => AppState { size, ..state },
            _ if state.help_modal.is_some() => {
                self.update_help_modal(state.clone(), state.help_modal.unwrap().clone(), action)
            }
            _ if state.vault_selector_modal.is_some() => self.update_vault_selector_modal(
                state.clone(),
                state.vault_selector_modal.unwrap().clone(),
                action,
            ),
            _ => match screen {
                Screen::Start(inner) => self.update_start_state(state, inner, action),
                Screen::Main(inner) => self.update_main_state(state, inner, action),
            },
        }
    }

    fn handle_event(&self, event: &Event) -> Option<Action> {
        match event {
            Event::Resize(cols, rows) => Some(Action::Resize(Size::new(*cols, *rows))),
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_press_key_event(key_event)
            }
            _ => None,
        }
    }

    fn handle_press_key_event(&self, key_event: &KeyEvent) -> Option<Action> {
        match key_event.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::ToggleHelp),
            KeyCode::Char(' ') => Some(Action::ToggleVaultSelector),
            KeyCode::Up => Some(Action::ScrollUp(ScrollAmount::One)),
            KeyCode::Down => Some(Action::ScrollDown(ScrollAmount::One)),
            KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::ScrollUp(ScrollAmount::HalfPage))
            }
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::Quit)
            }
            KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::ScrollDown(ScrollAmount::HalfPage))
            }
            KeyCode::Char('t') => Some(Action::ToggleMode),
            KeyCode::Char('k') => Some(Action::Prev),
            KeyCode::Char('j') => Some(Action::Next),
            KeyCode::Enter => Some(Action::Select),
            _ => None,
        }
    }

    fn draw(&self, state: &AppState<'a>) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();
        let mut state = state.clone();

        terminal.draw(move |frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            self.render_ref(area, buf, &mut state);
        })?;

        Ok(())
    }
}
