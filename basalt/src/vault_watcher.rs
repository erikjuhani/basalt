use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use notify_debouncer_full::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer, RecommendedCache,
};

const DEBOUNCE: Duration = Duration::from_millis(250);

pub struct VaultWatcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    rx: Receiver<()>,
    path: PathBuf,
}

impl VaultWatcher {
    pub fn new(path: &Path) -> notify_debouncer_full::notify::Result<Self> {
        let (tx, rx) = mpsc::channel::<()>();

        let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            let relevant = events
                .iter()
                .flat_map(|event| event.paths.iter())
                .any(|p| is_relevant_path(p));
            if relevant {
                let _ = tx.send(());
            }
        })?;

        debouncer.watch(path, RecursiveMode::Recursive)?;

        Ok(VaultWatcher {
            _debouncer: debouncer,
            rx,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Drains all pending events and returns true if at least one was received.
    pub fn drain(&self) -> bool {
        let mut received = false;
        while self.rx.try_recv().is_ok() {
            received = true;
        }
        received
    }
}

fn is_relevant_path(path: &Path) -> bool {
    // Skip anything inside a hidden directory (`.obsidian`, `.git`, …) and
    // hidden files / editor swap files at the vault root.
    if path
        .components()
        .any(|c| matches!(c.as_os_str().to_str(), Some(s) if s.starts_with('.')))
    {
        return false;
    }

    // Accept Markdown files and paths with no extension (directories or
    // unknown — the explorer rescan will sort them out).
    match path.extension() {
        Some(ext) => ext == "md",
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::is_relevant_path;
    use std::path::PathBuf;

    #[test]
    fn markdown_files_are_relevant() {
        assert!(is_relevant_path(&PathBuf::from("/vault/note.md")));
        assert!(is_relevant_path(&PathBuf::from("/vault/sub/note.md")));
    }

    #[test]
    fn directories_are_relevant() {
        assert!(is_relevant_path(&PathBuf::from("/vault/new-folder")));
    }

    #[test]
    fn hidden_paths_are_ignored() {
        assert!(!is_relevant_path(&PathBuf::from(
            "/vault/.obsidian/workspace.json"
        )));
        assert!(!is_relevant_path(&PathBuf::from("/vault/.git/HEAD")));
        assert!(!is_relevant_path(&PathBuf::from("/vault/.DS_Store")));
    }

    #[test]
    fn non_markdown_files_are_ignored() {
        assert!(!is_relevant_path(&PathBuf::from("/vault/image.png")));
        assert!(!is_relevant_path(&PathBuf::from("/vault/note.md.swp")));
    }
}
