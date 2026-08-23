use basalt_core::obsidian::{self, create_untitled_dir, create_untitled_note, Note, Vault};
use ratatui::{
    buffer::Buffer,
    crossterm::{
        cursor::SetCursorStyle,
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
        execute,
    },
    layout::{Constraint, Flex, Layout, Position, Rect, Size},
    style::Style,
    widgets::{Block, StatefulWidget, Widget},
    DefaultTerminal,
};
use tracing::{debug, error, info, warn};

use std::{
    cell::RefCell,
    fmt::Debug,
    fs,
    io::Result,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    command,
    config::{self, Config, Keystroke, Theme},
    debug_log::{self, DebugLogModal, DebugLogModalState, LogLevel},
    explorer::{self, Explorer, ExplorerState, Item, Visibility},
    header::Header,
    help_modal::{self, HelpModal, HelpModalState},
    input::{self, Input, InputModalState},
    note_editor::{
        self, ast,
        editor::NoteEditor,
        state::{EditMode, Mode, NoteEditorState, View},
    },
    outline::{self, Outline, OutlineState},
    splash_modal::{self, SplashModal, SplashModalState},
    statusbar::{StatusBar, StatusBarState},
    stylized_text::{self, FontStyle},
    tabs::{Tab, Tabs},
    text_counts::{CharCount, WordCount},
    theme_selector_modal::{self, ThemeSelectorModal, ThemeSelectorModalState},
    toast::{self, Toast, TOAST_WIDTH},
    vault_selector_modal::{self, VaultSelectorModal, VaultSelectorModalState},
    vault_watcher::VaultWatcher,
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
    pending_keys: Vec<Keystroke>,

    active_pane: ActivePane,
    theme: Theme,
    explorer: ExplorerState,
    tabs: Tabs<'a>,
    outline: OutlineState,
    toasts: Vec<Toast>,

    input_modal: InputModalState,
    splash_modal: SplashModalState<'a>,
    help_modal: HelpModalState,
    vault_selector_modal: VaultSelectorModalState<'a>,
    debug_log_modal: DebugLogModalState,
    theme_selector_modal: ThemeSelectorModalState,
}

impl<'a> AppState<'a> {
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub fn active_component(&self) -> ActivePane {
        // Ordered top-most first, matching the modal render (z) order so the
        // visually top modal is the one that receives input.
        if self.debug_log_modal.visible {
            return ActivePane::DebugLogModal;
        }

        if self.help_modal.visible {
            return ActivePane::HelpModal;
        }

        if self.theme_selector_modal.visible {
            return ActivePane::ThemeSelectorModal;
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
    CopyToClipboard(String),
    Resize(Size),
    SetActivePane(ActivePane),
    RefreshVault {
        rename: Option<(PathBuf, PathBuf)>,
        select: Option<PathBuf>,
    },
    RescanVault,
    CreateUntitledNote,
    CreateUntitledFolder,
    OpenVault(&'a Vault),
    SelectNote(SelectedNote),
    UpdateSelectedNoteContent((String, Option<Vec<ast::Node>>)),
    TabNext,
    TabPrevious,
    CloseTab,

    Batch(Vec<Message<'a>>),
    Toast(toast::Message),
    Input(input::Message),
    Splash(splash_modal::Message),
    Explorer(explorer::Message),
    NoteEditor(note_editor::Message),
    Outline(outline::Message),
    HelpModal(help_modal::Message),
    VaultSelectorModal(vault_selector_modal::Message),
    DebugLog(debug_log::Message),
    ThemeSelectorModal(theme_selector_modal::Message),
    PreviewTheme(Theme),
    SaveTheme(String),
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
    DebugLogModal,
    ThemeSelectorModal,
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
            ActivePane::DebugLogModal => "Debug Log",
            ActivePane::ThemeSelectorModal => "Theme Selector",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SelectedNote {
    name: String,
    path: PathBuf,
    content: String,
}

impl SelectedNote {
    pub fn new(name: &str, path: &Path, content: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_path_buf(),
            content: content.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn set_path(&mut self, path: &Path) {
        self.path = path.to_path_buf();
    }
}

impl From<Note> for SelectedNote {
    fn from(value: Note) -> Self {
        Self {
            name: value.name().to_string(),
            path: value.path().to_path_buf(),
            content: fs::read_to_string(value.path()).unwrap_or_default(),
        }
    }
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

/// Applies a colour theme to the app and every sub-state that caches it.
/// The note editor re-lays out so its baked-in span colours update.
fn apply_theme(state: &mut AppState, theme: Theme) {
    state.theme = theme;
    state.explorer.set_theme(&theme);
    state.outline.set_theme(&theme);
    state.tabs.set_theme(&theme);
}

fn active_config_section<'a>(
    config: &'a Config,
    active: ActivePane,
) -> &'a config::ConfigSection<'a> {
    match active {
        ActivePane::Splash => &config.splash,
        ActivePane::Explorer => &config.explorer,
        ActivePane::Outline => &config.outline,
        ActivePane::HelpModal => &config.help_modal,
        ActivePane::VaultSelectorModal => &config.vault_selector_modal,
        ActivePane::ThemeSelectorModal => &config.theme_selector_modal,
        ActivePane::Input => &config.input_modal,
        ActivePane::NoteEditor => &config.note_editor,
        ActivePane::DebugLogModal => &config.debug_log_modal,
    }
}

fn focused_cursor(state: &AppState) -> Option<Position> {
    match state.active_component() {
        ActivePane::NoteEditor => state
            .tabs
            .active_editor()
            .and_then(|editor| editor.terminal_cursor),
        ActivePane::Input => state.input_modal.terminal_cursor,
        _ => None,
    }
}

fn cursor_style(state: &AppState) -> SetCursorStyle {
    let inserting = match state.active_component() {
        ActivePane::NoteEditor => matches!(
            state.tabs.active_editor().map(NoteEditorState::mode),
            Some(Mode::Insert | Mode::Edit)
        ),
        ActivePane::Input => state.input_modal.is_editing(),
        _ => false,
    };
    if inserting {
        SetCursorStyle::SteadyBar
    } else {
        SetCursorStyle::SteadyBlock
    }
}

fn rebuild_outline(state: &mut AppState, config: &Config) {
    let is_open = state.outline.is_open();
    let was_active = state.outline.active;
    state.outline = match state.tabs.active_editor() {
        Some(editor) => OutlineState::new(
            &editor.ast_nodes,
            editor.current_block_idx(),
            is_open,
            &config.symbols,
        ),
        None => OutlineState::new(&[], 0, is_open, &config.symbols),
    };
    state.outline.set_active(was_active);
    // A fresh OutlineState carries the default theme; re-apply the active one so
    // rebuilding (e.g. on a tab switch) doesn't revert it to the default look.
    state.outline.set_theme(&state.theme);
}

fn focus_active_editor(state: &mut AppState) {
    let focused = state.active_pane == ActivePane::NoteEditor;
    if let Some(editor) = state.tabs.active_editor_mut() {
        editor.set_active(focused);
    }
}

fn sync_explorer_to_active_tab(state: &mut AppState) {
    if let Some(path) = state
        .tabs
        .active_note()
        .map(|note| note.path().to_path_buf())
    {
        state.explorer.reveal_path(&path);
    }
}

/// Raw normal-mode keys that no static binding can express; `None` falls
/// through to the config keybindings.
fn normal_mode_raw_key(
    editor: &note_editor::state::NoteEditorState,
    key: &KeyEvent,
) -> Option<note_editor::Message> {
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    if editor.awaiting_replace() {
        return Some(note_editor::Message::ReplaceTarget(character));
    }
    if editor.awaiting_text_object() {
        return Some(note_editor::Message::TextObjectTarget(character));
    }
    if editor.awaiting_find_target() {
        return Some(note_editor::Message::FindTarget(character));
    }
    match character.to_digit(10) {
        // A lone `0` is the line-start motion; only a count in progress claims it.
        Some(digit) if digit != 0 || editor.has_pending_count() => {
            Some(note_editor::Message::CountDigit(digit as u8))
        }
        _ => None,
    }
}

pub struct App<'a> {
    state: AppState<'a>,
    config: Config<'a>,
    terminal: RefCell<DefaultTerminal>,
    vault_watcher: RefCell<Option<VaultWatcher>>,
}

impl<'a> App<'a> {
    pub fn new(state: AppState<'a>, config: Config<'a>, terminal: DefaultTerminal) -> Self {
        Self {
            state,
            // TODO: Surface toast if read config returns error
            config,
            terminal: RefCell::new(terminal),
            vault_watcher: RefCell::new(None),
        }
    }

    fn ensure_watcher_for(&self, path: &Path) {
        let mut current = self.vault_watcher.borrow_mut();
        let needs_swap = match current.as_ref() {
            Some(watcher) => watcher.path() != path,
            None => !path.as_os_str().is_empty(),
        };
        if !needs_swap {
            return;
        }
        *current = if path.is_dir() {
            VaultWatcher::new(path).ok()
        } else {
            None
        };
    }

    fn watcher_has_changes(&self) -> bool {
        self.vault_watcher
            .borrow()
            .as_ref()
            .is_some_and(|w| w.drain())
    }

    pub fn start(
        terminal: DefaultTerminal,
        vaults: Vec<&Vault>,
        initial_vault: Option<Vault>,
        debug: bool,
        log_level: LogLevel,
        theme_override: Option<String>,
    ) -> Result<()> {
        let version = stylized_text::stylize(VERSION, FontStyle::Script);
        let size = terminal.size()?;
        let (mut config, warnings) = config::load().unwrap();

        if let Some(name) = &theme_override {
            config.theme = config::theme::theme_by_name(name);
        }

        let vault = initial_vault.clone().unwrap_or_default();
        let explorer = match &initial_vault {
            Some(v) => ExplorerState::new(&v.name, v.entries(), &config.symbols),
            None => ExplorerState::default(),
        };
        let active_pane = if initial_vault.is_some() {
            ActivePane::Explorer
        } else {
            ActivePane::default()
        };

        let mut state = AppState {
            vault,
            explorer,
            active_pane,
            theme: config.theme,
            screen_size: size,
            help_modal: HelpModalState::new(&help_text(&version)),
            vault_selector_modal: VaultSelectorModalState::new(vaults.clone()),
            theme_selector_modal: ThemeSelectorModalState::new(config::theme::load_themes()),
            splash_modal: SplashModalState::new(&version, vaults, initial_vault.is_none()),
            outline: OutlineState {
                symbols: config.symbols.clone(),
                ..Default::default()
            },
            debug_log_modal: DebugLogModalState {
                visible: debug,
                min_level: log_level,
                ..Default::default()
            },
            toasts: warnings
                .into_iter()
                .map(|message| {
                    warn!(message, "config warning");
                    toast::Toast::warn(&message, Duration::from_secs(5))
                })
                .collect(),
            ..Default::default()
        };

        apply_theme(&mut state, config.theme);

        App::new(state, config, terminal).run()
    }

    fn run(&'a mut self) -> Result<()> {
        self.state.is_running = true;

        let mut state = self.state.clone();
        let config = self.config.clone();

        self.ensure_watcher_for(&state.vault.path);

        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        while state.is_running {
            self.draw(&mut state)?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)? {
                let event = event::read()?;

                let mut message = App::handle_event(&config, &mut state, event);
                while message.is_some() {
                    message = App::update(self.terminal.get_mut(), &config, &mut state, message);
                }
                self.ensure_watcher_for(&state.vault.path);
            }

            if self.watcher_has_changes() {
                let mut message = Some(Message::RescanVault);
                while message.is_some() {
                    message = App::update(self.terminal.get_mut(), &config, &mut state, message);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                App::update(
                    self.terminal.get_mut(),
                    &config,
                    &mut state,
                    Some(Message::Toast(toast::Message::Tick)),
                );
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn draw(&self, state: &mut AppState<'a>) -> Result<()> {
        let mut terminal = self.terminal.borrow_mut();

        terminal.draw(|frame| {
            let area = frame.area();
            self.render(area, frame.buffer_mut(), state);

            if let Some(position) = focused_cursor(state) {
                frame.set_cursor_position(position);
            }
        })?;

        if focused_cursor(state).is_some() {
            execute!(terminal.backend_mut(), cursor_style(state))?;
        }

        Ok(())
    }

    fn handle_event(
        config: &'a Config,
        state: &mut AppState<'_>,
        event: Event,
    ) -> Option<Message<'a>> {
        match event {
            Event::Resize(cols, rows) => Some(Message::Resize(Size::new(cols, rows))),
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                App::handle_key_event(config, state, key_event)
            }
            _ => None,
        }
    }

    fn handle_key_event(
        config: &'a Config,
        state: &mut AppState<'_>,
        key_event: KeyEvent,
    ) -> Option<Message<'a>> {
        // Vim normal-mode inputs no static binding can express: a pending
        // replace/find/text-object target, or count digits.
        if matches!(state.active_component(), ActivePane::NoteEditor) {
            let raw = state
                .tabs
                .active_editor()
                .filter(|editor| editor.is_editing() && editor.vim_mode() && !editor.insert_mode())
                .and_then(|editor| normal_mode_raw_key(editor, &key_event));
            if let Some(message) = raw {
                state.pending_keys.clear();
                return Some(Message::NoteEditor(message));
            }
        }

        match state.active_component() {
            ActivePane::NoteEditor
                if state
                    .tabs
                    .active_editor()
                    .is_some_and(|editor| editor.is_editing() && editor.insert_mode()) =>
            {
                state.pending_keys.clear();
                note_editor::handle_editing_event(key_event).map(Message::NoteEditor)
            }
            ActivePane::Input if state.input_modal.is_editing() => {
                state.pending_keys.clear();
                input::handle_editing_event(key_event).map(Message::Input)
            }
            active => App::handle_pending_keys(
                Keystroke::from(key_event),
                config,
                active,
                &mut state.pending_keys,
            ),
        }
    }

    fn handle_pending_keys(
        key: Keystroke,
        config: &'a Config,
        active: ActivePane,
        pending_keys: &mut Vec<Keystroke>,
    ) -> Option<Message<'a>> {
        pending_keys.push(key.clone());
        let section = active_config_section(config, active);

        let global_message = config.global.sequence_to_message(pending_keys);
        if global_message.is_some() {
            pending_keys.clear();
            return global_message;
        }

        let section_message = section.sequence_to_message(pending_keys);
        if section_message.is_some() {
            pending_keys.clear();
            return section_message;
        }

        let is_sequence_prefix = config.global.is_sequence_prefix(pending_keys)
            || section.is_sequence_prefix(pending_keys);

        if is_sequence_prefix {
            return None;
        }

        let is_sequence = pending_keys.len() > 1;

        pending_keys.clear();
        is_sequence
            .then(|| App::handle_pending_keys(key, config, active, pending_keys))
            .flatten()
    }

    fn update(
        terminal: &mut DefaultTerminal,
        config: &Config,
        state: &mut AppState<'a>,
        message: Option<Message<'a>>,
    ) -> Option<Message<'a>> {
        match message? {
            Message::Batch(messages) => {
                for msg in messages {
                    let mut next = Some(msg);
                    while next.is_some() {
                        next = App::update(terminal, config, state, next);
                    }
                }
            }
            Message::Quit => state.is_running = false,
            Message::Resize(size) => state.screen_size = size,
            Message::RefreshVault { rename, select } => {
                if let Some((old, new)) = &rename {
                    // FIXME: Handle error propagation when wiki link update fails
                    if let Err(error) = obsidian::vault::update_wiki_links(state.vault(), old, new)
                    {
                        warn!(?error, "failed to update wiki links");
                    }
                    let name = new
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or_default();
                    state.tabs.rename(old, new, name);
                }
                state.explorer.with_entries(state.vault.entries(), select);
                debug!(?rename, "refreshed vault");

                // Reload the note editor for the currently selected note
                let selected_note = if state
                    .explorer
                    .list_state
                    .selected()
                    .zip(state.explorer.selected_item_index)
                    .is_some_and(|(a, b)| a == b)
                {
                    if let Some(Item::File { note, .. }) = state.explorer.current_item() {
                        Some(SelectedNote::from(note))
                    } else {
                        None
                    }
                } else {
                    state.tabs.active_note().cloned()
                };

                if let Some(note) = selected_note {
                    return Some(Message::Batch(vec![
                        Message::SelectNote(note),
                        Message::SetActivePane(ActivePane::Explorer),
                    ]));
                }
                return Some(Message::SetActivePane(ActivePane::Explorer));
            }
            Message::RescanVault => {
                state.explorer.refresh_entries(state.vault.entries());
                debug!("rescanned vault after watcher change");
            }
            Message::CreateUntitledNote => {
                let path = match state.explorer.current_item() {
                    Some(Item::Directory { path, .. }) => path,
                    Some(Item::File { note, .. }) => {
                        note.path().parent().unwrap_or(&state.vault.path)
                    }
                    _ => &state.vault.path,
                };
                match create_untitled_note(path) {
                    Ok(note) => {
                        info!(path = %note.path().display(), "created note");
                        return Some(Message::Batch(vec![
                            Message::Explorer(explorer::Message::Open),
                            Message::RefreshVault {
                                rename: None,
                                select: Some(note.path().to_path_buf()),
                            },
                            Message::Toast(toast::Message::Create(toast::Toast::success(
                                "Note created",
                                Duration::from_secs(2),
                            ))),
                            Message::SelectNote(note.into()),
                        ]));
                    }
                    Err(error) => {
                        error!(?error, "failed to create note");
                        return Some(Message::Toast(toast::Message::Create(toast::Toast::error(
                            "Failed to create a new note",
                            Duration::from_secs(2),
                        ))));
                    }
                }
            }
            Message::CreateUntitledFolder => {
                let path = match state.explorer.current_item() {
                    Some(Item::Directory { path, .. }) => path,
                    Some(Item::File { note, .. }) => {
                        note.path().parent().unwrap_or(&state.vault.path)
                    }
                    _ => &state.vault.path,
                };
                match create_untitled_dir(path) {
                    Ok(note) => {
                        info!(path = %note.path().display(), "created folder");
                        return Some(Message::Batch(vec![
                            Message::Explorer(explorer::Message::Open),
                            Message::RefreshVault {
                                rename: None,
                                select: Some(note.path().to_path_buf()),
                            },
                            Message::Toast(toast::Message::Create(toast::Toast::success(
                                "Folder created",
                                Duration::from_secs(2),
                            ))),
                        ]));
                    }
                    Err(error) => {
                        error!(?error, "failed to create folder");
                        return Some(Message::Toast(toast::Message::Create(toast::Toast::error(
                            "Failed to create a new folder",
                            Duration::from_secs(2),
                        ))));
                    }
                }
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
                    if let Some(editor) = state.tabs.active_editor_mut() {
                        editor.set_active(true);
                    }
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
                info!(vault = %vault.name, "opened vault");
                state.vault = vault.clone();
                state.explorer = ExplorerState::new(&vault.name, vault.entries(), &config.symbols);
                state.tabs = Tabs::default();
                rebuild_outline(state, config);
                apply_theme(state, state.theme);
                return Some(Message::SetActivePane(ActivePane::Explorer));
            }
            Message::SelectNote(selected_note) => {
                info!(note = %selected_note.name, "selected note");
                let is_different = state
                    .tabs
                    .active_note()
                    .is_some_and(|note| note.content != selected_note.content);

                if !state.tabs.open_or_focus(selected_note.path()) {
                    let mut editor = NoteEditorState::new(
                        &selected_note.content,
                        &selected_note.name,
                        &selected_note.path,
                        &config.symbols,
                    );
                    editor.set_vim_mode(config.vim_mode);
                    editor.set_editor_enabled(config.experimental_editor);
                    editor.set_wrap(config.wrap);
                    if config.experimental_editor && config.vim_mode {
                        editor.set_view(View::Edit(EditMode::Source));
                    } else {
                        editor.set_view(View::Read);
                    }
                    state.tabs.open(Tab {
                        note: selected_note,
                        editor,
                    });
                }

                rebuild_outline(state, config);

                apply_theme(state, state.theme);

                if state.explorer.visibility == Visibility::FullWidth && is_different {
                    return Some(Message::Explorer(explorer::Message::HidePane));
                }
            }
            Message::UpdateSelectedNoteContent((updated_content, nodes)) => {
                if let Some(selected_note) = state.tabs.active_note_mut() {
                    selected_note.content = updated_content;
                    return nodes.map(|nodes| Message::Outline(outline::Message::SetNodes(nodes)));
                }
            }
            Message::TabNext => {
                state.tabs.next();
                focus_active_editor(state);
                sync_explorer_to_active_tab(state);
                rebuild_outline(state, config);
            }
            Message::TabPrevious => {
                state.tabs.prev();
                focus_active_editor(state);
                sync_explorer_to_active_tab(state);
                rebuild_outline(state, config);
            }
            Message::CloseTab => {
                state.tabs.close_active();
                focus_active_editor(state);
                sync_explorer_to_active_tab(state);
                rebuild_outline(state, config);
            }
            Message::Exec(command) => {
                let (note_name, note_path) = state
                    .tabs
                    .active_note()
                    .map(|note| (note.name(), note.path().to_string_lossy()))
                    .unwrap_or_default();

                return command::sync_command(
                    terminal,
                    command,
                    &state.vault.name,
                    note_name,
                    &note_path,
                );
            }

            Message::Spawn(command) => {
                let (note_name, note_path) = state
                    .tabs
                    .active_note()
                    .map(|note| (note.name(), note.path().to_string_lossy()))
                    .unwrap_or_default();

                return command::spawn_command(command, &state.vault.name, note_name, &note_path);
            }

            Message::CopyToClipboard(text) => {
                let toast = match crate::clipboard::copy(&text) {
                    Ok(_) => Toast::success("Yanked to clipboard", Duration::from_secs(2)),
                    Err(_) => Toast::error("Failed to copy to clipboard", Duration::from_secs(2)),
                };
                return Some(Message::Toast(toast::Message::Create(toast)));
            }

            Message::HelpModal(message) => {
                return help_modal::update(&message, state.screen_size, &mut state.help_modal);
            }
            Message::VaultSelectorModal(message) => {
                return vault_selector_modal::update(&message, &mut state.vault_selector_modal);
            }
            Message::ThemeSelectorModal(message) => {
                return theme_selector_modal::update(
                    &message,
                    &mut state.theme_selector_modal,
                    state.theme,
                );
            }
            Message::PreviewTheme(theme) => apply_theme(state, theme),
            Message::SaveTheme(name) => {
                let toast = match config::save_theme(&name) {
                    Ok(_) => {
                        Toast::success(&format!("Saved theme \"{name}\""), Duration::from_secs(2))
                    }
                    Err(error) => Toast::error(
                        &format!("Could not save theme: {error}"),
                        Duration::from_secs(4),
                    ),
                };
                return Some(Message::Toast(toast::Message::Create(toast)));
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
                let size = state.screen_size;
                if let Some(editor) = state.tabs.active_editor_mut() {
                    return note_editor::update(message, size, editor);
                }
                // With no open tab there is no editor to update, but pane
                // navigation must still work so the user isn't trapped in the
                // empty editor.
                return match message {
                    note_editor::Message::SwitchPaneNext => {
                        Some(Message::SetActivePane(ActivePane::Outline))
                    }
                    note_editor::Message::SwitchPanePrevious => {
                        Some(Message::SetActivePane(ActivePane::Explorer))
                    }
                    note_editor::Message::ToggleExplorer => {
                        Some(Message::Explorer(explorer::Message::Toggle))
                    }
                    note_editor::Message::ToggleOutline => {
                        Some(Message::Outline(outline::Message::Toggle))
                    }
                    _ => None,
                };
            }
            Message::Input(message) => return input::update(message, &mut state.input_modal),
            Message::DebugLog(message) => {
                return debug_log::update(&message, state.screen_size, &mut state.debug_log_modal);
            }
            Message::Toast(message) => return toast::update(message, &mut state.toasts),
        };

        None
    }

    fn render_splash(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: Theme,
        state: &mut SplashModalState<'a>,
    ) {
        let border_modal = self.config.symbols.border_modal.into();
        let vault_active = self.config.symbols.vault_active.clone();
        let preset = self.config.symbols.preset;
        SplashModal::new(border_modal, vault_active, theme, preset).render(area, buf, state)
    }

    fn render_main(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        let [header, content, statusbar] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .horizontal_margin(1)
        .areas(area);

        Header::new(&self.config.symbols, &state.theme, &state.tabs).render(header, buf);

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

        let theme = state.theme;

        Explorer::new().render(explorer_pane, buf, &mut state.explorer);
        match state.tabs.active_editor_mut() {
            Some(editor) => NoteEditor::default().render(note, buf, editor),
            None => {
                let mut empty = NoteEditorState::new("", "", Path::new(""), &self.config.symbols);
                empty.set_theme(&theme);
                empty.set_active(state.active_pane == ActivePane::NoteEditor);
                NoteEditor::default().render(note, buf, &mut empty);
            }
        }
        Outline.render(outline, buf, &mut state.outline);
        let border_modal = self.config.symbols.border_modal.into();
        Input::new(border_modal, theme).render(explorer_pane, buf, &mut state.input_modal);

        let (word_count, char_count) = state
            .tabs
            .active_note()
            .map(|note| {
                let content = note.content.as_str();
                (WordCount::from(content), CharCount::from(content))
            })
            .unwrap_or_default();

        let mode = state
            .tabs
            .active_editor()
            .map(|editor| editor.mode())
            .unwrap_or_default();

        let mut status_bar_state = StatusBarState::new(
            state.active_pane.into(),
            mode,
            word_count.into(),
            char_count.into(),
        );

        let status_bar = StatusBar::new(&theme);
        status_bar.render(statusbar, buf, &mut status_bar_state);

        self.render_modals(area, buf, state);
        self.render_toasts(area, buf, state);

        if state.debug_log_modal.visible {
            let border_modal = self.config.symbols.border_modal.into();
            let memory_mb =
                memory_stats::memory_stats().map(|stats| stats.physical_mem as f64 / 1_048_576.0);
            DebugLogModal::new(border_modal, state.theme, memory_mb).render(
                area,
                buf,
                &mut state.debug_log_modal,
            );
        }
    }

    fn render_modals(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        let theme = state.theme;

        if state.splash_modal.visible {
            self.render_splash(area, buf, theme, &mut state.splash_modal);
        }

        if state.vault_selector_modal.visible {
            let border_modal = self.config.symbols.border_modal.into();
            let vault_active = self.config.symbols.vault_active.clone();
            VaultSelectorModal::new(border_modal, vault_active, theme).render(
                area,
                buf,
                &mut state.vault_selector_modal,
            );
        }

        if state.theme_selector_modal.visible {
            let border_modal = self.config.symbols.border_modal.into();
            ThemeSelectorModal::new(border_modal, theme).render(
                area,
                buf,
                &mut state.theme_selector_modal,
            );
        }

        if state.help_modal.visible {
            let border_modal = self.config.symbols.border_modal.into();
            HelpModal::new(border_modal, theme).render(area, buf, &mut state.help_modal);
        }
    }

    fn render_toasts(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        let [_, toast_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(TOAST_WIDTH)])
                .horizontal_margin(1)
                .flex(Flex::End)
                .areas(area);

        let mut y_offset: u16 = 0;
        state.toasts.iter().rev().for_each(|toast| {
            let mut toast_area = toast_area;
            toast_area.y += y_offset;
            y_offset += toast.height();
            if toast_area.y >= area.bottom() {
                return;
            }
            let mut toast = toast.clone();
            toast.border_type = self.config.symbols.border_modal.into();
            toast.icon = toast.level_icon(&self.config.symbols);
            toast.theme = state.theme;
            toast.render(toast_area, buf)
        });
    }
}

impl<'a> StatefulWidget for &App<'a> {
    type State = AppState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Block::new()
            .style(Style::new().bg(state.theme.background))
            .render(area, buf);
        self.render_main(area, buf, state);
    }
}
