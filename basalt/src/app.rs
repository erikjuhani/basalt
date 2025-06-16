use basalt_core::obsidian::{Note, Vault, VaultEntry};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect, Size},
    widgets::{StatefulWidget, StatefulWidgetRef},
    DefaultTerminal,
};

use std::{cell::RefCell, fmt::Debug, io::Result};

use crate::{
    config::{self, Config},
    explorer::{Explorer, ExplorerState},
    help_modal::{HelpModal, HelpModalState},
    markdown::{MarkdownView, MarkdownViewState},
    splash::{Splash, SplashState},
    statusbar::{StatusBar, StatusBarState},
    text_counts::{CharCount, WordCount},
    vault_selector_modal::{VaultSelectorModal, VaultSelectorModalState},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_TEXT: &str = include_str!("./help.txt");

#[derive(Debug, Default, Clone, PartialEq)]
pub enum ScrollAmount {
    #[default]
    One,
    HalfPage,
}

fn calc_scroll_amount(scroll_amount: ScrollAmount, height: usize) -> usize {
    match scroll_amount {
        ScrollAmount::One => 1,
        ScrollAmount::HalfPage => height / 2,
    }
}

#[derive(Default, Clone)]
struct MainState<'a> {
    active_pane: ActivePane,
    explorer: ExplorerState<'a>,
    note_viewer: MarkdownViewState,
    selected_note: Option<SelectedNote>,
}

impl<'a> MainState<'a> {
    fn new(selected_vault_name: &'a str, notes: Vec<VaultEntry>) -> Self {
        Self {
            active_pane: ActivePane::Explorer,
            explorer: ExplorerState::new(selected_vault_name, notes).set_active(true),
            ..Default::default()
        }
    }
}

#[derive(Default, Clone)]
pub struct AppState<'a> {
    _config: Config,
    screen: ScreenState<'a>,
    screen_size: Size,
    is_running: bool,

    help_modal: HelpModalState,
    vault_selector_modal: VaultSelectorModalState<'a>,
}

fn modal_area_height(size: Size) -> usize {
    let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
    let [area] = vertical.areas(Rect::new(0, 0, size.width, size.height.saturating_sub(3)));
    area.height.into()
}

#[derive(Clone)]
enum ScreenState<'a> {
    Splash(SplashState<'a>),
    Main(Box<MainState<'a>>),
}

impl<'a> AppState<'a> {
    pub fn active_component(&self) -> ActivePane {
        if self.help_modal.visible {
            return ActivePane::HelpModal;
        }

        if self.vault_selector_modal.visible {
            return ActivePane::VaultSelectorModal;
        }

        match &self.screen {
            ScreenState::Splash(..) => ActivePane::Splash,
            ScreenState::Main(state) => state.active_pane,
        }
    }

    pub fn set_running(&self, is_running: bool) -> Self {
        Self {
            is_running,
            ..self.clone()
        }
    }

    fn with_vault_selector_modal_state(
        &self,
        vault_selector_modal: VaultSelectorModalState<'a>,
    ) -> Self {
        Self {
            vault_selector_modal,
            ..self.clone()
        }
    }

    fn with_help_modal_state(&self, help_modal: HelpModalState) -> Self {
        Self {
            help_modal,
            ..self.clone()
        }
    }

    fn with_main_state(&self, main_state: MainState<'a>) -> Self {
        Self {
            screen: ScreenState::Main(Box::new(main_state)),
            ..self.clone()
        }
    }

    fn with_splash_state(&self, splash_state: SplashState<'a>) -> Self {
        Self {
            screen: ScreenState::Splash(splash_state),
            ..self.clone()
        }
    }
}

impl Default for ScreenState<'_> {
    fn default() -> Self {
        Self::Splash(SplashState::default())
    }
}

mod splash {
    use crossterm::event::{KeyCode, KeyEvent};

    use crate::splash::SplashState;

    #[derive(Clone)]
    pub enum Message {
        Up,
        Down,
        Open,
    }

    pub fn handle_event(key: &KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Up),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Down),
            KeyCode::Enter => Some(Message::Open),
            _ => None,
        }
    }

    pub fn update(message: Message, state: SplashState) -> SplashState {
        match message {
            Message::Up => state.previous(),
            Message::Down => state.next(),
            Message::Open => state.select(),
        }
    }
}

mod explorer {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::explorer::ExplorerState;

    use super::ScrollAmount;

    #[derive(Clone)]
    pub enum Message {
        Up,
        Down,
        Open,
        Sort,
        Toggle,
        SwitchPane,
        ScrollUp(ScrollAmount),
        ScrollDown(ScrollAmount),
    }

    pub fn handle_event(key: &KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Up),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Down),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Message::Open),
            KeyCode::Tab => Some(Message::SwitchPane),
            KeyCode::Char('t') => Some(Message::Toggle),
            KeyCode::Char('s') => Some(Message::Sort),
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::Sort)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ScrollUp(ScrollAmount::HalfPage))
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ScrollDown(ScrollAmount::HalfPage))
            }
            _ => None,
        }
    }

    pub fn update(message: Message, state: ExplorerState) -> ExplorerState {
        match message {
            Message::Up => state.previous(1),
            Message::Down => state.next(1),
            Message::Sort => state.sort(),
            Message::Open => state.select(),
            Message::Toggle => state.toggle(),
            Message::SwitchPane => {
                if state.active {
                    state.set_active(false)
                } else {
                    state.set_active(true)
                }
            }
            _ => state,
        }
    }
}

mod note_viewer {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::ScrollAmount;

    #[derive(Clone)]
    pub enum Message {
        SwitchPane,
        ToggleExplorer,
        ScrollUp(ScrollAmount),
        ScrollDown(ScrollAmount),
    }

    pub fn handle_event(key: &KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Message::ScrollUp(ScrollAmount::One)),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::ScrollDown(ScrollAmount::One)),
            KeyCode::Tab => Some(Message::SwitchPane),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ScrollUp(ScrollAmount::HalfPage))
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ScrollDown(ScrollAmount::HalfPage))
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ToggleExplorer)
            }
            KeyCode::Char('t') => Some(Message::ToggleExplorer),
            _ => None,
        }
    }
}

mod help_modal {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::help_modal::HelpModalState;

    use super::ScrollAmount;

    #[derive(Clone)]
    pub enum Message {
        Toggle,
        Close,
        ScrollUp(ScrollAmount),
        ScrollDown(ScrollAmount),
    }

    pub fn handle_event(key: &KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Esc => Some(Message::Close),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::ScrollUp(ScrollAmount::One)),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::ScrollDown(ScrollAmount::One)),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ScrollUp(ScrollAmount::HalfPage))
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::ScrollDown(ScrollAmount::HalfPage))
            }
            _ => None,
        }
    }

    pub fn update(message: Message, state: HelpModalState) -> HelpModalState {
        match message {
            Message::Toggle => state.toggle_visibility(),
            Message::Close => state.hide(),
            _ => state,
        }
    }
}

mod vault_selector_modal {
    use crossterm::event::{KeyCode, KeyEvent};

    use crate::vault_selector_modal::VaultSelectorModalState;

    #[derive(Clone)]
    pub enum Message {
        Toggle,
        Up,
        Down,
        Select,
        Close,
    }

    pub fn handle_event(key: &KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Esc => Some(Message::Close),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::Up),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::Down),
            KeyCode::Enter => Some(Message::Select),
            _ => None,
        }
    }

    pub fn update(message: Message, state: VaultSelectorModalState) -> VaultSelectorModalState {
        match message {
            Message::Up => state.previous(),
            Message::Down => state.next(),
            Message::Toggle => state.toggle_visibility(),
            Message::Select => state.select(),
            Message::Close => state.hide(),
        }
    }
}

#[derive(Clone)]
pub enum Message {
    Quit,
    Resize(Size),

    Splash(splash::Message),
    Explorer(explorer::Message),
    NoteViewer(note_viewer::Message),
    HelpModal(help_modal::Message),
    VaultSelectorModal(vault_selector_modal::Message),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ActivePane {
    #[default]
    Splash,
    Explorer,
    NoteViewer,
    HelpModal,
    VaultSelectorModal,
}

impl From<ActivePane> for &str {
    fn from(value: ActivePane) -> Self {
        match value {
            ActivePane::Splash => "Splash",
            ActivePane::Explorer => "Explorer",
            ActivePane::NoteViewer => "Note Viewer",
            ActivePane::HelpModal => "Help",
            ActivePane::VaultSelectorModal => "Vault Selector",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SelectedNote {
    name: String,
    path: String,
    content: String,
}

impl From<Note> for SelectedNote {
    fn from(value: Note) -> Self {
        Self {
            name: value.name.clone(),
            path: value.path.to_string_lossy().to_string(),
            content: Note::read_to_string(&value).unwrap_or_default(),
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

pub struct App<'a> {
    state: AppState<'a>,
    terminal: RefCell<DefaultTerminal>,
}

impl<'a> App<'a> {
    pub fn new(state: AppState<'a>, terminal: DefaultTerminal) -> Self {
        Self {
            state,
            terminal: RefCell::new(terminal),
        }
    }

    pub fn start(terminal: DefaultTerminal, vaults: Vec<&Vault>) -> Result<()> {
        let version = format!("{VERSION}~alpha");
        let size = terminal.size()?;

        let state = AppState {
            // Surface toast if read config returns error
            _config: config::load().unwrap(),
            screen_size: size,
            help_modal: HelpModalState::new(&help_text()),
            vault_selector_modal: VaultSelectorModalState::new(vaults.clone()),
            ..Default::default()
        }
        .with_splash_state(SplashState::new(&version, vaults));

        App::new(state, terminal).run()
    }

    fn run(&'a mut self) -> Result<()> {
        self.state.is_running = true;

        while self.state.is_running {
            self.draw(&self.state)?;
            let event = event::read()?;
            let action = self.handle_event(&event);
            self.state = self.update(&self.state, action);
        }

        Ok(())
    }

    fn draw(&self, state: &'a AppState<'a>) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();
        let mut state = state.clone();

        terminal.draw(move |frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            self.render_ref(area, buf, &mut state);
        })?;

        Ok(())
    }

    fn handle_event(&self, event: &Event) -> Option<Message> {
        match event {
            Event::Resize(cols, rows) => Some(Message::Resize(Size::new(*cols, *rows))),
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => None,
        }
    }

    #[rustfmt::skip]
    fn handle_active_component_event(&self, key: &KeyEvent, active_component: ActivePane) -> Option<Message> {
        match active_component {
            ActivePane::Splash => splash::handle_event(key).map(Message::Splash),
            ActivePane::Explorer => explorer::handle_event(key).map(Message::Explorer),
            ActivePane::NoteViewer => note_viewer::handle_event(key).map(Message::NoteViewer),
            ActivePane::HelpModal => help_modal::handle_event(key).map(Message::HelpModal),
            ActivePane::VaultSelectorModal => vault_selector_modal::handle_event(key).map(Message::VaultSelectorModal),
        }
    }

    fn handle_key_event(&self, key: &KeyEvent) -> Option<Message> {
        let global_action = match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) => Some(Message::Quit),
            (KeyCode::Char('?'), _) => Some(Message::HelpModal(help_modal::Message::Toggle)),
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => Some(Message::VaultSelectorModal(
                vault_selector_modal::Message::Toggle,
            )),
            _ => None,
        };

        if global_action.is_some() {
            return global_action;
        }

        let active_component = self.state.active_component();
        self.handle_active_component_event(key, active_component)
    }

    fn update(&self, state: &AppState<'a>, message: Option<Message>) -> AppState<'a> {
        let state = state.clone();
        let Some(message) = message else {
            return state;
        };

        let screen = state.screen.clone();

        match message {
            Message::Quit => state.set_running(false),
            Message::Resize(size) => AppState {
                screen_size: size,
                ..state
            },
            Message::HelpModal(message) => {
                let help_modal = help_modal::update(message.clone(), state.help_modal.clone());

                match message {
                    help_modal::Message::ScrollDown(scroll_amount) => {
                        state.with_help_modal_state(help_modal.scroll_down(calc_scroll_amount(
                            scroll_amount,
                            modal_area_height(state.screen_size),
                        )))
                    }
                    help_modal::Message::ScrollUp(scroll_amount) => {
                        state.with_help_modal_state(help_modal.scroll_up(calc_scroll_amount(
                            scroll_amount,
                            modal_area_height(state.screen_size),
                        )))
                    }
                    _ => state.with_help_modal_state(help_modal),
                }
            }
            Message::VaultSelectorModal(message) => {
                let ScreenState::Main(_) = screen else {
                    return state;
                };

                let vault_selector_modal = vault_selector_modal::update(
                    message.clone(),
                    state.vault_selector_modal.clone(),
                );

                match message {
                    vault_selector_modal::Message::Select => vault_selector_modal
                        .selected()
                        .and_then(|index| vault_selector_modal.clone().get_item(index))
                        .map(|vault| {
                            state
                                .with_main_state(MainState::new(&vault.name, vault.entries()))
                                .with_vault_selector_modal_state(vault_selector_modal.hide())
                        })
                        .unwrap_or(state),
                    _ => state.with_vault_selector_modal_state(vault_selector_modal),
                }
            }
            Message::Splash(message) => {
                let ScreenState::Splash(splash_state) = screen else {
                    return state;
                };

                let splash_state = splash::update(message.clone(), splash_state);

                match message {
                    splash::Message::Open => splash_state
                        .selected()
                        .and_then(|index| splash_state.clone().get_item(index))
                        .map(|vault| {
                            state.with_main_state(MainState::new(&vault.name, vault.entries()))
                        })
                        .unwrap_or(state),
                    _ => state.with_splash_state(splash_state),
                }
            }
            Message::Explorer(message) => {
                let ScreenState::Main(main_state) = screen else {
                    return state;
                };

                let explorer = explorer::update(message.clone(), main_state.explorer.clone());

                match message {
                    explorer::Message::SwitchPane => state.with_main_state(MainState {
                        active_pane: ActivePane::NoteViewer,
                        note_viewer: main_state.note_viewer.set_active(true),
                        explorer,
                        ..*main_state
                    }),
                    explorer::Message::ScrollUp(scroll_amount) => {
                        state.with_main_state(MainState {
                            explorer: explorer.previous(calc_scroll_amount(
                                scroll_amount,
                                state.screen_size.height.into(),
                            )),
                            ..*main_state
                        })
                    }
                    explorer::Message::ScrollDown(scroll_amount) => {
                        state.with_main_state(MainState {
                            explorer: explorer.next(calc_scroll_amount(
                                scroll_amount,
                                state.screen_size.height.into(),
                            )),
                            ..*main_state
                        })
                    }
                    explorer::Message::Toggle => state.with_main_state(match explorer.open {
                        true => MainState {
                            explorer,
                            ..*main_state
                        },
                        false => MainState {
                            active_pane: ActivePane::NoteViewer,
                            explorer: explorer.set_active(false),
                            note_viewer: main_state.note_viewer.set_active(true),
                            ..*main_state
                        },
                    }),
                    explorer::Message::Open => {
                        let note = explorer.selected_note.clone();
                        let selected_note = note.map(SelectedNote::from);

                        let note_viewer = main_state
                            .note_viewer
                            .clone()
                            .set_text(
                                selected_note
                                    .clone()
                                    .map(|note| note.content.clone())
                                    .unwrap_or_default(),
                            )
                            .reset_scrollbar();

                        state.with_main_state(MainState {
                            explorer,
                            note_viewer,
                            selected_note,
                            ..*main_state
                        })
                    }
                    _ => state.with_main_state(MainState {
                        explorer,
                        ..*main_state
                    }),
                }
            }
            Message::NoteViewer(message) => {
                let ScreenState::Main(main_state) = screen else {
                    return state;
                };

                match message {
                    note_viewer::Message::ScrollUp(scroll_amount) => {
                        state.with_main_state(MainState {
                            note_viewer: main_state.note_viewer.scroll_up(calc_scroll_amount(
                                scroll_amount,
                                state.screen_size.height.into(),
                            )),
                            ..*main_state
                        })
                    }
                    note_viewer::Message::ScrollDown(scroll_amount) => {
                        state.with_main_state(MainState {
                            note_viewer: main_state.note_viewer.scroll_down(calc_scroll_amount(
                                scroll_amount,
                                state.screen_size.height.into(),
                            )),
                            ..*main_state
                        })
                    }
                    note_viewer::Message::SwitchPane => state.with_main_state(MainState {
                        active_pane: ActivePane::Explorer,
                        note_viewer: main_state.note_viewer.set_active(false),
                        explorer: main_state.explorer.set_active(true),
                        ..*main_state
                    }),
                    note_viewer::Message::ToggleExplorer => {
                        state.with_main_state(match main_state.explorer.open {
                            true => MainState {
                                explorer: main_state.explorer.toggle(),
                                ..*main_state
                            },
                            false => MainState {
                                active_pane: ActivePane::Explorer,
                                explorer: main_state.explorer.toggle().set_active(true),
                                note_viewer: main_state.note_viewer.set_active(false),
                                ..*main_state
                            },
                        })
                    }
                }
            }
        }
    }

    fn render_splash(&self, area: Rect, buf: &mut Buffer, state: &mut SplashState<'a>) {
        Splash::default().render_ref(area, buf, state)
    }

    fn render_main(&self, area: Rect, buf: &mut Buffer, state: &mut MainState<'a>) {
        let [content, statusbar] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
            .horizontal_margin(1)
            .areas(area);

        let (left, right) = if state.explorer.open {
            (Constraint::Length(35), Constraint::Fill(1))
        } else {
            (Constraint::Length(5), Constraint::Fill(1))
        };

        let [explorer_pane, note] = Layout::horizontal([left, right]).areas(content);

        Explorer::new().render(explorer_pane, buf, &mut state.explorer);
        MarkdownView.render_ref(note, buf, &mut state.note_viewer);

        let (_, counts) = state
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
            state.active_pane.into(),
            word_count.into(),
            char_count.into(),
        );

        let status_bar = StatusBar::default();
        status_bar.render_ref(statusbar, buf, &mut status_bar_state);
    }

    fn render_modals(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        if state.vault_selector_modal.visible {
            VaultSelectorModal::default().render(area, buf, &mut state.vault_selector_modal);
        }

        if state.help_modal.visible {
            HelpModal.render(area, buf, &mut state.help_modal);
        }
    }
}

impl<'a> StatefulWidgetRef for App<'a> {
    type State = AppState<'a>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        match &mut state.screen {
            ScreenState::Splash(state) => self.render_splash(area, buf, state),
            ScreenState::Main(state) => self.render_main(area, buf, state),
        };

        self.render_modals(area, buf, state)
    }
}
