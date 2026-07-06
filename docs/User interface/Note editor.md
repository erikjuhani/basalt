The Note editor is the main pane in the center of the interface. It displays the selected note with rendered markdown, including headings, lists, code blocks and other elements.

![[note-editor.avif]]

Use `j`/`k` or arrow keys to scroll through the note. You can scroll faster with `Ctrl+U` and `Ctrl+D` for half-page jumps.

## Inline images

Images embedded in a note render inline, scaled to the note width. Basalt supports Obsidian `![[embed]]` syntax, standard `![alt](path)` links and remote `http(s)` URLs. An image renders partially as it scrolls through the viewport, and editing an embed updates the rendered image without reopening the note. GIFs render their first frame only for now.

![[image.avif]]

The maximum image height is a fraction of the viewport height, set with `image_max_height` in the config (default `1.0`). Raising it lets an image fill more of the width at the cost of more vertical space. Images need a terminal with graphics support (Kitty, iTerm2 or Sixel) and otherwise fall back to Unicode half-blocks.

Add `|width` or `|widthxheight` to size an image in pixels, matching Obsidian: `![[banner.png|200]]` or `![alt|200](path)`. Sizes are scaled for the display so they match Obsidian's on-screen size.

## Key mappings

| Mapping           | Description                          |
| ----------------- | ------------------------------------ |
| `j` / `↓`         | Move cursor down                     |
| `k` / `↑`         | Move cursor up                       |
| `t`               | Toggle explorer pane                 |
| `Tab`             | Switch to next pane                  |
| `Shift+Tab`       | Switch to previous pane              |
| `Ctrl+B`          | Toggle explorer pane                 |
| `Ctrl+O`          | Toggle outline pane                  |
| `Ctrl+U`          | Scroll up half page                  |
| `Ctrl+D`          | Scroll down half page                |
| `Ctrl+Shift+↑`    | Jump to top of note                  |
| `Ctrl+Shift+↓`    | Jump to bottom of note               |
| `gk` / `gj`       | Move up / down one visible row       |
| `Enter` / `gd`    | Follow the link under the cursor     |

`j` and `k` move by whole line, so a soft-wrapped line counts once. `gk` and `gj` move by visible row instead.

Following a wiki-link opens the note it names and creates that note when it does not exist. Following a plain URL opens your browser.

For text editing capabilities, see [[Editor (experimental)]].
