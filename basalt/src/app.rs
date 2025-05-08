use basalt_core::obsidian::Vault;
use basalt_widgets::{
    help::{Help, HelpModalState},
    markdown::MarkdownView,
    statusbar::{StatusBar, StatusBarState},
    vault::{VaultSelector, VaultSelectorState},
};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Size},
    widgets::{StatefulWidget, StatefulWidgetRef},
    DefaultTerminal,
};

use crate::{
    actions::Action,
    events::key_event_to_action,
    main_view::MainView,
    mode::Mode,
    screen::Screen,
    start_view::{StartView, StartViewState},
    text_counts::{CharCount, WordCount},
};
use std::{fs, io::Result, marker::PhantomData, time::Duration};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HELP_TEXT: &str = include_str!("./help.txt");

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Context {
    HelpModal,
    VaultSelector,
    #[default]
    StartScreen,
    Explorer,
    Editor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppState<'a> {
    pub vaults: Vec<&'a Vault>,
    pub selected_vault: Option<&'a Vault>,
    pub help_modal_state: Option<HelpModalState>,
    pub vault_selector_state: Option<VaultSelectorState<'a>>,
    pub size: Size,
    pub is_running: bool,
    pub ctx: Context,
    pub screen: Screen<'a>,
    pub previous_ctx: Option<Context>, // This is for restoring context after modal closes
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> AppState<'a> {
    fn new(vaults: Vec<&'a Vault>, size: Size, version: &'a str) -> Self {
        AppState {
            vaults: vaults.clone(),
            selected_vault: None,
            help_modal_state: None,
            vault_selector_state: None,
            size,
            is_running: true,
            ctx: Context::default(),
            screen: Screen::Start(StartView {
                start_state: StartViewState::new(version, size, vaults),
            }),
            previous_ctx: None,
            _lifetime: PhantomData,
        }
    }
}

pub struct App<'a> {
    state: AppState<'a>,
    terminal: DefaultTerminal,
}

impl<'a> App<'a> {
    pub fn start(terminal: DefaultTerminal, vaults: Vec<&'a Vault>) -> Result<()> {
        let version = format!("{VERSION}~alpha");
        let size = terminal.size()?;
        let initial_state = AppState::new(vaults, size, &version);

        let mut app = App {
            state: initial_state,
            terminal,
        };
        app.run()
    }

    pub fn run(&mut self) -> Result<()> {
        while self.state.is_running {
            self.draw()?;
            if let Some(action) = self.handle_input()? {
                let new_state = update(self.state.clone(), action);
                self.state = new_state;
                // In a full TEA, `update` might also return Commands (side effects) but I'm not
                // going to go into that now.
                // self.execute_commands(commands);
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let current_state = &mut self.state;
        self.terminal.draw(|frame| {
            render_ui(frame, current_state);
        })?;
        Ok(())
    }

    fn handle_input(&mut self) -> Result<Option<Action>> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    return Ok(key_event_to_action(key_event, &self.state.ctx)); // Pass current context
                }
                Event::Resize(cols, rows) => {
                    return Ok(Some(Action::Resize(Size::new(cols, rows))));
                }
                _ => {}
            }
        }
        Ok(None)
    }
}

// --- Update Function ---
// This function takes the current state and an action, and returns the new state.
// It should be a pure function: no side effects, only transforms state.
fn update(mut current_state: AppState, action: Action) -> AppState {
    let determine_ctx_after_modal_close_pure = |state: &AppState| -> Context {
        if let Some(prev_ctx) = &state.previous_ctx {
            prev_ctx.clone()
        } else {
            match state.screen {
                Screen::Main(_) => Context::Explorer,
                Screen::Start(_) => Context::StartScreen,
            }
        }
    };

    match action {
        Action::Quit => {
            current_state.is_running = false;
        }
        // Action::Resize(new_size) => {
        //     current_state.size = new_size;
        //     // Propagate size change to relevant states if they need it (e.g., for layout)
        //     // This might be better handled in the view or specific component states if they cache size-dependent things.
        //     match &mut current_state.screen {
        //         Screen::Start(start_view) => start_view.start_state.resize(new_size),
        //         Screen::Main(main_view) => { /* main_view.resize(new_size) if needed */ }
        //     }
        // }
        Action::ToggleHelp => {
            if current_state.ctx == Context::HelpModal {
                current_state.ctx = determine_ctx_after_modal_close_pure(&current_state);
                current_state.previous_ctx = None;
                current_state.help_modal_state = None;
            } else {
                current_state.previous_ctx = Some(current_state.ctx.clone());
                current_state.ctx = Context::HelpModal;
                let help_text_content = help_text();
                current_state.help_modal_state =
                    Some(HelpModalState::new(help_text_content.lines().count()));
            }
        }
        Action::ToggleVaultSelector => {
            if current_state.ctx == Context::VaultSelector {
                current_state.ctx = determine_ctx_after_modal_close_pure(&current_state);
                current_state.previous_ctx = None;
                current_state.vault_selector_state = None;
            } else {
                current_state.previous_ctx = Some(current_state.ctx.clone());
                current_state.ctx = Context::VaultSelector;
                current_state.vault_selector_state =
                    Some(VaultSelectorState::new(current_state.vaults.clone()));
            }
        }
        _ => match current_state.ctx {
            Context::HelpModal => {
                current_state = update_help_modal(current_state, action);
            }
            Context::VaultSelector => {
                current_state = update_vault_selector(current_state, action);
            }
            Context::StartScreen => {
                current_state = update_start_screen(current_state, action);
            }
            Context::Explorer => {
                current_state = update_explorer(current_state, action);
            }
            Context::Editor => {
                current_state = update_editor(current_state, action);
            }
        },
    }
    current_state
}

fn update_help_modal(mut state: AppState, action: Action) -> AppState {
    if let Some(modal_state) = state.help_modal_state.as_mut() {
        match action {
            Action::ScrollUp(amount) => {
                modal_state.scroll_up(calc_scroll_amount(amount, state.size))
            }
            Action::ScrollDown(amount) => {
                modal_state.scroll_down(calc_scroll_amount(amount, state.size))
            }
            Action::Next => modal_state.scroll_down(1),
            Action::Prev => modal_state.scroll_up(1),
            _ => {}
        }
    }
    state
}

fn update_vault_selector(mut state: AppState, action: Action) -> AppState {
    let mut close_modal_and_revert_ctx = false;
    let mut new_ctx_after_selection: Option<Context> = None;

    if let Some(modal_state) = state.vault_selector_state.as_mut() {
        match action {
            Action::Select => {
                modal_state.select();
                if let Some(index) = modal_state.selected() {
                    if let Some(vault) = modal_state.get_item(index) {
                        let entries = vault.load().unwrap_or_default();
                        let size = state.size;
                        match MainView::new(&vault, entries, size) {
                            Ok(main_view) => {
                                state.screen = Screen::Main(main_view);
                                state.selected_vault = Some(&vault);
                                new_ctx_after_selection = Some(Context::Explorer);
                                close_modal_and_revert_ctx = true;
                            }
                            Err(_) => {
                                close_modal_and_revert_ctx = true;
                            }
                        }
                    }
                }
            }
            Action::Next => modal_state.next(),
            Action::Prev => modal_state.previous(),
            _ => {}
        }
    }

    if close_modal_and_revert_ctx {
        state.vault_selector_state = None;
        state.ctx = new_ctx_after_selection.unwrap_or_else(|| {
            let determine_ctx_after_modal_close_pure = |s: &AppState| -> Context {
                if let Some(prev_ctx) = &s.previous_ctx {
                    prev_ctx.clone()
                } else {
                    match s.screen {
                        Screen::Main(_) => Context::Explorer,
                        Screen::Start(_) => Context::StartScreen,
                    }
                }
            };
            determine_ctx_after_modal_close_pure(&state)
        });
        state.previous_ctx = None;
    }
    state
}

fn update_start_screen(mut state: AppState, action: Action) -> AppState {
    if let Screen::Start(start_view) = &mut state.screen {
        match action {
            Action::Select => {
                start_view.start_state.select();
                if let Some(index) = start_view.start_state.selected() {
                    if let Some(vault) = start_view.start_state.get_item(index) {
                        let entries = vault.load().unwrap_or_default();
                        let size = state.size;
                        if let Ok(main_view) = MainView::new(&vault, entries, size) {
                            state.screen = Screen::Main(main_view);
                            state.selected_vault = Some(&vault);
                            state.ctx = Context::Explorer;
                        }
                    }
                }
            }
            Action::Next => start_view.start_state.next(),
            Action::Prev => start_view.start_state.previous(),
            _ => {}
        }
    }
    state
}

fn update_explorer(mut state: AppState, action: Action) -> AppState {
    if let Screen::Main(main_view) = &mut state.screen {
        match action {
            Action::ToggleMode => {
                state.ctx = Context::Editor;
                main_view.mode = Mode::Normal;
            }
            Action::Select => {
                let selected_path = main_view.explorer_state.selected().last();
                if let Some(path) = selected_path {
                    if path.is_dir() {
                        main_view.explorer_state.toggle_selected();
                    } else {
                        // Perform side effect (file reading) conceptually as a Command later.
                        // For now, I'm doing it like this, but it breaks "purity".
                        // To maintain that purity, Action::SelectOnFile(path) would be dispatched,
                        // and a command handler would read the file and dispatch Action::FileContentLoaded(content).
                        if let Ok(content) = fs::read_to_string(&path) {
                            main_view.markdown_view_state.set_text(content);
                            main_view.markdown_view_state.reset_scrollbar();
                            state.ctx = Context::Editor;
                            main_view.mode = Mode::Normal;
                        }
                    }
                }
                main_view.explorer_state.scroll_selected_into_view();
            }
            Action::Next => {
                if main_view.explorer_state.select_next() {
                    main_view.explorer_state.scroll_selected_into_view();
                }
            }
            Action::Prev => {
                if main_view.explorer_state.select_prev() {
                    main_view.explorer_state.scroll_selected_into_view();
                }
            }
            _ => {}
        }
    }
    state
}

fn update_editor(mut state: AppState, action: Action) -> AppState {
    if let Screen::Main(main_view) = &mut state.screen {
        match action {
            Action::ToggleMode => {
                state.ctx = Context::Explorer;
                main_view.mode = Mode::Select;
            }
            Action::ScrollUp(amount) => {
                main_view
                    .markdown_view_state
                    .scroll_up(calc_scroll_amount(amount, state.size));
            }
            Action::ScrollDown(amount) => {
                main_view
                    .markdown_view_state
                    .scroll_down(calc_scroll_amount(amount, state.size));
            }
            Action::Next => main_view.markdown_view_state.scroll_down(1),
            Action::Prev => main_view.markdown_view_state.scroll_up(1),
            _ => {}
        }
    }
    state
}

// --- Pure View Function (render_ui) ---
// Takes an immutable reference to state.
// TEA prefers view functions to be pure. If widgets mutate state for rendering, it's a slight deviation.
// Often, render-specific mutable state (like scroll position) is updated in the `update` function.
fn render_ui<'a>(frame: &mut ratatui::Frame<'_>, state: &mut AppState<'a>) {
    let area = frame.area();

    // Render current screen
    match &mut state.screen {
        Screen::Start(start_view) => {
            let mut start_view_state_for_render = &mut start_view.start_state;
            StartView::default().render_ref(
                area,
                frame.buffer_mut(),
                &mut start_view_state_for_render,
            );
        }
        Screen::Main(main_view) => {
            let [content_area, statusbar_area] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

            let explorer_constraint =
                if state.ctx == Context::Explorer || main_view.mode == Mode::Select {
                    Constraint::Length(35)
                } else {
                    Constraint::Length(5)
                };
            let [explorer_area, note_area] =
                Layout::horizontal([explorer_constraint, Constraint::Fill(1)]).areas(content_area);

            main_view.explorer.render_ref(
                explorer_area,
                frame.buffer_mut(),
                &mut main_view.explorer_state,
            );

            let mut md_view_state_for_render = main_view.markdown_view_state.clone();

            MarkdownView.render_ref(note_area, frame.buffer_mut(), &mut md_view_state_for_render);

            if let Some(selected_vault) = &state.selected_vault {
                let name = selected_vault.name.as_str();
                let (word_count, char_count) = main_view.explorer_state.selected().last().map_or(
                    (WordCount::from(0), CharCount::from(0)),
                    |selected_path| {
                        if selected_path.is_dir() {
                            (WordCount::from(0), CharCount::from(0))
                        } else {
                            fs::read_to_string(selected_path).map_or_else(
                                |_| (WordCount::from(0), CharCount::from(0)),
                                |content| {
                                    (
                                        WordCount::from(content.as_str()),
                                        CharCount::from(content.as_str()),
                                    )
                                },
                            )
                        }
                    },
                );

                let mode_str = main_view.mode.as_str().to_uppercase();
                let mut status_bar_state_for_render = StatusBarState::new(
                    &mode_str,
                    Some(name),
                    word_count.into(),
                    char_count.into(),
                );
                StatusBar::default().render_ref(
                    statusbar_area,
                    frame.buffer_mut(),
                    &mut status_bar_state_for_render,
                );
            }
        }
    }

    // Render modals on top
    match state.ctx {
        Context::HelpModal => {
            if let Some(modal_state_data) = &state.help_modal_state {
                let mut modal_state_for_render = modal_state_data.clone();
                Help::new(help_text().as_str()).render(
                    area,
                    frame.buffer_mut(),
                    &mut modal_state_for_render,
                );
            }
        }
        Context::VaultSelector => {
            if let Some(modal_state_data) = &state.vault_selector_state {
                let mut modal_state_for_render = modal_state_data.clone();
                VaultSelector::default().render_ref(
                    area,
                    frame.buffer_mut(),
                    &mut modal_state_for_render,
                );
            }
        }
        _ => {}
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollAmount {
    #[default]
    One,
    HalfPage,
}

fn calc_scroll_amount(scroll_amount: ScrollAmount, size: Size) -> usize {
    match scroll_amount {
        ScrollAmount::One => 1,
        ScrollAmount::HalfPage => (size.height / 3).max(1) as usize,
    }
}

fn help_text() -> String {
    let version = format!("{VERSION}~alpha");
    HELP_TEXT.replace(
        "%version-notice",
        format!("This is the read-only release of Basalt ({version})").as_str(),
    )
}

// I'll try to implement something like this after I've made more progress on the explorer
// pub enum Command {
//     SaveFile(String, String), // path, content
//     ReadFile(String),         // path
// }
