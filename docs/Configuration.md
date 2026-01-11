[[Basalt]] features and functionality can be customized using a user-defined configuration file. The configuration file should be located in one of the following directories:

**macOS and Unix:**

- `$HOME/.basalt.toml`
- `$XDG_CONFIG_HOME/basalt/config.toml`

**Windows:**

- `%USERPROFILE%\.basalt.toml`
- `%APPDATA%\basalt\config.toml`

If configuration files exist in multiple locations, only the first one found will be used, with the home directory configuration taking precedence. 

> [!WARNING]
>
> This behavior may change in future versions to merge all found configurations instead.

## Key Mappings

Basalt key mappings can be modified or extended by defining key mappings in the user configuration file.

Each key mapping is associated with a specific 'pane' and becomes active when that pane has focus. The global section applies to all panes and is evaluated first.

Key bindings support both built-in Basalt commands and custom arbitrary command execution, allowing you to integrate external applications and create automation workflows.

## Custom Command Execution

In addition to built-in commands, you can execute arbitrary external commands using special command prefixes:

```toml
[global]
key_bindings = [
  { key = "q", command = "quit" },  # Built-in command
  { key = "ctrl+alt+e", command = "spawn:open obsidian://daily?vault=%vault" },  # Custom spawn command
  { key = "ctrl+g", command = "exec:nvim %note" },  # Custom exec command
]
```

### Command Types

#### Built-in Commands

These are Basalt's native commands that control application behavior (see full list in the Default Configuration section below).

#### Execute Command - `exec:`

Runs a command in the current shell environment. The command will block until completion. Only the first argument is treated as the executable command, with all remaining arguments passed as literal parameters.

```toml
key_bindings = [
  { key = "ctrl+alt+e", command = "exec:nvim %note_path" },
]
```

#### Spawn Process - `spawn:`

Spawns a new process without blocking. This is for opening external applications or URLs. Like `exec:`, only the first argument is the executable, with remaining arguments passed as literal parameters.

```toml
key_bindings = [
  { key = "ctrl+o", command = "spawn:open %note" },
  { key = "ctrl+b", command = "spawn:open https://example.com" },
]
```

## Variables

Basalt provides special variables that are dynamically replaced with current context information:

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `%vault` | Current vault name | `my-notes` |
| `%note` | Current note name | `My Note` |
| `%note_path` | Current note file path | `/path/to/vault/daily/2024-01-15.md` |

Variables can be used anywhere within the command string and will be replaced at runtime.

## Integration Examples

### Obsidian Integration

Obsidian supports URL schemes that start with `obsidian://`, allowing you to trigger various actions within the application:

```toml
# Open daily note in Obsidian
key_bindings = [
  { key = "ctrl+d", command = "spawn:open obsidian://daily?vault=%vault" },
]

# Open specific note in Obsidian
key_bindings = [
  { key = "ctrl+alt+e", command = "spawn:open obsidian://open?vault=%vault&file=%note" },
]

# Create new note in Obsidian
key_bindings = [
  { key = "ctrl+n", command = "spawn:open obsidian://new?vault=%vault&name=New Note" },
]

# Open specific path in Obsidian
key_bindings = [
  { key = "ctrl+p", command = "spawn:open obsidian://open?path=%note_path" },
]
```

Common Obsidian URI actions include:
- `obsidian://open` - Open a specific note
- `obsidian://new` - Create a new note
- `obsidian://daily` - Open daily note
- `obsidian://search` - Perform a search

For more information about Obsidian URI schemes, see the [official Obsidian URI documentation](https://help.obsidian.md/Extending+Obsidian/Obsidian+URI).

### External Editor Integration

```toml
# Open current note in VS Code
key_bindings = [
  { key = "ctrl+c", command = "spawn:code %note" },
]

# Open current note in vim
key_bindings = [
  { key = "ctrl+v", command = "exec:vim %note" },
]

# Open current note in nano
key_bindings = [
  { key = "ctrl+n", command = "exec:nano %note" },
]
```

### Web Integration

```toml
# Open specific URLs
key_bindings = [
  { key = "ctrl+g", command = "spawn:open https://github.com/user/repo" },
]

# Search engines - construct URLs with variables
key_bindings = [
  { key = "ctrl+s", command = "spawn:open https://www.google.com/search?q=%note" },
]
```

## Platform Considerations

- **macOS**: Use `cmd` instead of `ctrl` for standard shortcuts, and `open` command for launching applications
- **Linux**: Use `xdg-open` for opening files/URLs with default applications
- **Windows**: Use `start` command for opening files/URLs

## Tips

1. **Use `spawn:` for non-blocking operations** like opening applications or URLs
2. **Use `exec:` for quick commands** that should complete before continuing
3. **Test your key bindings** to ensure they don't conflict with existing Basalt shortcuts
4. **Command limitations** - only the first argument is the executable, no shell expansion or piping
5. **Use full paths** if executables aren't in your PATH
6. **Variables work in arguments** - `%vault`, `%note` and `%note_path` are expanded in argument positions

## Troubleshooting

- If a key binding doesn't work, check that the key combination isn't already used by Basalt or your system
- Ensure external commands are available in your PATH
- Use absolute paths for commands if they're not found
- Check that variables like `%vault`, `%note` and `%note_path` are being populated correctly in your context
- Shell features like pipes (`|`), redirects (`>`), and command substitution (`$(...)`) are not supported
- Complex operations requiring shell features should be wrapped in scripts that can be called as single commands

## Default configuration

```toml
# The corresponding pane needs to be _active_ in order for the keybindings to
# be read and the attached command activated.
#
# Global commands:
#
# quit: exits the application
# vault_selector_modal_toggle: toggles vault selector modal (not available in splash screen)
# help_modal_toggle: toggles help modal
# spawn: <command> spawns a new process without blocking. This is for opening external applications or URLs.
# exec: <command> runs a command in the current shell environment.
#
# Splash commands:
#
# splash_up: moves selector up
# splash_down: moves selector down
# splash_open: opens the selected vault
#
# Explorer commands:
#
# explorer_up: moves selector up
# explorer_down: moves selector down
# explorer_open: opens the selected note in note viewer
# explorer_sort: toggles note and folder sorting between A-z and Z-a
# explorer_toggle: toggles explorer pane
# explorer_toggle_outline: toggles outline pane
# explorer_toggle_input_rename: opens input modal to rename selected note or directory
# explorer_hide_pane: hides pane by steps, from full-width to regular width, to hidden
# explorer_expand_pane: explands pane by steps, from hiddent to regular width, to full-width
# explorer_switch_pane_next: switches focus to next pane
# explorer_switch_pane_previous: switches focus to previous pane
# explorer_scroll_up_one: scrolls the selector up by one
# explorer_scroll_down_one: scrolls the selector down by one
# explorer_scroll_up_half_page: scrolls the selector up half a page
# explorer_scroll_down_half_page: scrolls the selector down half a page
#
# Outline commands:
#
# outline_up: moves selector up
# outline_down: moves selector down
# outline_toggle: toggles outline panel
# outline_toggle_explorer: toggles explorer pane
# outline_switch_pane_next: switches focus to next pane
# outline_switch_pane_previous: switches focus to previous pane
# outline_expand: expands or collapses headings
# outline_select: select heading and move note editor cursor to heading location
#
# Note editor commands:
#
# note_editor_scroll_up_one: scrolls up by one
# note_editor_scroll_down_one: scrolls down by one
# note_editor_scroll_up_half_page: scrolls up by half page
# note_editor_scroll_down_half_page: scrolls down by half page
# note_editor_toggle_explorer: toggles explorer pane
# note_editor_toggle_outline: toggles outline pane
# note_editor_switch_pane_next: switches focus to next pane
# note_editor_switch_pane_previous: switches focus to previous pane
#
# Experimental note editor commands:
# note_editor_experimental_set_edit_view: changes to edit view
# note_editor_experimental_toggle_view: toggles between edit and read views
# note_editor_experimental_set_read_view: changes to read view
# note_editor_experimental_save: saves note changes to file
# note_editor_experimental_exit: cancels editing and changes to read
# note_editor_experimental_cursor_left: moves cursor left
# note_editor_experimental_cursor_right: moves cursor right
# note_editor_experimental_cursor_word_forward: moves cursor forward by a word
# note_editor_experimental_cursor_word_backward: moves cursor backwards by a word
#
# Input modal commands:
#
# input_modal_edit_mode: enter edit mode for typing
# input_modal_accept: accept changes and close modal
# input_modal_cancel: cancel and close modal without saving
# input_modal_left: move cursor left
# input_modal_right: move cursor right
# input_modal_word_forward: move cursor forward by word
# input_modal_word_backward: move cursor backward by word
#
# Help modal commands:
#
# help_modal_toggle: toggles help modal
# help_modal_close: closes help modal
# help_modal_scroll_up_one: scrolls up by one
# help_modal_scroll_down_one: scrolls down by one
# help_modal_scroll_up_half_page: scrolls up by half page
# help_modal_scroll_down_half_page: scrolls down by half page
#
# Vault selector modal commands:
#
# vault_selector_modal_up: moves selector up
# vault_selector_modal_down: moves selector down
# vault_selector_modal_close: closes vault selector modal
# vault_selector_modal_open: opens the selected vault 
# vault_selector_modal_toggle: toggles vault selector modal

# Editor is experimental
experimental_editor = false

[global]
key_bindings = [
 { key = "q", command = "quit" },
 { key = "ctrl+g", command = "vault_selector_modal_toggle" },
 { key = "?", command = "help_modal_toggle" },
 { key = "ctrl+alt+e", command = "exec:vi %note_path" },
 { key = "ctrl+alt+o", command = "spawn:open obsidian://open?vault=%vault&file=%note" },
]

[splash]
key_bindings = [
 { key = "k", command = "splash_up" },
 { key = "j", command = "splash_down" },
 { key = "up", command = "splash_up" },
 { key = "down", command = "splash_down" },
 { key = "enter", command = "splash_open" },
]

[explorer]
key_bindings = [
 { key = "k", command = "explorer_up" },
 { key = "j", command = "explorer_down" },
 { key = "up", command = "explorer_up" },
 { key = "down", command = "explorer_down" },
 { key = "t", command = "explorer_toggle" },
 { key = "h", command = "explorer_hide_pane" },
 { key = "l", command = "explorer_expand_pane" },
 { key = "left", command = "explorer_hide_pane" },
 { key = "right", command = "explorer_expand_pane" },
 { key = "s", command = "explorer_sort" },
 { key = "r", command = "explorer_toggle_input_rename" },
 { key = "tab", command = "explorer_switch_pane_next" },
 { key = "shift+backtab", command = "explorer_switch_pane_previous" },
 { key = "enter", command = "explorer_open" },
 { key = "ctrl+b", command = "explorer_toggle" },
 { key = "ctrl+u", command = "explorer_scroll_up_half_page" },
 { key = "ctrl+d", command = "explorer_scroll_down_half_page" },
 { key = "ctrl+o", command = "explorer_toggle_outline" },
]

[outline]
key_bindings = [
 { key = "k", command = "outline_up" },
 { key = "j", command = "outline_down" },
 { key = "up", command = "outline_up" },
 { key = "down", command = "outline_down" },
 { key = "ctrl+o", command = "outline_toggle" },
 { key = "ctrl+b", command = "outline_toggle_explorer" },
 { key = "t", command = "outline_toggle_explorer" },
 { key = "tab", command = "outline_switch_pane_next" },
 { key = "shift+backtab", command = "outline_switch_pane_previous" },
 { key = "enter", command = "outline_expand" },
 { key = "g", command = "outline_select" },
]

[note_editor]
key_bindings = [
 { key = "k", command = "note_editor_cursor_up" },
 { key = "j", command = "note_editor_cursor_down" },
 { key = "up", command = "note_editor_cursor_up" },
 { key = "down", command = "note_editor_cursor_down" },
 { key = "t", command = "note_editor_toggle_explorer" },
 { key = "tab", command = "note_editor_switch_pane_next" },
 { key = "shift+backtab", command = "note_editor_switch_pane_previous" },
 { key = "ctrl+b", command = "note_editor_toggle_explorer" },
 { key = "ctrl+u", command = "note_editor_scroll_up_half_page" },
 { key = "ctrl+d", command = "note_editor_scroll_down_half_page" },
 { key = "ctrl+o", command = "note_editor_toggle_outline" },

 # Experimental editor 
 { key = "i", command = "note_editor_experimental_set_edit_view" },
 { key = "ctrl+e", command = "note_editor_experimental_toggle_view" },
 { key = "shift+r", command = "note_editor_experimental_set_read_view" },
 { key = "ctrl+x", command = "note_editor_experimental_save" },
 { key = "esc", command = "note_editor_experimental_exit" },
 { key = "h", command = "note_editor_experimental_cursor_left" },
 { key = "l", command = "note_editor_experimental_cursor_right" },
 { key = "left", command = "note_editor_experimental_cursor_left" },
 { key = "right", command = "note_editor_experimental_cursor_right" },
 # 'f' translates to arrow key right
 { key = "alt+f", command = "note_editor_experimental_cursor_word_forward" },
 # 'b' translates to arrow key left
 { key = "alt+b", command = "note_editor_experimental_cursor_word_backward" },
]

[input_modal]
key_bindings = [
 { key = "esc", command = "input_modal_cancel" },
 { key = "enter", command = "input_modal_accept" },
 { key = "i", command = "input_modal_edit_mode" },
 { key = "h", command = "input_modal_left" },
 { key = "l", command = "input_modal_right" },
 { key = "left", command = "input_modal_left" },
 { key = "right", command = "input_modal_right" },
 # 'f' translates to arrow key right
 { key = "alt+f", command = "input_modal_word_forward" },
 # 'b' translates to arrow key left
 { key = "alt+b", command = "input_modal_word_backward" },
]

[help_modal]
key_bindings = [
 { key = "esc", command = "help_modal_close" },
 { key = "k", command = "help_modal_scroll_up_one" },
 { key = "j", command = "help_modal_scroll_down_one" },
 { key = "up", command = "help_modal_scroll_up_one" },
 { key = "down", command = "help_modal_scroll_down_one" },
 { key = "ctrl+u", command = "help_modal_scroll_up_half_page" },
 { key = "ctrl+d", command = "help_modal_scroll_down_half_page" },
]

[vault_selector_modal]
key_bindings = [
 { key = "k", command = "vault_selector_modal_up" },
 { key = "j", command = "vault_selector_modal_down" },
 { key = "up", command = "vault_selector_modal_up" },
 { key = "down", command = "vault_selector_modal_down" },
 { key = "enter", command = "vault_selector_modal_open" },
 { key = "esc", command = "vault_selector_modal_close" },
]
```
