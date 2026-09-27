Key mappings can be modified or extended by defining them in the [[Configuration|configuration file]].

Each key mapping is associated with a specific pane and becomes active when that pane has focus. The `global` section applies to all panes and is evaluated first.

```toml
[global]
key_bindings = [
  { key = "q", command = "quit" },
  { key = "?", command = "help_modal_toggle" },
]

[explorer]
key_bindings = [
  { key = "k", command = "explorer_up" },
  { key = "j", command = "explorer_down" },
]
```

## Key sequence syntax

A key can be a single character, a named key, a modified key or a **sequence** of keystrokes. Sequences are written as a multi-character string and only fire when all keys are pressed in order with nothing in between. This makes it possible to define vim- or Helix-style bindings like `gg`.

```toml
[note_editor]
key_bindings = [
  { key = "gg", command = "note_editor_scroll_to_top" },
  { key = "G",  command = "note_editor_scroll_to_bottom" },
]
```

An uppercase letter like `G` is shorthand for `shift+g`. Shift is implied automatically.

### Named keys in sequences

A multi-character string splits into one keystroke per character, so a named
key like `space` or `enter` cannot start a sequence directly (`<space>f` would
otherwise read as the keys `s p a c e ...`). Wrap a named or modified key in
`<...>` to group it into a single keystroke. This enables Helix-style leader
bindings:

```toml
[global]
key_bindings = [
  { key = "<space>f",       command = "vault_selector_modal_toggle" },
  { key = "<space><space>", command = "help_modal_toggle" },
  { key = "<ctrl+w>q",      command = "quit" },
]
```

`<` and `>` are reserved for this syntax; bind the literal keys with `<lt>` and
`<gt>`.

## Leader key

`<leader>` is a placeholder for a prefix key of your choosing, so a whole set of
bindings can be moved to a different prefix by changing one line. It defaults to
`<space>` and is set at the top level of the configuration file:

```toml
leader = ","

[global]
key_bindings = [
  { key = "<leader>f", command = "explorer_toggle" },
]
```

The leader applies to the defaults as well as to your own bindings, so the above
turns `Space v` into `,v`, `Space d` into `,d`, and binds `,f` on top.

| Default binding | Command                       |
| --------------- | ----------------------------- |
| `<leader>s`     | Toggle full-text search       |
| `<leader>v`     | Toggle vault selector modal   |
| `<leader>d`     | Toggle debug log overlay      |
| `<leader>e`     | Open the note in `vi`         |
| `<leader>o`     | Open the note in Obsidian     |

The leader may be any key, including a modified one (`leader = "ctrl+w"`) or a
sequence (`leader = "gs"`), and it may appear more than once in a binding
(`<leader><leader>`). It cannot refer to itself: `leader = "<leader>"` is an
error.

## Available commands

### Global commands

| Command                       | Description                          |
| ----------------------------- | ------------------------------------ |
| `quit`                        | Exit the application                 |
| `search_toggle`               | Toggle full-text search              |
| `vault_selector_modal_toggle` | Toggle vault selector modal          |
| `help_modal_toggle`           | Toggle help modal                    |
| `tab_next`                    | Focus the next open note tab         |
| `tab_previous`                | Focus the previous open note tab     |
| `tab_close`                   | Close the active note tab            |

### Splash commands

| Command        | Description              |
| -------------- | ------------------------ |
| `splash_up`    | Move selector up         |
| `splash_down`  | Move selector down       |
| `splash_open`  | Open the selected vault  |

### Explorer commands

| Command                          | Description                                    |
| -------------------------------- | ---------------------------------------------- |
| `explorer_up`                    | Move selector up                               |
| `explorer_down`                  | Move selector down                             |
| `explorer_open`                  | Open selected note in note editor              |
| `explorer_sort`                  | Toggle sort between A-z and Z-a                |
| `explorer_toggle`                | Toggle explorer pane                           |
| `explorer_toggle_outline`        | Toggle outline pane                            |
| `explorer_toggle_input_rename`   | Open rename dialog for selected item           |
| `explorer_new_untitled_note`     | Create a new untitled note                     |
| `explorer_new_untitled_folder`   | Create a new untitled folder                   |
| `explorer_hide_pane`             | Hide pane (stepped)                            |
| `explorer_expand_pane`           | Expand pane (stepped)                          |
| `explorer_switch_pane_next`      | Switch focus to next pane                      |
| `explorer_switch_pane_previous`  | Switch focus to previous pane                  |
| `explorer_scroll_up_one`         | Scroll selector up by one                      |
| `explorer_scroll_down_one`       | Scroll selector down by one                    |
| `explorer_scroll_up_half_page`   | Scroll selector up half a page                 |
| `explorer_scroll_down_half_page` | Scroll selector down half a page               |
| `explorer_scroll_to_top`         | Jump to the first item                         |
| `explorer_scroll_to_bottom`      | Jump to the last item                          |

### Outline commands

| Command                         | Description                                     |
| ------------------------------- | ----------------------------------------------- |
| `outline_up`                    | Move selector up                                |
| `outline_down`                  | Move selector down                              |
| `outline_toggle`                | Toggle outline pane                             |
| `outline_toggle_explorer`       | Toggle explorer pane                            |
| `outline_switch_pane_next`      | Switch focus to next pane                       |
| `outline_switch_pane_previous`  | Switch focus to previous pane                   |
| `outline_expand`                | Expand or collapse heading                      |
| `outline_select`                | Jump to heading in editor                       |

### Note editor commands

| Command                                | Description                         |
| -------------------------------------- | ----------------------------------- |
| `note_editor_cursor_up`                | Move cursor up                      |
| `note_editor_cursor_down`              | Move cursor down                    |
| `note_editor_scroll_up_one`            | Scroll up by one                    |
| `note_editor_scroll_down_one`          | Scroll down by one                  |
| `note_editor_scroll_up_half_page`      | Scroll up half page                 |
| `note_editor_scroll_down_half_page`    | Scroll down half page               |
| `note_editor_scroll_to_top`            | Jump to the top of the note         |
| `note_editor_scroll_to_bottom`         | Jump to the bottom of the note      |
| `note_editor_toggle_explorer`          | Toggle explorer pane                |
| `note_editor_toggle_outline`           | Toggle outline pane                 |
| `note_editor_switch_pane_next`         | Switch focus to next pane           |
| `note_editor_switch_pane_previous`     | Switch focus to previous pane       |
| `note_editor_cursor_screen_up`         | Move cursor up one visible row      |
| `note_editor_cursor_screen_down`       | Move cursor down one visible row    |
| `note_editor_follow_link`              | Follow the link under the cursor    |

`note_editor_cursor_up` and `note_editor_cursor_down` move by whole line, so a
soft-wrapped line counts once. `note_editor_cursor_screen_up` and
`note_editor_cursor_screen_down` move by visible row instead, which matches vim
`gk` and `gj`.

`note_editor_follow_link` travels to the note a wiki-link names and creates
that note when it does not exist. On a plain URL it opens your browser.

### Experimental editor commands

| Command                                          | Description                    |
| ------------------------------------------------ | ------------------------------ |
| `note_editor_experimental_set_edit_view`          | Switch to edit view            |
| `note_editor_experimental_toggle_view`            | Toggle between edit and read   |
| `note_editor_experimental_set_read_view`          | Switch to read view            |
| `note_editor_experimental_save`                   | Save note changes              |
| `note_editor_experimental_exit`                   | Cancel editing, switch to read |
| `note_editor_experimental_cursor_left`            | Move cursor left               |
| `note_editor_experimental_cursor_right`           | Move cursor right              |
| `note_editor_experimental_cursor_word_forward`    | Move cursor forward by word    |
| `note_editor_experimental_cursor_word_backward`   | Move cursor backward by word   |
| `note_editor_insert_mode`                         | Enter insert mode              |
| `note_editor_append`                              | Enter insert mode after the cursor |
| `note_editor_visual_mode`                         | Start a charwise selection     |
| `note_editor_visual_line_mode`                    | Start a linewise selection     |

`note_editor_experimental_set_edit_mode`, `note_editor_experimental_set_read_mode`
and `note_editor_experimental_exit_mode` are deprecated aliases of the `_view` and
`_exit` commands above. They still work but will be removed in the next major
version.

### Vim mode motions

These commands need `vim_mode = true` in `[note_editor]`. They act in Normal
mode. The key in brackets is the default binding from the vim preset.

| Command                                | Description                                       |
| -------------------------------------- | ------------------------------------------------- |
| `note_editor_cursor_line_start`        | Move to the start of the line (`0`)               |
| `note_editor_cursor_first_non_blank`   | Move to the first non-blank character (`^`)       |
| `note_editor_cursor_line_end`          | Move to the end of the line (`$`)                 |
| `note_editor_cursor_doc_start`         | Move to the start of the note (`gg`)              |
| `note_editor_cursor_doc_end`           | Move to the end of the note (`G`)                 |
| `note_editor_cursor_word_end`          | Move to the end of the word (`e`)                 |
| `note_editor_cursor_word_forward_big`  | Move forward by whitespace-delimited word (`W`)   |
| `note_editor_cursor_word_backward_big` | Move backward by whitespace-delimited word (`B`)  |
| `note_editor_cursor_word_end_big`      | Move to the end of the whitespace-delimited word (`E`) |
| `note_editor_paragraph_forward`        | Move to the next paragraph (`}`)                  |
| `note_editor_paragraph_backward`       | Move to the previous paragraph (`{`)              |
| `note_editor_matching_pair`            | Jump to the matching bracket (`%`)                |
| `note_editor_find_forward`             | Jump forward to the next typed character (`f`)    |
| `note_editor_find_backward`            | Jump backward to the next typed character (`F`)   |
| `note_editor_till_forward`             | Jump forward, stopping before the character (`t`) |
| `note_editor_till_backward`            | Jump backward, stopping after the character (`T`) |
| `note_editor_repeat_find`              | Repeat the last find (`;`)                        |
| `note_editor_repeat_find_reverse`      | Repeat the last find in reverse (`,`)             |

### Vim mode operators and edits

| Command                              | Description                                          |
| ------------------------------------ | ---------------------------------------------------- |
| `note_editor_delete`                 | Delete operator; takes a motion (`d`)                |
| `note_editor_change`                 | Change operator; deletes then enters insert (`c`)    |
| `note_editor_yank`                   | Yank operator; copies to the clipboard (`y`)         |
| `note_editor_delete_under_cursor`    | Delete the character under the cursor (`x`)          |
| `note_editor_delete_to_line_end`     | Delete to the end of the line (`D`)                  |
| `note_editor_change_to_line_end`     | Change to the end of the line (`C`)                  |
| `note_editor_substitute_char`        | Delete the character and enter insert mode (`s`)     |
| `note_editor_replace_char`           | Replace the character with the next one typed (`r`)  |
| `note_editor_paste_after`            | Paste after the cursor (`p`)                         |
| `note_editor_paste_before`           | Paste before the cursor (`P`)                        |
| `note_editor_undo`                   | Undo the last change (`u`)                           |
| `note_editor_redo`                   | Redo the undone change (`Ctrl+R`)                    |

### Theme selector modal commands

| Command                        | Description                                        |
| ------------------------------ | -------------------------------------------------- |
| `theme_selector_modal_toggle`  | Toggle the theme picker                            |
| `theme_selector_modal_up`      | Move selector up and preview the theme             |
| `theme_selector_modal_down`    | Move selector down and preview the theme           |
| `theme_selector_modal_open`    | Keep the highlighted theme and close the picker    |
| `theme_selector_modal_close`   | Close the picker and revert to the previous theme  |

### Debug log commands

| Command                             | Description                                     |
| ----------------------------------- | ----------------------------------------------- |
| `debug_log_toggle`                  | Toggle the debug log overlay                    |
| `debug_log_close`                   | Close the debug log overlay                     |
| `debug_log_clear`                   | Clear all captured log entries                  |
| `debug_log_cycle_level`             | Cycle the minimum visible level (trace → error) |
| `debug_log_scroll_up_one`           | Scroll up by one                                |
| `debug_log_scroll_down_one`         | Scroll down by one                              |
| `debug_log_scroll_up_half_page`     | Scroll up half page                             |
| `debug_log_scroll_down_half_page`   | Scroll down half page                           |

### Input modal commands

| Command                    | Description                     |
| -------------------------- | ------------------------------- |
| `input_modal_edit_mode`    | Enter edit mode for typing      |
| `input_modal_accept`       | Accept changes and close modal  |
| `input_modal_cancel`       | Cancel and close modal          |
| `input_modal_left`         | Move cursor left                |
| `input_modal_right`        | Move cursor right               |
| `input_modal_word_forward` | Move cursor forward by word     |
| `input_modal_word_backward`| Move cursor backward by word    |

### Help modal commands

| Command                           | Description              |
| --------------------------------- | ------------------------ |
| `help_modal_toggle`               | Toggle help modal        |
| `help_modal_close`                | Close help modal         |
| `help_modal_scroll_up_one`        | Scroll up by one         |
| `help_modal_scroll_down_one`      | Scroll down by one       |
| `help_modal_scroll_up_half_page`  | Scroll up half page      |
| `help_modal_scroll_down_half_page`| Scroll down half page    |

### Vault selector modal commands

| Command                        | Description                     |
| ------------------------------ | ------------------------------- |
| `vault_selector_modal_up`      | Move selector up                |
| `vault_selector_modal_down`    | Move selector down              |
| `vault_selector_modal_close`   | Close vault selector modal      |
| `vault_selector_modal_open`    | Open selected vault             |
| `vault_selector_modal_toggle`  | Toggle vault selector modal     |
