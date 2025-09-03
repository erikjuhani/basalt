# Changelog

## 0.10.2 (Unreleased)

- [Set editor to READ mode when exiting EDIT mode](https://github.com/erikjuhani/basalt/commit/e063b625157827e5c38660058badf9e48611bb62)

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
