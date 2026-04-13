use basalt_core::obsidian::{self, create_untitled_dir, create_untitled_note, Note, Vault};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect, Size},
    widgets::{StatefulWidget, Widget},
    DefaultTerminal,
};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use std::{
    cell::RefCell,
    fmt::Debug,
    fs,
    io::Result,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    command,
    config::{self, Config, Keystroke},
    explorer::{self, Explorer, ExplorerState, Item, Visibility},
    help_modal::{self, HelpModal, HelpModalState},
    input::{self, Input, InputModalState},
    note_editor::{
        self, ast,
        editor::NoteEditor,
        state::{EditMode, NoteEditorState, View},
    },
    outline::{self, Outline, OutlineState},
    splash_modal::{self, SplashModal, SplashModalState},
    statusbar::{StatusBar, StatusBarState},
    stylized_text::{self, FontStyle},
    text_counts::{CharCount, WordCount},
    toast::{self, Toast, TOAST_WIDTH},
    vault_selector_modal::{self, VaultSelectorModal, VaultSelectorModalState},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_TEXT: &str = include_str!("./help.txt");

#[derive(Clone)]
pub struct SyntectContext {
    pub syntax_set: SyntaxSet,
    pub theme: syntect::highlighting::Theme,
    pub selection_color: Option<ratatui::style::Color>,
}

impl std::fmt::Debug for SyntectContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntectContext").finish_non_exhaustive()
    }
}

impl SyntectContext {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get("base16-ocean.dark")
            .or_else(|| theme_set.themes.values().next())
            .cloned()
            .unwrap_or_default();
        let selection_color = theme
            .settings
            .selection
            .map(|c| ratatui::style::Color::Rgb(c.r, c.g, c.b));
        Self {
            syntax_set,
            theme,
            selection_color,
        }
    }
}

impl Default for SyntectContext {
    fn default() -> Self {
        Self::new()
    }
}

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
    explorer: ExplorerState,
    note_editor: NoteEditorState<'a>,
    outline: OutlineState,
    selected_note: Option<SelectedNote>,
    toasts: Vec<Toast>,

    input_modal: InputModalState,
    splash_modal: SplashModalState<'a>,
    help_modal: HelpModalState,
    vault_selector_modal: VaultSelectorModalState<'a>,
    syntect_ctx: Option<SyntectContext>,
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

    pub fn syntect_ctx(&self) -> Option<&SyntectContext> {
        self.syntect_ctx.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message<'a> {
    Quit,
    Exec(String),
    Spawn(String),
    Resize(Size),
    SetActivePane(ActivePane),
    RefreshVault {
        rename: Option<(PathBuf, PathBuf)>,
        select: Option<PathBuf>,
    },
    CreateUntitledNote,
    CreateUntitledFolder,
    OpenVault(&'a Vault),
    SelectNote(SelectedNote),
    UpdateSelectedNoteContent((String, Option<Vec<ast::Node>>)),

    Batch(Vec<Message<'a>>),
    Toast(toast::Message),
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
        ActivePane::Input => &config.input_modal,
        ActivePane::NoteEditor => &config.note_editor,
    }
}

pub struct App<'a> {
    state: AppState<'a>,
    config: Config<'a>,
    terminal: RefCell<DefaultTerminal>,
}

impl<'a> App<'a> {
    pub fn new(state: AppState<'a>, config: Config<'a>, terminal: DefaultTerminal) -> Self {
        Self {
            state,
            // TODO: Surface toast if read config returns error
            config,
            terminal: RefCell::new(terminal),
        }
    }

    pub fn start(terminal: DefaultTerminal, vaults: Vec<&Vault>) -> Result<()> {
        let version = stylized_text::stylize(VERSION, FontStyle::Script);
        let size = terminal.size()?;
        let (config, warnings) = config::load().unwrap();

        let state = AppState {
            screen_size: size,
            help_modal: HelpModalState::new(&help_text(&version)),
            vault_selector_modal: VaultSelectorModalState::new(vaults.clone()),
            splash_modal: SplashModalState::new(&version, vaults, true),
            outline: OutlineState {
                symbols: config.symbols.clone(),
                ..Default::default()
            },
            toasts: warnings
                .into_iter()
                .map(|message| toast::Toast::warn(&message, Duration::from_secs(5)))
                .collect(),
            syntect_ctx: Some(SyntectContext::new()),
            ..Default::default()
        };

        App::new(state, config, terminal).run()
    }

    fn run(&'a mut self) -> Result<()> {
        self.state.is_running = true;

        let mut state = self.state.clone();
        let config = self.config.clone();

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
            let buf = frame.buffer_mut();
            self.render(area, buf, state);
        })?;

        // OSC 8 hyperlink emission — secondary pass after ratatui draw
        // Always emitted; terminals without OSC 8 support silently ignore
        self.emit_osc8_links(state)?;

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
        match state.active_component() {
            ActivePane::NoteEditor
                if state.note_editor.is_editing() && state.note_editor.insert_mode() =>
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
                    let _ = obsidian::vault::update_wiki_links(state.vault(), old, new);
                }
                state.explorer.with_entries(state.vault.entries(), select);

                // Reload the note editor for the currently selected note
                let selected_note = if state
                    .explorer
                    .list_state
                    .selected()
                    .zip(state.explorer.selected_item_index)
                    .is_some_and(|(a, b)| a == b)
                {
                    if let Some(Item::File(note)) = state.explorer.current_item() {
                        Some(SelectedNote::from(note))
                    } else {
                        None
                    }
                } else {
                    state.selected_note.clone()
                };

                if let Some(note) = selected_note {
                    return Some(Message::Batch(vec![
                        Message::SelectNote(note),
                        Message::SetActivePane(ActivePane::Explorer),
                    ]));
                }
                return Some(Message::SetActivePane(ActivePane::Explorer));
            }
            Message::CreateUntitledNote => match create_untitled_note(&state.vault) {
                Ok(note) => {
                    return Some(Message::Batch(vec![
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
                Err(_) => {
                    return Some(Message::Toast(toast::Message::Create(toast::Toast::error(
                        "Failed to create a new note",
                        Duration::from_secs(2),
                    ))));
                }
            },
            Message::CreateUntitledFolder => match create_untitled_dir(&state.vault) {
                Ok(note) => {
                    return Some(Message::Batch(vec![
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
                Err(_) => {
                    return Some(Message::Toast(toast::Message::Create(toast::Toast::error(
                        "Failed to create a new folder",
                        Duration::from_secs(2),
                    ))));
                }
            },
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
                state.explorer = ExplorerState::new(&vault.name, vault.entries(), &config.symbols);
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
                    &config.symbols,
                    state.syntect_ctx.as_ref(),
                );

                let vim_mode = config.vim_mode;
                state.note_editor.set_vim_mode(vim_mode);

                let editor_enabled = config.experimental_editor;
                state.note_editor.set_editor_enabled(editor_enabled);

                if editor_enabled && vim_mode {
                    state.note_editor.set_view(View::Edit(EditMode::Source));
                } else {
                    state.note_editor.set_view(View::Read);
                }

                // TODO: This should be behind an event/message
                state.outline = OutlineState::new(
                    &state.note_editor.ast_nodes,
                    state.note_editor.current_block(),
                    state.outline.is_open(),
                    &config.symbols,
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
                    &state.vault.name,
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

                return command::spawn_command(command, &state.vault.name, note_name, &note_path);
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
                return note_editor::update(message, state.screen_size, &mut state.note_editor);
            }
            Message::Input(message) => return input::update(message, &mut state.input_modal),
            Message::Toast(message) => return toast::update(message, &mut state.toasts),
        };

        None
    }

    fn render_splash(&self, area: Rect, buf: &mut Buffer, state: &mut SplashModalState<'a>) {
        let border_modal = self.config.symbols.border_modal.into();
        let vault_active = self.config.symbols.vault_active.clone();
        SplashModal::new(border_modal, vault_active).render(area, buf, state)
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
        let border_modal = self.config.symbols.border_modal.into();
        Input::new(border_modal).render(area, buf, &mut state.input_modal);

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
        status_bar.render(statusbar, buf, &mut status_bar_state);

        self.render_modals(area, buf, state);
        self.render_toasts(area, buf, state);
    }

    fn render_modals(&self, area: Rect, buf: &mut Buffer, state: &mut AppState<'a>) {
        if state.splash_modal.visible {
            self.render_splash(area, buf, &mut state.splash_modal);
        }

        if state.vault_selector_modal.visible {
            let border_modal = self.config.symbols.border_modal.into();
            let vault_active = self.config.symbols.vault_active.clone();
            VaultSelectorModal::new(border_modal, vault_active).render(
                area,
                buf,
                &mut state.vault_selector_modal,
            );
        }

        if state.help_modal.visible {
            let border_modal = self.config.symbols.border_modal.into();
            HelpModal::new(border_modal).render(area, buf, &mut state.help_modal);
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
            toast.render(toast_area, buf)
        });
    }

    /// Emit OSC 8 terminal hyperlink sequences for external links visible in the viewport.
    ///
    /// Called after `terminal.draw()` each frame. Positions are computed from `link_map`
    /// entries (populated during layout) relative to `inner_area` (set during render).
    ///
    /// OSC 8 format: `\x1b]8;;<url>\x07<text>\x1b]8;;\x07`
    /// BEL terminator (\x07) chosen for broadest terminal compatibility.
    ///
    /// Graceful degradation: terminals that do not support OSC 8 silently ignore the
    /// sequences — the link text still appears with its visual style (underline + color).
    fn emit_osc8_links(&self, state: &AppState<'_>) -> Result<()> {
        use crate::note_editor::rich_text::LinkTarget;
        use std::io::Write;

        let inner_area = state.note_editor.inner_area;
        let viewport_top = state.note_editor.viewport().top() as usize;
        let viewport_height = inner_area.height as usize;

        let mut stdout = std::io::stdout();

        // Reset any lingering OSC 8 state from previous frame
        let _ = write!(stdout, "\x1b]8;;\x07");
        let _ = stdout.flush();

        for entry in &state.note_editor.link_map {
            if let LinkTarget::External(url) = &entry.target {
                let line_idx = entry.line_idx;

                // Skip lines outside viewport
                if line_idx < viewport_top {
                    continue;
                }
                let visible_row = line_idx - viewport_top;
                if visible_row >= viewport_height {
                    continue;
                }

                let screen_x = inner_area.x + entry.col_start as u16;
                let screen_y = inner_area.y + visible_row as u16;

                // Emit OSC 8: open hyperlink, print text, close hyperlink
                let osc8 = format!("\x1b[{};{}H\x1b]8;;{}\x07{}\x1b]8;;\x07",
                    screen_y + 1, screen_x + 1, url, entry.text);
                let _ = write!(stdout, "{osc8}");
            }
        }

        let _ = stdout.flush();
        Ok(())
    }
}

impl<'a> StatefulWidget for &App<'a> {
    type State = AppState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_main(area, buf, state);
    }
}
