# Changelog

## [0.12.2](https://github.com/erikjuhani/basalt/releases/tag/basalt/0.12.2) (Unreleased)

### Fixed

- [5f93090](https://github.com/erikjuhani/basalt/commit/5f93090f2679eaa0785f56a398317cdc82629d13) Fix viewport not scrolling with cursor in edit mode by @erikjuhani

> Call `ensure_cursor_visible` after cursor movement and text editing
> operations (`insert_char`, `delete_char`, `cursor_left`, `cursor_right`,
> `cursor_word_forward`, `cursor_word_backward`) so the viewport follows
> the cursor. These calls were already present in `cursor_up` and
> `cursor_down` but missing from the other methods.
>
> Fixes #316

- [5fd1359](https://github.com/erikjuhani/basalt/commit/5fd1359843bf407a2d80b8035ac258794d2c71f3) Fix input modal position when viewport is scrolled by @erikjuhani

> Account for the list scroll offset when calculating the input modal
> y-position in `ToggleInputRename`. Previously, the position used the
> absolute selected index, which placed the modal outside the visible area
> when the list was scrolled.

- [a4ed28f](https://github.com/erikjuhani/basalt/commit/a4ed28f19898a64c39936a2a66c08a0734e39620) Disable smart punctuation by @erikjuhani

> Smart punctuation transforms characters like ' and " to curly variants
> like ‘ and “. The latter variants have different byte lengths. This had
> an effect that made the source offset not match with the rendered offset
> causing issues like inability to move the cursor downwards if the
> current paragraph contained characters that were transformed into
> 'smart' variants due to overlapping offsets it the length difference
> caused.
>
> The fix was to disable the smart punctuation, and come back to it at a
> later date and do the change holistically. This needs most likely some
> architectural change to allow smart punctuation to work. The smart
> punctuation should be treated as virtual variant that only has an effect
> in the rendering part of the text.
>
> This fixes: #371

- [2f83c49](https://github.com/erikjuhani/basalt/commit/2f83c492ebdfe089b0d0b638f30011c3b91d1d69) Fix editing when file is empty

> When opening an empty file and entering edit mode, no text or cursor was
> visible until pressing ESC. This was caused by three issues:
>
> - No AST nodes existed for empty files, so layout produced no virtual
>   lines and typed text was invisible
> - Cursor rendering was gated on non-empty content, which is only updated
>   on exit from edit mode
> - `render_raw` produced no content lines for empty content, leaving the
>   cursor with no valid position
>
> Fix by creating a placeholder paragraph node when entering insert mode
> on an empty document, allowing cursor rendering in edit mode, and
> producing a content line in `render_raw` for the empty content case.

## [0.12.1](https://github.com/erikjuhani/basalt/releases/tag/basalt/0.12.1) (Jan, 26 2026)

### Changed

- [3f81196](https://github.com/erikjuhani/basalt/commit/3f811966f359cb3ebd162478ebb5b95093e0afc7) Swap the sort symbol to a more common one by @erikjuhani

> The previous 𝌆 tetragram for centre symbol had multiple issues between
> different terminal emulators and recently the update of unicode-width
> that changed classification on some symbols making the width differ,
> from the previous version.
>
> Without this change I cannot update to newer version of unicode-width.

- [9716042](https://github.com/erikjuhani/basalt/commit/97160424a5186e9615ec2e8e40f12a68254ec24a) Remove ~beta suffix from version string

> Obsolete feature as the 0. major version should tell enough about the
> instability of this application.

### Fixed

- [001fe3e](https://github.com/erikjuhani/basalt/commit/001fe3e6d0cab7cb85bc77c588ad0c6de2691dab) Fix cursor movement for multi-byte unicode characters by @erikjuhani

> The cursor now correctly handles multi-byte characters (emojis, unicode
> symbols) when moving left/right and when calculating visual positions.
> Previously, the editor assumed 1 byte per character, causing the cursor
> to land in the middle of multi-byte sequences.
>
> Key changes:
>
> - Use byte lengths instead of character counts for source range tracking
> - Convert between byte offsets and character boundaries properly
> - Update `insert_char` and `delete_char` to account for variable byte
>   widths
> - Fix `source_offset_to_virtual_column` to use byte indices instead of
>   char indices
>
> Fixes #314

- [225184b](https://github.com/erikjuhani/basalt/commit/225184b9e69147162ae171836fe322f420be3014) Fix cursor movement through empty lines in code blocks by @erikjuhani

> The cursor could not move upwards past empty lines in code blocks in both
> edit and read modes.
>
> In edit mode, `virtual_position_to_source_offset` incorrectly returned
> `source_range.end` for empty lines because `cur_col` included synthetic
> span widths. Added `content_col` to track only content character widths.
>
> In read mode, the Visual rendering of code blocks didn't account for
> newlines when calculating source ranges, causing empty lines to have
> empty ranges (e.g., 5..5). Now uses `line_range()` which properly adds 1
> for newlines.
>
> Fixes #321

- [3f31151](https://github.com/erikjuhani/basalt/commit/3f31151e2566495b51353202d820f8c0bb4da970) Update all wiki-links in notes after rename

> Now all wiki-links will be updated after renaming a note in basalt.
> We call `update_wiki_links` in the RefreshVault message handler to
> automatically update links across the vault when a note is renamed.
>
> Also refreshed the note editor content after rename to reflect any
> wiki-link changes in the currently open note, otherwise the content
> would not be refreshed properly.
>
> Fixes #307

## [0.12.0](https://github.com/erikjuhani/basalt/releases/tag/basalt/0.12.0) (Jan, 18 2026)

### Added

- [342fbd6](https://github.com/erikjuhani/basalt/commit/342fbd6f9bd91e595aba95c7c0dd050873e5e205) Add input modal with note and directory rename functionality by @erikjuhani

> > [!CAUTION]
> > BEWARE! This rename implementation does not cover updating the
> > wiki-links. If you use the rename on notes that are referenced as
> > wiki-links—these links will be broken after and needs to be manually
> > corrected.
>
> Add an input modal that provides dynamic modal text editing. The modal
> features a text input widget and cursor navigation, which supports
> character-by-character and word-based movement. The vim-like modes are
> limited. Only aforementioned movement and text editing.
>
> The modal is integrated with the explorer pane, and is available by
> pressing 'r' (default key binding) on the selected item. The rename
> operation leverages the `rename_note` and `rename_dir` functions added
> in affec53 and a347325, the vault is 'reloaded' after rename.

### Dependencies

- [0b7e9ab](https://github.com/erikjuhani/basalt/commit/0b7e9ab2ac648a982de13c395b4d33a23a85bcd3) Update Rust crate ratatui to 0.30.0 by @renovate-updater[bot]

> | datasource | package | from   | to     |
> | ---------- | ------- | ------ | ------ |
> | crate      | ratatui | 0.29.0 | 0.30.0 |

### Fixed

- [5f9d342](https://github.com/erikjuhani/basalt/commit/5f9d34224a498ae3d7d010adf887fdcdb07317c9) Fix sorting to match Obsidian sorting

> Fixes #69. Previously, when user sorted the vault items the folders
> would also be sorted according to the same rules as notes, however, this
> is not how obsidian sorts. Obsidian sorts only files by default, not
> directories. Directories are initially sorted A-z, and then kept in that
> order when sorting files.

## [0.11.2](https://github.com/erikjuhani/basalt/releases/tag/basalt/0.11.2) (Dec, 21 2025)

### Changed

- [88ec357](https://github.com/erikjuhani/basalt/commit/88ec3577ef32f31e2277050bfe572d8fe18506cf) Update basalt-core version to 0.7.0

> Use direct definitions in the respective crates instead of using
> workspace dependencies for basalt-core and basalt-widgets.

### Fixed

- [8744562](https://github.com/erikjuhani/basalt/commit/8744562b37d04d5522ac81b13914d605e31a053a) Fix nested task list rendering to properly indent subtasks by @erikjuhani

> The parser now correctly nests subtasks within their parent task nodes
> rather than treating them as siblings. The task_kind field changed from
> Option to Vec to track nested task states, similar to item_kind.
>
> Nested task lists are now properly rendered following the same
> implementation as in the list items code.

- [5b54928](https://github.com/erikjuhani/basalt/commit/5b54928c66133f6755c82ac34ce9cbc5fa8b7026) Fixes 'sticky' symbols when switching between read and edit by @erikjuhani

> The sticky key effect was visible for example with task lists when tasks
> were intended with tabs in the source. These tab characters would never
> replace the existing symbols from the buffer. The sticky symbols issue
> was fixed by replacing the tab characters with two spaces.

## [0.11.1](https://github.com/erikjuhani/basalt/releases/tag/basalt/0.11.1) (Dec, 08 2025)

### Fixed

- [63b5e9f](https://github.com/erikjuhani/basalt/commit/63b5e9f618aac66bf47f8b4160a959600dcab28d) Use basalt-core 0.6.3 version in basalt

> basalt-core 0.6.3 fixes vault json deserializer for `ts` field and sets
> it as optional. It was previously set as required. If the `ts` field was
> missing it would crash basalt.

## [0.11.0](https://github.com/erikjuhani/basalt/releases/tag/basalt/0.11.0) (Nov, 30 2025)

Basalt author and maintainer here! Wanted to write a few words before the _regular_ changelog.

Phew, this took longer than expected, but here we are. The editor feature does not offer feature parity with the tui-textarea that was being used previously, however, it can properly wrap the text while writing, which frankly, I find quite pleasing.

If you encounter any bugs or additional strangeness please open an issue! The editor was made by me and most likely contains errors. Use with caution! :)

I'm taking a small break from basalt for the advent of code puzzles! So expect slower development during December.

This release fixes the following issues: [#105](https://github.com/erikjuhani/basalt/issues/105), [#104](https://github.com/erikjuhani/basalt/issues/104), [#95](https://github.com/erikjuhani/basalt/issues/95)

Demo:

![basalt demo of new 0.11.0 version editor capabilities](https://github.com/erikjuhani/basalt/blob/083ca1abc96ae35bf8ba144476c0224a63854259/assets/basalt-0-11-0.gif?raw=true)

### Added

- [ba8f3a0](https://github.com/erikjuhani/basalt/commit/ba8f3a0b260f935e88346a0f451572bdeac8ffe8) Introduce virtual document structure with rendering by @erikjuhani

> I decided to implement a virtual document, which is essentially the
> virtually rendered version of the markdown document. This virtual
> document is a collection of virtual blocks, virtual lines and virtual
> spans.
>
> Virtual lines and spans are turned into Ratatui variants to render them
> in terminal.
>
> Virtual spans are separated into two concepts, synthetic and content.
>
> - Synthetic spans are elements that are not calculated as part of the
>   markdown source.
> - Content spans on the other hand are elements that are calculated as
>   part of the markdown source.
>
> This separation enables more fluid use cases and easier management
> codewise for more rendered content, like text wrapping symbols,
> additional emphasize lines or spans, etc.
>
> Rendering is a collection of functions that are turned into virtual
> blocks. These virtual blocks map directly into top level markdown nodes.
>
> All rendered functions wrap the text with the given max width. The text
> wrapping is a generalized wrapping function that can be now run for "any"
> markdown node that is defined in the ast module.

- [943256b](https://github.com/erikjuhani/basalt/commit/943256b8710cdd2081836c877623a3db8be21b70) Add Cursor module by @erikjuhani

> Cursor module is responsible for keeping up with the cursor state.
> Cursor can be switched between two modes, read and edit.
>
> Each mode behaves a bit differently, read only considers the virtual
> elements, and edit mode considers the source content.
>
> For now only the read mode variant is properly implemented.
>
> The rendering is handled with a separate stateful CursorWidget component
> that takes the cursor state as an input and draws the cursor
> accordingly.
>
> In read mode the cursor is drawn as a full-width line cursor.

- [b26f0ff](https://github.com/erikjuhani/basalt/commit/b26f0ffbf5b2ca7d86c6fc1bb882b8259d5c1610) Add soft break parsing to markdown parser

> Also add empty_line() helper function to TextSegment struct. This
> creates a new empty line with "\n" as the content.
>
> This empty line can then be split in the render functions, but still
> keep the content inside a single markdown node (e.g. paragraph).

- [5222b7e](https://github.com/erikjuhani/basalt/commit/5222b7e316227076d287f897500cc479386db53e) Add text wrapping helper utility

> The text wrapper module exposes `wrap_preserve_trailing`, which, as name
> implies, keeps the trailing whitespace.
>
> I'm using the `textwrap::WordSeparator` to find and iterate over the
> words in the text, and then determine if the word fits in the current
> line by using the passed max width variable. The `textwrap` crate itself
> did not ship with a premade wrapping utility that would have preserved
> the whitespace, at least, I did not find such utility.

- [bf2c86d](https://github.com/erikjuhani/basalt/commit/bf2c86d9cc4791517364297dca2173e2a0cd2828) Add a simple viewport abstraction

> The viewport abstraction wraps the ratatui layout structure Rect and
> uses additional layout data structures like Size and Offset.

- [5b83d5f](https://github.com/erikjuhani/basalt/commit/5b83d5ffd30892b89353329b0c224000fa1896a1) Add `chars` and `char_indices` methods to virtual span

> These helper methods will allow easier access to the underlying chars
> and their byte indices.

### Changed

- [ee4976f](https://github.com/erikjuhani/basalt/commit/ee4976f7734fc2ef67145195be3fa0f7f8fd3ecf) Replace old editor with new implementation by @erikjuhani

> The old note editor variant was hard to maintain and was lacking proper
> structure. Adding new ast nodes or elements was a cumbersome process.
>
> The new variant uses logical structures like virtual document and
> separate rendering functions to achieve a more cohesive end result.
>
> The editor now requires to have a viewport in order to render anything
> properly. This is a requirement for example to decide the correct
> wrapping width for text elements.
>
> This change also introduces fix for scrolling. Now scrolling works
> properly and cursor is always visible in the viewport. This fixes the
> issue #104.
>
> The rendering is simplified drastically due to the use of more logical
> structures.
>
> In this commit, only read mode is enabled and the edit mode support is
> missing.

- [88be5fa](https://github.com/erikjuhani/basalt/commit/88be5fa7e73b0165681db8f366db752cfdbc0075) Return a reference instead of owned RichText

> No reason to not return a reference, and we avoid allocation.

- [739704a](https://github.com/erikjuhani/basalt/commit/739704a371032b0ae4c985145694657315ab2ed4) In virtual span width() returns both content and synthetic width

> Previously only content width was taken into account, however, this does
> not work as intended as the synthetic width needs to be calculated as
> well to find for example the correct offset for cursor column.

- [fa59ed5](https://github.com/erikjuhani/basalt/commit/fa59ed566d7f504d9af0a635754fccc1c7b9207f) Remove unused methods from virtual line

> Also simplified and improved the existing methods. For example virtual
> spans now retuns a slice instead of owned Vec.

- [d00f9d4](https://github.com/erikjuhani/basalt/commit/d00f9d41fa05274644a8250d2bd1684193da78af) Implement custom text editor

> This custom implementation replaces tui-textarea with proper text
> wrapping and better WYSIWYG experience. Also the custom implementation
> allows for more granular control over how elements are, rendered and
> positioned.
>
> I tried to mimic the previous functionality in a way so as little is
> lost as possible feature wise, obviously, since this is made from
> scratch the feature parity is far off still.
>
> Additionally there is some known issues, like cursor positioning is
> incorrect with unicode symbols in source content. This can be observed
> by wrong end position when moving the cursor to the right most end. The
> cursor appears as if it is stuck, but that is due to wrong count
> somewhere, which should be fixed, but in a different commit.
>
> For now the implementation is very limited and implements only a
> restricted set of features: Editing markdown nodes, saving changes to
> file, moving by words and scrolling by half pages.
>
> This commit has quite many changes, and, unfortunately the nature of how
> this refactor was introduced, was difficult to separate the commits
> atomically and cleanly to smaller pieces.
>
> But the most notable ones are:
>
> - Cursor changes, which include the cursor movement by columns and words
>   and proper cursor positioning from the source offset location.
>
> - Editor state changes to allow insertion, deletion and saving of files,
>   and source range shifting, which is related to the editor
>   functionality, which essentially shifts the end of the source range,
>   if the source ranges are not shifted properly and if the text buffer
>   exceeds the range start of next node the text buffer would be
>   replicated and rendered in-place of the next node.
>
> - Render changes, which introduce a new consolidated text wrapping and
>   handling for newline characters in both visual and raw rendering
>   modes. The new consolidated text wrapping uses the new whitespace
>   preserving text wrap function. Additionally source offset for rendered
>   virtual lines were fixed. Also added unicode-width dependency for
>   accurate unicode character width calculations in render.

### Dependencies

- [2985a13](https://github.com/erikjuhani/basalt/commit/2985a13678b0783af61d9dbd16a74c2c1d639b87) Update Rust crate etcetera to 0.11.0 by @renovate-updater[bot]

> | datasource | package  | from   | to     |
> | ---------- | -------- | ------ | ------ |
> | crate      | etcetera | 0.10.0 | 0.11.0 |

### New Contributors
* @erikjuhani made their first contribution in [#191](https://github.com/erikjuhani/basalt/pull/191)
* @renovate-updater[bot] made their first contribution
* @istudyatuni made their first contribution


**Full Changelog**: https://github.com/erikjuhani/basalt/compare/basalt/v0.10.4...basalt/0.11.0


## 0.11.0 (Unreleased)

This release adds new improved note editor with proper text wrapping for all markdown elements (excluding code blocks).

The new improved and refactored editor code should enable faster feature creation.

### Added

- [Add Cursor module](https://github.com/erikjuhani/basalt/commit/943256b8710cdd2081836c877623a3db8be21b70)
- [Introduce virtual document structure with rendering](https://github.com/erikjuhani/basalt/commit/ba8f3a0b260f935e88346a0f451572bdeac8ffe8)

### Changed

- [Replace old editor with new implementation](https://github.com/erikjuhani/basalt/commit/ee4976f7734fc2ef67145195be3fa0f7f8fd3ecf)

## 0.10.4 (Oct, 6 2025)

This release adds note name support in the editor pane. Follows a similar approach as in the Obsidian app itself. The note name can not be changed yet in basalt.

Additionally contains some fixes and slight changes to headings to make them work better with the new note name implementation.

### Added

- [Add support to render filename in the editor pane](https://github.com/erikjuhani/basalt/commit/f23e1c40c04cad62aa23e99983adf9dc9bc4474c)

### Fixed

- [Fix Outline state allowing to move past visible items](https://github.com/erikjuhani/basalt/commit/3339182706790d0865abb2f2bd1ceee129183397)
- [Fix sub item rendering](https://github.com/erikjuhani/basalt/commit/e5c2835a90d38dd239b5ed820cd95b65a5934321)

### Changed

- [Change markdown heading level 1 and 2 to more subtle](https://github.com/erikjuhani/basalt/commit/e3db134e45621215e66991a88edf5d9388db23eb)
- [Only text is crossed over for "hard checked" tasks](https://github.com/erikjuhani/basalt/commit/13a8a26a69ad84f29c6edc7cf76e52a905b2996f)

## 0.10.3 (Sep, 15 2025)

This release adds the support to easily hide and expand the explorer pane (file tree). Expanding and hiding is done with h, l and arrow left, and arrow right.

When explorer is expanded a ⟹  symbol is shown for clarity of the current state.

### Added

- [Support expandable explorer commands in Explorer widget](https://github.com/erikjuhani/basalt/commit/4e815790a30f4ee949fbc25648e2c676dd19ab59)
- [Add hide_pane and expand_pane explorer commands](https://github.com/erikjuhani/basalt/commit/18f2f0b06c6b73eaa120852a845e4d33796980b1)

### Fixed

- [Fix crash when note editor has no width available](https://github.com/erikjuhani/basalt/commit/f52d084cdbec9c7e49d82a7f0c89a0b6d5d950a7)

## 0.10.2 (Sep, 13 2025)

Deprecated the following config commands:

- "note_editor_experimental_set_edit_mode"
- "note_editor_experimental_set_read_mode"
- "note_editor_experimental_exit_mode"

Use these instead:

- "note_editor_experimental_set_edit_view"
- "note_editor_experimental_set_read_view" and
- "note_editor_experimental_exit"

### Changed

- [Change note editor views and modes to follow Obsidian equivalent](https://github.com/erikjuhani/basalt/commit/371df9adf40624762dbf81b36c7395c7a5c34d3b)

## 0.10.1 (2025-08-31)

### Added

- [Add arbitrary (sync, spawn) command execution](https://github.com/erikjuhani/basalt/commit/750108f3282af5e947c23eb88ff3b5f8f196d0e4)

### Changed

- [Adjusted Explorer folded border to match outline](https://github.com/erikjuhani/basalt/commit/56ee16be7cfc8a211a980295818a2a2204009f98)

## 0.10.0 (2025-08-21)

### Added

- [Add `Outline` module](https://github.com/erikjuhani/basalt/commit/f02ac878102915d749ae79d60203ec512c5ef484)

### Changed

- [Change focus switch to support previous and next panes](https://github.com/erikjuhani/basalt/commit/d1cb962370cf03ec3f3da0527427d037fa81ccfd)

## 0.9.0 (2025-07-30)

### Added

- [Add experimental note editor support](https://github.com/erikjuhani/basalt/commit/924e2e25d9515b08cead11f3f4ef0413ef500a22)

## 0.8.0 (2025-06-25)

### Added

- [Add user configuration file support for customizable key bindings](https://github.com/erikjuhani/basalt/commit/b04b41a13a84aa2fce3300fa1b4cc44954f62f4f)
- [Adds a 'config' field to the AppState, which is based on a toml file (#25)](https://github.com/erikjuhani/basalt/commit/ed24f4c649b5ea66896911e5350ba27ea03b4694)

### Fixed

- [Fix display issue with active Pane UI element](https://github.com/erikjuhani/basalt/commit/f05eb3af66e18b886c774670f972284c2bcce427)

## 0.7.0 (2025-06-15)

### Changed

- [Refactor state management](https://github.com/erikjuhani/basalt/commit/0d49afb9dd7078215ed3fb15ee6dea23da1c0ba9)

### Added

- [Add visiblity and visiblity helper methods to HelpModal](https://github.com/erikjuhani/basalt/commit/8f92863932325157ffe0e181470d194ee90b2a23)
- [Add visibility and helper methods to VaultSelectorModal](https://github.com/erikjuhani/basalt/commit/1243a33d62d0cac04d2bb7556477e44867b491f8)
- [Add active field to MarkdownView to indicate active state](https://github.com/erikjuhani/basalt/commit/5880a160f30628ebec4f6e043e97b83ccb8a1899)

## 0.6.1 (2025-06-07)

### Fixed

- [Use snap folder `/current` instead of `/common`](https://github.com/erikjuhani/basalt/commit/ac0ee653250e0ca052063506f10d61a9ce2f7735)

## 0.6.0 (2025-06-01)

### Added

- [Add `Explorer` module](https://github.com/erikjuhani/basalt/commit/5d1f05fcbe5c0add6f687512fc3cf538a2df1148)

### Fixed

- [Fix large size difference between variants](https://github.com/erikjuhani/basalt/commit/159ae7ab22ab5cd4351075b2fe526a5628cfb3b9)

## 0.5.0 (2025-05-25)

### Fixed

- [Support deeper block quotes with proper prefix recursion](https://github.com/erikjuhani/basalt/commit/3f1ed73a0edcfbb17800cb27d7bda145b93369f6)
- [Add two space indentation to list items](https://github.com/erikjuhani/basalt/commit/b1a021e25759c39cee00cd1b787ccfafa1ad4ad4)
- [Fix code block rendering](https://github.com/erikjuhani/basalt/commit/cae8fae154d7a6da2ec0ffb6b28ac85b2cc73023)

### Changed

- [Change Markdown headings to stylized variants](https://github.com/erikjuhani/basalt/commit/30321916b5d6f79afe2a58f9b45b6eaa963ac12e)

## 0.4.1 (2025-05-25)

### Changed

- [Use dark gray color instead of black](https://github.com/erikjuhani/basalt/commit/237c7e436c76d61fe4339aa961e1f77a2ffbb43d)

## 0.4.0 (2025-05-25)

### Fixed

- [Update basalt-core to version 0.5.0](https://github.com/erikjuhani/basalt/commit/a30d611b79a98b661aabd27eca0c8caa69e27fa8), which potentially fixes #44

Check basalt-core CHANGELOG [here](../basalt-core/CHANGELOG.md).

## 0.3.7 (2025-05-22)

### Added

- [Add `stylized_text` module](https://github.com/erikjuhani/basalt/commit/47db925ef858831672be69fb11bcf272522e1b3a)
- [Add `lib.rs` which allows basalt to be used as a library](https://github.com/erikjuhani/basalt/commit/ce094ed8aab1945aad36955bce83eeea09085177)

### Fixed

- [Use a regular loop instead of recursion for rendering](https://github.com/erikjuhani/basalt/commit/4d9e6c83f2342b12501c2f316dbab05ab68119ab)

## 0.3.6 (2025-05-21)

### Fixed

- [Fix panic, when there are no notes inside a vault](https://github.com/erikjuhani/basalt/commit/4644f90a595f8000e983475b78e0d3605a5bc16e)

## 0.3.5

### Fixed

- [Use config_dir() to locate obsidian.json on Windows (#38)](https://github.com/erikjuhani/basalt/commit/839674c3e8fa1d8a9e6b7852bcc659dbd88e45dc)

## 0.3.4

### Added

- [Refactor Markdown event parser (#28)](https://github.com/erikjuhani/basalt/commit/4e82e7523a72064afe98c6c6de6ba8e84a334b71)
- [Add support for `LooselyChecked` task kind (#29)](https://github.com/erikjuhani/basalt/commit/1b9df5b0e167442f039fc02f8221a6a390e44acc)
- [Add support for ordered lists](https://github.com/erikjuhani/basalt/commit/7f715bb04c66066959588abfca5f29a3b3df22a7)
- [Add text wrapping for paragraphs](https://github.com/erikjuhani/basalt/commit/4a57d9a91e22c511bdbe23ae90fb6a3244d2dc32)

### Changed

- [Change checkbox symbol (#30)](https://github.com/erikjuhani/basalt/commit/11b944cbca19a020d984fbb272724ec80d1119e0)
- [Render code block as a full-width block](https://github.com/erikjuhani/basalt/commit/67905b4bacbff266c5579ac78be9ee65d9c23c85)

## 0.3.1

### Fixed

- [Adjusted the conditional config location for linux from ~/.../Obsidian to ~/.../obsidian](https://github.com/erikjuhani/basalt/commit/1bcc0375b9cb101e3fe8ace979c055ab0206bbd1)

## 0.3.0

### Added

- [Add `app` module](https://github.com/erikjuhani/basalt/commit/bd615f8da8813312fd9351b1ccdcf5e29b164d6d)
- [Add `start` module](https://github.com/erikjuhani/basalt/commit/e5ce84bee9b3801fdc4aecd43eb091c3055050fd)
- [Add `help_modal` module with `help.txt`](https://github.com/erikjuhani/basalt/commit/617e688bc277e4534d2f8fafaf9f0288cd026702)
- [Add `statusbar` module](https://github.com/erikjuhani/basalt/commit/05b42183514172c1b640c0d7ae5d6e3683942d5f)
- [Add `sidepanel` module](https://github.com/erikjuhani/basalt/commit/537917da8905db138c0839a05df2e80795f29524)
- [Add `vault_selector` and `vault_selector_modal`](https://github.com/erikjuhani/basalt/commit/8a42a008c094088a5bfb76178d566fd71246d380)
- [Add `text_counts` module](https://github.com/erikjuhani/basalt/commit/f646b8a1c2b0e055b7dd4c5b6f0963759200c731)
