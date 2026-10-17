//! This module provides the filter that decides which vault entries are left out.
use regex::Regex;
use serde::Deserialize;
use std::{fs, path::Path};

/// Decides which entries of a vault are left out when walking or watching it.
///
/// An entry is ignored if:
/// - its name starts with a dot (`.`), such as `.obsidian`, `.git` or `.trash`, or
/// - it matches one of the vault's "Excluded files", stored as `userIgnoreFilters` in
///   `.obsidian/app.json`.
///
/// An excluded files entry is matched against the vault-relative path the same way Obsidian
/// does: an entry wrapped in slashes (`/.../`) is a regular expression, anything else is a path
/// prefix such as `Archive/` or `folder/note.md`. Paths always use `/` as the separator.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use basalt_core::obsidian::VaultFilter;
///
/// let filter = VaultFilter::from_user_ignore_filters(["Archive/", "/\\.excalidraw\\.md$/"]);
///
/// assert!(filter.is_ignored(Path::new(".git"), true));
/// assert!(filter.is_ignored(Path::new("Archive"), true));
/// assert!(filter.is_ignored(Path::new("Drawing.excalidraw.md"), false));
/// assert!(!filter.is_ignored(Path::new("Notes/note.md"), false));
/// ```
#[derive(Debug, Clone, Default)]
pub struct VaultFilter {
    user_ignore_filters: Vec<IgnoreFilter>,
}

#[derive(Debug, Clone)]
enum IgnoreFilter {
    Prefix(String),
    Regex(Regex),
}

impl IgnoreFilter {
    fn parse(filter: &str) -> Option<Self> {
        match filter.strip_prefix('/').and_then(|f| f.strip_suffix('/')) {
            // Obsidian drops an invalid expression, so it is dropped here as well.
            Some(pattern) => Regex::new(pattern).ok().map(Self::Regex),
            None if filter.is_empty() => None,
            None => Some(Self::Prefix(filter.to_string())),
        }
    }

    fn is_match(&self, path: &str) -> bool {
        match self {
            Self::Prefix(prefix) => path.starts_with(prefix.as_str()),
            Self::Regex(regex) => regex.is_match(path),
        }
    }
}

impl VaultFilter {
    /// Creates a filter from Obsidian "Excluded files" entries, in the format of
    /// `userIgnoreFilters`. Empty entries and invalid regular expressions are left out.
    pub fn from_user_ignore_filters<T: AsRef<str>>(filters: impl IntoIterator<Item = T>) -> Self {
        Self {
            user_ignore_filters: filters
                .into_iter()
                .filter_map(|filter| IgnoreFilter::parse(filter.as_ref()))
                .collect(),
        }
    }

    /// Returns `true` if the entry at `path`, relative to the vault root, should be left out.
    /// `is_dir` tells whether the entry is a directory.
    ///
    /// A directory is matched with a trailing slash, so `Archive/` ignores the `Archive`
    /// directory itself, and with it everything inside, without reading it.
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let Some(name) = path.file_name().map(|name| name.to_string_lossy()) else {
            return false;
        };

        if name.starts_with('.') {
            return true;
        }

        if self.user_ignore_filters.is_empty() {
            return false;
        }

        let mut path = path
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if is_dir {
            path.push('/');
        }

        self.user_ignore_filters
            .iter()
            .any(|filter| filter.is_match(&path))
    }
}

/// Loads the vault's "Excluded files" from `app.json` in the given `.obsidian` directory as a
/// [`VaultFilter`].
///
/// A missing or malformed `app.json` gives a filter with only the built-in rules, since the
/// file is Obsidian's to write and a vault doesn't need one.
pub fn load_from(config_dir: &Path) -> VaultFilter {
    #[derive(Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct AppJson {
        user_ignore_filters: Vec<String>,
    }

    let app_json = fs::read_to_string(config_dir.join("app.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<AppJson>(&contents).ok())
        .unwrap_or_default();

    VaultFilter::from_user_ignore_filters(app_json.user_ignore_filters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hidden_entries_are_ignored_at_any_depth() {
        let filter = VaultFilter::default();

        assert!(filter.is_ignored(Path::new(".git"), true));
        assert!(filter.is_ignored(Path::new("folder/.trash"), true));
        assert!(filter.is_ignored(Path::new("folder/.hidden.md"), false));
        assert!(!filter.is_ignored(Path::new("folder/note.md"), false));
    }

    #[test]
    fn prefix_filters_match_vault_relative_paths() {
        let filter = VaultFilter::from_user_ignore_filters(["Archive/", "folder/note.md"]);

        assert!(filter.is_ignored(Path::new("Archive"), true));
        assert!(filter.is_ignored(Path::new("Archive/old.md"), false));
        assert!(filter.is_ignored(Path::new("folder/note.md"), false));
        assert!(!filter.is_ignored(Path::new("folder/other.md"), false));
        assert!(!filter.is_ignored(Path::new("folder"), true));
        assert!(!filter.is_ignored(Path::new("Archive.md"), false));
        assert!(!filter.is_ignored(Path::new("notes/Archive"), true));
    }

    #[test]
    fn regex_filters_match_vault_relative_paths() {
        let filter = VaultFilter::from_user_ignore_filters([r"/\.excalidraw\.md$/"]);

        assert!(filter.is_ignored(Path::new("a/Drawing.excalidraw.md"), false));
        assert!(!filter.is_ignored(Path::new("a/Drawing.md"), false));
    }

    #[test]
    fn empty_and_invalid_filters_are_dropped() {
        let filter = VaultFilter::from_user_ignore_filters(["", "/(/"]);

        assert!(filter.user_ignore_filters.is_empty());
        assert!(!filter.is_ignored(Path::new("note.md"), false));
    }

    #[test]
    fn load_from_reads_user_ignore_filters() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("app.json"),
            r#"{"alwaysUpdateLinks":true,"userIgnoreFilters":["Archive/"]}"#,
        )
        .unwrap();

        let filter = load_from(dir.path());

        assert!(filter.is_ignored(Path::new("Archive"), true));
        assert!(!filter.is_ignored(Path::new("Notes"), true));
    }

    #[test]
    fn load_from_without_valid_app_json_has_no_user_filters() {
        let dir = tempdir().unwrap();
        assert!(load_from(dir.path()).user_ignore_filters.is_empty());

        fs::write(dir.path().join("app.json"), "not json").unwrap();
        assert!(load_from(dir.path()).user_ignore_filters.is_empty());

        fs::write(dir.path().join("app.json"), r#"{"userIgnoreFilters":"x"}"#).unwrap();
        assert!(load_from(dir.path()).user_ignore_filters.is_empty());
    }
}
