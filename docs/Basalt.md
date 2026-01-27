[[Basalt]] is a cross-platform TUI (Terminal User Interface) for managing Obsidian vaults and notes. It runs on Windows, macOS, and Linux.

[[Basalt]] is not a replacement for Obsidian. Instead, it provides a minimalist terminal interface with readable markdown rendering and a [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) experience.

## Vision

- [[Basalt]] functions as a companion app for [[Obsidian]] that enables quick note editing without interrupting the terminal flow
- Basalt enables text editing in a familiar way ([[Obsidian]], [[vim]]) without having to rely on external editors
- [[Basalt]] is a terminal based [WYSIWYG](https://en.wikipedia.org/wiki/WYSIWYG) markdown editor
- [[Basalt]] works as a CLI for finding / deleting / creating notes and works with the rest of the unix tooling
- [[Basalt]] is a standalone terminal note managing application that works with Obsidian

## Background

[[Basalt]] was created to bridge the gap between terminal-based workflows and Obsidian's visual note-taking experience. While tools like [obsidian.nvim](https://github.com/epwalsh/obsidian.nvim) provide Obsidian integration within Neovim, Basalt takes a different approach: a dedicated TUI that renders markdown visually while keeping you in the terminal.

The goal is not to replace Obsidian, but to complement it. Basalt provides quick access to your notes from anywhere in the terminal without breaking your workflow.

## Architecture

[[Basalt]] follows an **Elm-inspired architecture**, a functional programming pattern that emphasizes unidirectional data flow and explicit state management. This architecture consists of three core concepts:

### Model (State)

The application state is represented by immutable data structures. The central `AppState` holds all application state, with each UI component (Explorer, Note Editor, Outline, etc.) maintaining its own sub-state. State is never mutated directly—instead, changes flow through the update cycle.

### Messages

User actions and events are represented as typed **messages**. Each component defines its own message enum (e.g., `explorer::Message::Select`, `note_editor::Message::CursorUp`). Messages describe _what happened_, not _how to handle it_.

### Update

The **update** function is the heart of the architecture. It takes the current state and a message, then returns the new state (and optionally a new message to process). This creates a predictable cycle:

```
Event → Message → Update → State → Render → Event
```

Messages can cascade: when one component's update returns a message for another component, the cycle continues until no new messages are produced.

### Properties

This architecture has the following properties:

- **Predictability**: Given the same state and message, the update function always produces the same result
- **Traceability**: All state changes flow through explicit message passing
- **Testability**: Update functions are pure and can be tested without rendering the UI
- **Modularity**: Components are isolated and communicate only through messages

### Crate Structure

Basalt is organized as a Rust workspace with three crates:

| Crate | Purpose |
|-------|---------|
| `basalt-core` | Domain logic and Obsidian integration (vault, note, markdown parsing). No UI dependencies. |
| `basalt-widgets` | Reusable [ratatui](https://ratatui.rs) widgets for the TUI |
| `basalt` (basalt-tui) | The main TUI application, combining core logic with the user interface |
