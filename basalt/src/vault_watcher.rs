use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};

use basalt_core::obsidian::VaultFilter;
use notify_debouncer_full::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode, WatcherKind},
    DebounceEventResult, Debouncer, RecommendedCache,
};

const DEBOUNCE: Duration = Duration::from_millis(250);

type VaultDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;

pub struct VaultWatcher {
    debouncer: VaultDebouncer,
    rx: Receiver<Vec<PathBuf>>,
    path: PathBuf,
    filter: Arc<VaultFilter>,
    /// Directories watched one by one, or `None` if the whole vault is watched recursively.
    watched_dirs: Option<HashSet<PathBuf>>,
}

impl VaultWatcher {
    /// Watches the vault at `path`, leaving out what `filter` ignores.
    ///
    /// With backends that put a watch on every directory, such as inotify, only the directories
    /// `filter` keeps are watched, one by one, so `.git` or excluded folders cost no watches. The
    /// watched set follows directories as they are created, renamed or removed. Backends that
    /// watch a tree natively (FSEvents, ReadDirectoryChangesW) watch the vault recursively and
    /// only filter the events.
    pub fn new(path: &Path, filter: VaultFilter) -> notify_debouncer_full::notify::Result<Self> {
        let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
        let filter = Arc::new(filter);

        let root = path.to_path_buf();
        let event_filter = Arc::clone(&filter);
        let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            let paths: Vec<PathBuf> = events
                .iter()
                .flat_map(|event| event.paths.iter())
                .filter(|p| !is_ignored(&root, &event_filter, p))
                .cloned()
                .collect();
            if !paths.is_empty() {
                let _ = tx.send(paths);
            }
        })?;

        let watched_dirs = match VaultDebouncer::kind() {
            WatcherKind::Fsevent | WatcherKind::ReadDirectoryChangesWatcher => {
                debouncer.watch(path, RecursiveMode::Recursive)?;
                None
            }
            _ => {
                debouncer.watch(path, RecursiveMode::NonRecursive)?;
                let mut watched_dirs = HashSet::from([path.to_path_buf()]);
                watch_dirs(&mut debouncer, &mut watched_dirs, path, &filter, path);
                Some(watched_dirs)
            }
        };

        Ok(VaultWatcher {
            debouncer,
            rx,
            path: path.to_path_buf(),
            filter,
            watched_dirs,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Drains all pending events and returns true if at least one of them is relevant to the
    /// explorer. Keeps the per-directory watches in step with the events.
    pub fn drain(&mut self) -> bool {
        let mut received = false;
        while let Ok(paths) = self.rx.try_recv() {
            for path in &paths {
                self.update_watches(path);
            }
            received |= paths.iter().any(|p| has_relevant_extension(p));
        }
        received
    }

    fn update_watches(&mut self, path: &Path) {
        let Some(watched_dirs) = self.watched_dirs.as_mut() else {
            return;
        };

        if is_dir(path) {
            if !watched_dirs.contains(path)
                && self
                    .debouncer
                    .watch(path, RecursiveMode::NonRecursive)
                    .is_ok()
            {
                watched_dirs.insert(path.to_path_buf());
                // The directory may have been filled before its watch was added.
                watch_dirs(
                    &mut self.debouncer,
                    watched_dirs,
                    &self.path,
                    &self.filter,
                    path,
                );
            }
        } else if watched_dirs.contains(path) {
            // Removed or renamed away. Inotify drops the watch of a removed directory by itself,
            // but keeps watching a renamed one under its old path.
            watched_dirs.retain(|dir| {
                let gone = dir.starts_with(path);
                if gone {
                    let _ = self.debouncer.unwatch(dir);
                }
                !gone
            });
        }
    }
}

/// Watches every directory below `dir` that `filter` keeps, without following symlinks.
fn watch_dirs(
    debouncer: &mut VaultDebouncer,
    watched_dirs: &mut HashSet<PathBuf>,
    root: &Path,
    filter: &VaultFilter,
    dir: &Path,
) {
    for subdir in subdirs(root, filter, dir) {
        if !watched_dirs.contains(&subdir)
            && debouncer
                .watch(&subdir, RecursiveMode::NonRecursive)
                .is_ok()
        {
            watched_dirs.insert(subdir);
        }
    }
}

/// Returns every directory below `dir` that `filter` keeps, without following symlinks.
fn subdirs(root: &Path, filter: &VaultFilter, dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .map(|entry| entry.path())
        .filter(|path| !is_ignored(root, filter, path))
        .flat_map(|path| {
            let mut dirs = subdirs(root, filter, &path);
            dirs.push(path);
            dirs
        })
        .collect()
}

/// Returns true for a directory that is not a symlink.
fn is_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Returns true if `path`, or any directory it is in below `root`, is ignored by `filter`.
/// Paths outside `root` are ignored.
fn is_ignored(root: &Path, filter: &VaultFilter, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };

    relative
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .any(|ancestor| filter.is_ignored(ancestor, ancestor != relative || is_dir(path)))
}

fn has_relevant_extension(path: &Path) -> bool {
    // Accept Markdown files and paths with no extension (directories or
    // unknown — the explorer rescan will sort them out).
    match path.extension() {
        Some(ext) => ext == "md",
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn is_relevant_path(path: &str) -> bool {
        let path = Path::new(path);
        !is_ignored(Path::new("/vault"), &VaultFilter::default(), path)
            && has_relevant_extension(path)
    }

    #[test]
    fn markdown_files_are_relevant() {
        assert!(is_relevant_path("/vault/note.md"));
        assert!(is_relevant_path("/vault/sub/note.md"));
    }

    #[test]
    fn directories_are_relevant() {
        assert!(is_relevant_path("/vault/new-folder"));
    }

    #[test]
    fn hidden_paths_are_ignored() {
        assert!(!is_relevant_path("/vault/.obsidian/workspace.json"));
        assert!(!is_relevant_path("/vault/.git/HEAD"));
        assert!(!is_relevant_path("/vault/.DS_Store"));
        assert!(!is_relevant_path("/vault/sub/.trash/note.md"));
    }

    #[test]
    fn non_markdown_files_are_ignored() {
        assert!(!is_relevant_path("/vault/image.png"));
        assert!(!is_relevant_path("/vault/note.md.swp"));
    }

    #[test]
    fn hidden_directories_above_the_vault_are_not_ignored() {
        let root = Path::new("/home/user/.vaults/notes");
        let path = Path::new("/home/user/.vaults/notes/note.md");
        assert!(!is_ignored(root, &VaultFilter::default(), path));
    }

    #[test]
    fn paths_outside_the_vault_are_ignored() {
        let path = Path::new("/elsewhere/note.md");
        assert!(is_ignored(
            Path::new("/vault"),
            &VaultFilter::default(),
            path
        ));
    }

    #[test]
    fn paths_inside_ignored_directories_are_ignored() {
        let root = Path::new("/vault");
        let filter = VaultFilter::from_user_ignore_filters(["Archive/"]);
        assert!(is_ignored(
            root,
            &filter,
            Path::new("/vault/Archive/old.md")
        ));
        assert!(is_ignored(
            root,
            &filter,
            Path::new("/vault/notes/.git/HEAD")
        ));
        assert!(!is_ignored(
            root,
            &filter,
            Path::new("/vault/Notes/note.md")
        ));
    }

    fn vault_with_ignored_dirs() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::create_dir_all(root.join("Archive/2020")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root, root.join("a/loop")).unwrap();
        dir
    }

    #[test]
    fn subdirs_skips_ignored_and_symlinked_directories() {
        let dir = vault_with_ignored_dirs();
        let root = dir.path();
        let filter = VaultFilter::from_user_ignore_filters(["Archive/"]);

        let subdirs: HashSet<PathBuf> = subdirs(root, &filter, root).into_iter().collect();

        assert_eq!(subdirs, HashSet::from([root.join("a"), root.join("a/b")]));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn watches_follow_created_and_removed_directories() {
        let dir = vault_with_ignored_dirs();
        let root = dir.path();
        let filter = VaultFilter::from_user_ignore_filters(["Archive/"]);

        let mut watcher = VaultWatcher::new(root, filter).unwrap();
        let watched = |watcher: &VaultWatcher| watcher.watched_dirs.clone().unwrap();
        assert_eq!(
            watched(&watcher),
            HashSet::from([root.to_path_buf(), root.join("a"), root.join("a/b")])
        );

        fs::create_dir_all(root.join("c/d")).unwrap();
        watcher.update_watches(&root.join("c"));
        assert!(watched(&watcher).contains(&root.join("c/d")));

        fs::remove_dir_all(root.join("a")).unwrap();
        watcher.update_watches(&root.join("a"));
        assert_eq!(
            watched(&watcher),
            HashSet::from([root.to_path_buf(), root.join("c"), root.join("c/d")])
        );
    }
}
