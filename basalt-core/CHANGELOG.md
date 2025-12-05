# Changelog

## [0.6.2](https://github.com/erikjuhani/basalt/releases/tag/basalt-core/0.6.2) (Dec, 06 2025)

### Added

- [fa3d5de](https://github.com/erikjuhani/basalt/commit/fa3d5de3310447956f5b4f24af747fb9efad161c) Add note creation methods to Vault

> Add create_note and create_untitled_note methods for creating notes in
> Obsidian vaults. When a note with the given name already exists, a
> numbered suffix is appended to find an available name, which correlates
> with how Obsidian app functions when creating new notes, up to 999
> attempts.
>
> The find_available_note_name method handles the name resolution by
> checking if a path exists and incrementing the suffix until a free name
> is found. A new `MaxAttemptsExceeded` error variant was added for when
> all 999 attempts are exhausted. This prevents the need for infinity
> loop since we have a relatively large breaking point.

### Dependencies

- [9a7e731](https://github.com/erikjuhani/basalt/commit/9a7e731f3e940fb52823b2939128e4ec97d11707) Bump thiserror from 2.0.12 to 2.0.15 by @dependabot[bot]

> Bumps [thiserror](https://github.com/dtolnay/thiserror) from 2.0.12 to 2.0.15.
> - [Release notes](https://github.com/dtolnay/thiserror/releases)
> - [Commits](https://github.com/dtolnay/thiserror/compare/2.0.12...2.0.15)
>
> ---
> updated-dependencies:
> - dependency-name: thiserror
>   dependency-version: 2.0.15
>   dependency-type: direct:production
>   update-type: version-update:semver-patch
> ...
>
> Signed-off-by: dependabot[bot] <support@github.com>

- [b66dd09](https://github.com/erikjuhani/basalt/commit/b66dd092c164cf6afe3be086fd72e0d1fb61fc14) Bump serde from 1.0.219 to 1.0.223 by @dependabot[bot]

> Bumps [serde](https://github.com/serde-rs/serde) from 1.0.219 to 1.0.223.
> - [Release notes](https://github.com/serde-rs/serde/releases)
> - [Commits](https://github.com/serde-rs/serde/compare/v1.0.219...v1.0.223)
>
> ---
> updated-dependencies:
> - dependency-name: serde
>   dependency-version: 1.0.223
>   dependency-type: direct:production
>   update-type: version-update:semver-patch
> ...
>
> Signed-off-by: dependabot[bot] <support@github.com>

- [5ad8ecc](https://github.com/erikjuhani/basalt/commit/5ad8ecc5a780a4027719b7e916b849bbe6110575) Update Rust crate pulldown-cmark to 0.13.0 by @renovate-updater[bot]


### Fixed

- [1a6e870](https://github.com/erikjuhani/basalt/commit/1a6e870a6687bb1149105777c7bb936b08a13e1a) Exhaust subscript and superscript pulldown-cmark tags by @erikjuhani

> Fixes the compiler issue. Pattern match was not exhaustive as subscript
> and superscript tags were added in 0.13.0 version of pulldown-cmark.

## 0.6.2 (Unreleased)

### markdown

#### Fixed

- [Exhaust subscript and superscript pulldown-cmark tags](https://github.com/erikjuhani/basalt/commit/1a6e870a6687bb1149105777c7bb936b08a13e1a)

## 0.6.1 (2025-06-05)

### obsidian

#### Fixed

- [Use snap folder `/current` instead of `/common`](https://github.com/erikjuhani/basalt/commit/ac0ee653250e0ca052063506f10d61a9ce2f7735)

## 0.6.0 (2025-06-01)

### obsidian

#### Breaking

- [Add `VaultEntry` to obsidian module and remove old implementation](https://github.com/erikjuhani/basalt/commit/f1fe41e0d6933d6e523094c60bacada411d07d68)

## 0.5.0 (2025-05-25)

### obsidian

#### Breaking

- [Return `Vec` instead of `Iterator` from `notes()`](https://github.com/erikjuhani/basalt/commit/d56f2529971f54e8931f31ed32e2651087050c24)
- [Remove `created` field from Note as obsolete](https://github.com/erikjuhani/basalt/commit/fa17bf67ed13f002b8a97c259c18013a19756907)

#### Changed

- [Use try_exists in load_from for global Obsidian config](https://github.com/erikjuhani/basalt/commit/9f5359ddf38b9b3482f066c3b3bbc3339d4fb2ff)

#### Fixed

- [Get all potential obsidian global config locations](https://github.com/erikjuhani/basalt/commit/a5136b18ea87d00c5ca53bb539910df22582f260)

## 0.4.3 (2025-05-21)

### Fixed

- [Fix clippy error with matches! expression](https://github.com/erikjuhani/basalt/commit/725eac3c0b5103a6de34cd155611d22091a245ab)
- [Use config_dir() to locate obsidian.json on Windows (#38)](https://github.com/erikjuhani/basalt/commit/839674c3e8fa1d8a9e6b7852bcc659dbd88e45dc)

## 0.4.2 (2025-05-01)

### Fixed

- [Adjusted the conditional config location for linux from ~/.../Obsidian to ~/.../obsidian, following the information provided by the link in the original source.](https://github.com/erikjuhani/basalt/commit/1bcc0375b9cb101e3fe8ace979c055ab0206bbd1)

## 0.4.1 (2025-04-20)

### Changed

- [Change `TryInto` to `TryFrom`](https://github.com/erikjuhani/basalt/commit/d0cc15c14d21507b148499808e92da78d958c771)

### Breaking

- [Move `Default` impl of `Note` under `note.rs`](https://github.com/erikjuhani/basalt/commit/3916185bf946dc6ff8af3efee02526ae3175fff5)
- [Return `Vec<&Vault>` from `vaults()` instead of `Iterator`](https://github.com/erikjuhani/basalt/commit/f7587c98e119bc0bb43b55425baeb2797d9682ee)
- [Use `Path` instead `PathBuf` when loading config from path](https://github.com/erikjuhani/basalt/commit/256fb33d8b0cb893496a1eea8a08ce025f33fb48)
- [Use `BTreeMap` instead of `HashMap` to keep same order of vaults](https://github.com/erikjuhani/basalt/commit/7ed11881cd83cc489f98bf0d2e679a6c7fa12d9d)
- [Add `source_range` field to Nodes](https://github.com/erikjuhani/basalt/commit/1c199259f3831768e1823a34c9165c489f71eed0)

## 0.2.2 (2025-02-27)

### Added

- [Add blank implementations for `TextNode` and `Text`](https://github.com/erikjuhani/basalt/commit/a252f62930ec59f21255d08278762734eb312cef)

### Fixed

- [Fix skipping text nodes in markdown parser](https://github.com/erikjuhani/basalt/commit/3bc112edd2b452ea7093d0e71fcfa0d02bc0b9c4)

## 0.2.1 (2025-02-23)

### Added

- [Add markdown parser and custom AST nodes](https://github.com/erikjuhani/basalt/commit/125bf5d4637f20b9816cb383c56c750a3e35d40c)

## 0.2.0 (2025-02-18)

### Added

- [Add `get_open_vault` method to config](https://github.com/erikjuhani/basalt/commit/8e7647bf9636392b6c330c4b6fe38e46f17f8a5a)

### Breaking

- [Rename `vault_by_name` method to `get_vault_by_name`](https://github.com/erikjuhani/basalt/commit/288931ae87fb639fd6437fa21b9a9b68a612b0d0)
