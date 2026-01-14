use basalt_core::obsidian::{Note, Vault};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect, Size},
    widgets::{StatefulWidget, StatefulWidgetRef},
    DefaultTerminal,
};

use std::{cell::RefCell, fmt::Debug, fs, io::Result, path::PathBuf};

use crate::{
    command,
    config::{self, Config},
    explorer::{self, Explorer, ExplorerState, Item, Visibility},
    help_modal::{self, HelpModal, HelpModalState},
    input::{self, Input, InputModalState},
    note_editor::{
        self, ast,
        editor::NoteEditor,
        state::{NoteEditorState, View},
    },
    outline::{self, Outline, OutlineState},
    splash_modal::{self, SplashModal, SplashModalState},
    statusbar::{StatusBar, StatusBarState},
    stylized_text::{self, FontStyle},
    text_counts::{CharCount, WordCount},
    vault_selector_modal::{self, VaultSelectorModal, VaultSelectorModalState},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_TEXT: &str = include_str!("./help.txt");

#[derive(Debug, Default, Clone, PartialEq)]
pub enum ScrollAmount {
    #[default]
    One,
    HalfPage,
}

pub fn calc_scroll_amount(scroll_amount: &ScrollAmount, height: usize) -> usize {
    match scroll_amount {
        ScrollAmount::One => 1,
        ScrollAmount::HalfPage => height / 2,
    }
}

#[derive(Default, Clone)]
pub struct AppState<'a> {
    vault: Vault,
    screen_size: Size,
    is_running: bool,

    active_pane: ActivePane,
    explorer: ExplorerState,
    note_editor: NoteEditorState<'a>,
    outline: OutlineState,
    selected_note: Option<SelectedNote>,

    input_modal: InputModalState,
    splash_modal: SplashModalState<'a>,
    help_modal: HelpModalState,
    vault_selector_modal: VaultSelectorModalState<'a>,
}

impl<'a> AppState<'a> {
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub fn active_component(&self) -> ActivePane {
        if self.help_modal.visible {
            return ActivePane::HelpModal;
        }

        if self.vault_selector_modal.visible {
            return ActivePane::VaultSelectorModal;
        }

        if self.splash_modal.visible {
            return ActivePane::Splash;
        }

        self.active_pane
    }

    pub fn set_running(&self, is_running: bool) -> Self {
        Self {
            is_running,
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message<'a> {
    Quit,
    Exec(String),
    Spawn(String),
    Resize(Size),
    SetActivePane(ActivePane),
    /// (original_path, new_path) TODO: Use tuple struct instead to be explicit
    RefreshVault(Option<(PathBuf, PathBuf)>),
    RefreshSelectedNote,
    OpenVault(&'a Vault),
    SelectNote(SelectedNote),
    UpdateSelectedNoteContent((String, Option<Vec<ast::Node>>)),

    Input(input::Message),
    Splash(splash_modal::Message),
    Explorer(explorer::Message),
    NoteEditor(note_editor::Message),
    Outline(outline::Message),
    HelpModal(help_modal::Message),
    VaultSelectorModal(vault_selector_modal::Message),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ActivePane {
    #[default]
    Splash,
    Explorer,
    NoteEditor,
    Outline,
    Input,
    HelpModal,
    VaultSelectorModal,
}

impl From<ActivePane> for &str {
    fn from(value: ActivePane) -> Self {
        match value {
            ActivePane::Splash => "Splash",
            ActivePane::Explorer => "Explorer",
            ActivePane::NoteEditor => "Note Editor",
            ActivePane::Outline => "Outline",
            ActivePane::Input => "Input",
            ActivePane::HelpModal => "Help",
            ActivePane::VaultSelectorModal => "Vault Selector",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SelectedNote {
    name: String,
    path: PathBuf,
    content: String,
}

impl From<&Note> for SelectedNote {
    fn from(value: &Note) -> Self {
        Self {
            name: value.name().to_string(),
            path: value.path().to_path_buf(),
            content: fs::read_to_string(value.path()).unwrap_or_default(),
        }
    }
}

fn help_text(version: &str) -> String {
    HELP_TEXT.replace("%version-notice", version)
}

pub struct App<'a> {
    state: AppState<'a>,
    config: Config<'a>,
    terminal: RefCell<DefaultTerminal>,
}

impl<'a> App<'a> {
    pub fn new(state: AppState<'a>, terminal: DefaultTerminal) -> Self {
        Self {
            state,
            // TODO: Surface toast if read config returns error
            config: config::load().unwrap(),
            terminal: RefCell::new(terminal),
        }
    }

    pub fn start(terminal: DefaultTerminal, vaults: Vec<&Vault>) -> Result<()> {
        let version = stylized_text::stylize(&format!("{VERSION}~beta"), FontStyle::Script);
        let size = terminal.size()?;

        let state = AppState {
            screen_size: size,
            help_modal: HelpModalState::new(&help_text(&version)),
            vault_selector_modal: VaultSelectorModalState::new(vaults.clone()),
            splash_modal: SplashModalState::new(&version, vaults, true),
            ..Default::default()
        };

        App::new(state, terminal).run()
    }

    fn run(&'a mut self) -> Result<()> {
        self.state.is_running = true;

        let mut state = self.state.clone();
        let config = self.config.clone();
        while state.is_running {
            self.draw(&mut state)?;
            let event = event::read()?;

            let mut message = App::handle_event(&config, &state, &event);
            while message.is_some() {
                message = App::update(self.terminal.get_mut(), &config, &mut state, message);
            }
        }

        Ok(())
    }

    fn draw(&self, state: &mut AppState<'a>) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();

        terminal.draw(move |frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            self.render_ref(area, buf, state);
        })?;

        Ok(())
    }

    fn handle_event(
        config: &'a Config,
        state: &AppState<'_>,
        event: &Event,
    ) -> Option<Message<'a>> {
        match event {
            Event::Resize(cols, rows) => Some(Message::Resize(Size::new(*cols, *rows))),
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                App::handle_key_event(config, state, key_event)
            }
            _ => None,
        }
    }

    #[rustfmt::skip]
    fn handle_active_component_event(config: &'a Config, state: &AppState<'_>, key: &KeyEvent, active_component: ActivePane) -> Option<Message<'a>> {
        match active_component {
            ActivePane::Splash => config.splash.key_to_message(key.into()),
            ActivePane::Explorer => config.explorer.key_to_message(key.into()),
            ActivePane::Outline => config.outline.key_to_message(key.into()),
            ActivePane::HelpModal => config.help_modal.key_to_message(key.into()),
            ActivePane::VaultSelectorModal => config.vault_selector_modal.key_to_message(key.into()),
            ActivePane::Input => {
                if state.input_modal.is_editing() {
                    input::handle_editing_event(key).map(Message::Input)
                } else {
                    config.input_modal.key_to_message(key.into())
                }
            },
            ActivePane::NoteEditor => {
                    if state.note_editor.is_editing() {
                        note_editor::handle_editing_event(key).map(Message::NoteEditor)
                    } else {
                        config.note_editor.key_to_message(key.into())
                    }
            }
        }
    }

    fn handle_key_event(
        config: &'a Config,
        state: &AppState<'_>,
        key: &KeyEvent,
    ) -> Option<Message<'a>> {
        let global_message = config.global.key_to_message(key.into());

        let is_editing = state.note_editor.is_editing() || state.input_modal.is_editing();

        if global_message.is_some() && !is_editing {
            return global_message;
        }

        let active_component = state.active_component();
        App::handle_active_component_event(config, state, key, active_component)
    }

    fn update(
        terminal: &mut DefaultTerminal,
        config: &Config,
        state: &mut AppState<'a>,
        message: Option<Message<'a>>,
    ) -> Option<Message<'a>> {
        match message? {
            Message::Quit => state.is_running = false,
            Message::Resize(size) => state.screen_size = size,
            Message::RefreshVault(rename) => {
                state.explorer.with_entries(state.vault.entries(), rename);
                return Some(Message::RefreshSelectedNote);
            }
            Message::RefreshSelectedNote => {
                if state
                    .explorer
                    .list_state
                    .selected()
                    .zip(state.explorer.selected_item_index)
                    .is_some_and(|(a, b)| a == b)
                {
                    if let Some(Item::File(note)) = state.explorer.current_item() {
                        state.note_editor.set_filepath(note.path());
                        state.note_editor.set_filename(note.name());
                        state.note_editor.update_layout();
                        state.explorer.select();
                    }
                }
                return Some(Message::SetActivePane(ActivePane::Explorer));
            }
            Message::SetActivePane(active_pane) => match active_pane {
                ActivePane::Explorer => {
                    state.active_pane = active_pane;
                    // TODO: use event/message
                    state.explorer.set_active(true);
                }
                ActivePane::NoteEditor => {
                    state.active_pane = active_pane;
                    // TODO: use event/message
                    state.note_editor.set_active(true);
                    if state.explorer.visibility == Visibility::FullWidth {
                        return Some(Message::Explorer(explorer::Message::HidePane));
                    }
                }
                ActivePane::Outline => {
                    state.active_pane = active_pane;
                    // TODO: use event/message
                    state.outline.set_active(true);
                }
                ActivePane::Input => {
                    state.active_pane = active_pane;
                }
                _ => {}
            },
            Message::OpenVault(vault) => {
                state.vault = vault.clone();
                state.explorer = ExplorerState::new(&vault.name, vault.entries());
                state.note_editor = NoteEditorState::default();
                return Some(Message::SetActivePane(ActivePane::Explorer));
            }
            Message::SelectNote(selected_note) => {
                let is_different = state
                    .selected_note
                    .as_ref()
                    .is_some_and(|note| note.content != selected_note.content);
                state.selected_note = Some(selected_note.clone());

                state.note_editor = NoteEditorState::new(
                    &selected_note.content,
                    &selected_note.name,
                    &selected_note.path,
                );

                if !config.experimental_editor {
                    state.note_editor.view = View::Read;
                }

                // TODO: This should be behind an event/message
                state.outline = OutlineState::new(
                    &state.note_editor.ast_nodes,
                    state.note_editor.current_block(),
                    state.outline.is_open(),
                );

                if state.explorer.visibility == Visibility::FullWidth && is_different {
                    return Some(Message::Explorer(explorer::Message::HidePane));
                }
            }
            Message::UpdateSelectedNoteContent((updated_content, nodes)) => {
                if let Some(selected_note) = state.selected_note.as_mut() {
                    selected_note.content = updated_content;
                    return nodes.map(|nodes| Message::Outline(outline::Message::SetNodes(nodes)));
                }
            }
            Message::Exec(command) => {
                let (note_name, note_path) = state
                    .selected_note
                    .as_ref()
                    .map(|note| (note.name.as_str(), note.path.to_string_lossy()))
                    .unwrap_or_default();

                return command::sync_command(
                    terminal,
                    command,
                    &state.explorer.title,
                    note_name,
                    &note_path,
                );
            }

            Message::Spawn(command) => {
                let (note_name, note_path) = state
                    .selected_note
                    .as_ref()
                    .map(|note| (note.name.as_str(), note.path.to_string_lossy()))
                    .unwrap_or_default();

                return command::spawn_command(
                    command,
                    &state.explorer.title,
                    note_name,
                    &note_path,
                );
            }

            Message::HelpModal(message) => {
                return help_modal::update(&message, state.screen_size, &mut state.help_modal);
            }
            Message::VaultSelectorModal(message) => {
                return vault_selector_modal::update(&message, &mut state.vault_selector_modal);
            }
            Message::Splash(message) => {
                return splash_modal::update(&message, &mut state.splash_modal);
            }
            Message::Explorer(message) => {
                return explorer::update(&message, state.screen_size, &mut state.explorer);
            }
            Message::Outline(message) => {
                return outline::update(&message, &mut state.outline);
            }
            Message::NoteEditor(message) => {
                return note_editor::update(&message, state.screen_size, &mut state.note_editor);
            }
            Message::Input(message) => return input::update(&message, &mut state.input_modal),
        };

        None
    }

    fn render_splash(&self, area: Rect, buf: &mut Buffer, state: &mut SplashModalState<'a>) {
        SplashModal::default().render_ref(area, buf, state)
    }

    fn render_main(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        let [content, statusbar] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
            .horizontal_margin(1)
            .areas(area);

        let (left, right) = match state.explorer.visibility {
            Visibility::Hidden => (Constraint::Length(4), Constraint::Fill(1)),
            Visibility::Visible => (Constraint::Length(35), Constraint::Fill(1)),
            Visibility::FullWidth => (Constraint::Fill(1), Constraint::Length(0)),
        };

        let [explorer_pane, note, outline] = Layout::horizontal([
            left,
            right,
            if state.outline.is_open() {
                Constraint::Length(35)
            } else {
                Constraint::Length(4)
            },
        ])
        .areas(content);

        Explorer::new().render(explorer_pane, buf, &mut state.explorer);
        NoteEditor::default().render(note, buf, &mut state.note_editor);
        Outline.render(outline, buf, &mut state.outline);
        Input.render(area, buf, &mut state.input_modal);

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

        self.render_modals(area, buf, state)
    }

    fn render_modals(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        if state.splash_modal.visible {
            self.render_splash(area, buf, &mut state.splash_modal);
        }

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
        self.render_main(area, buf, state);
    }
}
