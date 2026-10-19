use std::{
    cmp::Reverse,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use dua_core::{Options, Order};

use super::{Error, Note, Result};

const MAX_THREADS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum VaultEntry {
    File(Note),
    Directory {
        name: String,
        path: PathBuf,
        entries: Vec<VaultEntry>,
    },
}

impl VaultEntry {
    #[allow(missing_docs)]
    pub fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } => name,
            Self::File(note) => note.name(),
        }
    }
}

impl VaultEntry {
    pub(crate) fn walk(
        path: &Path,
        skip: impl Fn(&Path, bool) -> bool + Send + Sync + 'static,
    ) -> Result<Self> {
        if path.is_file() {
            let note = Note::try_from((note_name(path)?, path.to_path_buf()))?;
            return Ok(VaultEntry::File(note));
        }

        let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let skip = Arc::new(skip);
        let mut tree = TreeBuilder::default();

        for entry in parallel_walk(&root, Arc::clone(&skip)) {
            let entry_path = entry.path();
            let relative = entry_path.strip_prefix(&root).unwrap_or(&entry_path);
            let path = path.join(relative);
            match kind(&entry, &path) {
                Some(Kind::Directory) if !skip(relative, true) => tree.add_directory(path),
                Some(Kind::Note) if !skip(relative, false) => tree.add_note(path)?,
                _ => {}
            }
        }

        tree.build(path)
    }
}

enum Kind {
    Directory,
    Note,
}

fn kind(entry: &dua_core::Entry, path: &Path) -> Option<Kind> {
    let file_type = entry.file_type;
    let is_symlink_to_file = file_type.is_symlink() && path.is_file();

    if file_type.is_dir() {
        Some(Kind::Directory)
    } else if file_type.is_file() || is_symlink_to_file {
        Some(Kind::Note)
    } else {
        None
    }
}

fn parallel_walk(
    root: &Path,
    skip: Arc<impl Fn(&Path, bool) -> bool + Send + Sync + 'static>,
) -> impl Iterator<Item = dua_core::Entry> {
    let descend_root = root.to_path_buf();
    let descend = move |entry: &dua_core::Entry| {
        let path = entry.path();
        let relative = path.strip_prefix(&descend_root).unwrap_or(&path);
        entry.depth == 0 || !skip(relative, true)
    };
    let threads = thread::available_parallelism().map_or(1, |n| n.get().min(MAX_THREADS));

    dua_core::walk(
        root,
        threads,
        Order::Completion,
        Options::default().skip_metadata(),
        descend,
    )
    .flatten()
    .filter(|entry| entry.depth > 0)
}

#[derive(Default)]
struct TreeBuilder {
    entries_by_parent: HashMap<PathBuf, Vec<VaultEntry>>,
    directories: Vec<PathBuf>,
}

impl TreeBuilder {
    fn add_directory(&mut self, path: PathBuf) {
        self.directories.push(path);
    }

    fn add_note(&mut self, path: PathBuf) -> Result<()> {
        let note = Note::new_unchecked(&note_name(&path)?, &path);
        self.add_to_parent(&path, VaultEntry::File(note));
        Ok(())
    }

    fn build(mut self, root: &Path) -> Result<VaultEntry> {
        let mut deepest_first = std::mem::take(&mut self.directories);
        deepest_first.sort_by_key(|path| Reverse(path.components().count()));

        for path in deepest_first {
            let directory = self.directory(&path)?;
            self.add_to_parent(&path, directory);
        }

        self.directory(root)
    }

    fn directory(&mut self, path: &Path) -> Result<VaultEntry> {
        Ok(VaultEntry::Directory {
            name: directory_name(path)?,
            path: path.to_path_buf(),
            entries: self.entries_by_parent.remove(path).unwrap_or_default(),
        })
    }

    fn add_to_parent(&mut self, path: &Path, entry: VaultEntry) {
        if let Some(parent) = path.parent() {
            self.entries_by_parent
                .entry(parent.to_path_buf())
                .or_default()
                .push(entry);
        }
    }
}

fn directory_name(path: &Path) -> Result<String> {
    to_name(path, path.file_name())
}

fn note_name(path: &Path) -> Result<String> {
    to_name(path, path.file_stem())
}

fn to_name(path: &Path, name: Option<&std::ffi::OsStr>) -> Result<String> {
    name.map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| Error::EmptyFileName(path.to_path_buf()))
}

impl TryFrom<&Path> for VaultEntry {
    type Error = Error;
    fn try_from(value: &Path) -> Result<Self> {
        Self::walk(value, |_, _| false)
    }
}

#[allow(missing_docs)]
pub trait FindNote {
    #[allow(missing_docs)]
    fn find_note<'a>(&'a self, path: &Path) -> Option<&'a Note>;
}

impl FindNote for [VaultEntry] {
    fn find_note<'a>(&'a self, path: &Path) -> Option<&'a Note> {
        self.iter().find_map(|entry| entry.find_note(path))
    }
}

impl FindNote for VaultEntry {
    fn find_note<'a>(&'a self, path: &Path) -> Option<&'a Note> {
        match self {
            VaultEntry::File(note) if note.path() == path => Some(note),
            VaultEntry::Directory {
                entries,
                path: dir_path,
                ..
            } if path.starts_with(dir_path) => entries.find_note(path),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;

    fn names(entries: &[VaultEntry]) -> Vec<&str> {
        let mut names: Vec<&str> = entries.iter().map(VaultEntry::name).collect();
        names.sort();
        names
    }

    #[test]
    fn directory_name_keeps_dots() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("57.02 The First Civilization");
        fs::create_dir(&path).unwrap();

        let entry = VaultEntry::try_from(path.as_path()).unwrap();

        assert_eq!(entry.name(), "57.02 The First Civilization");
    }

    #[test]
    fn note_name_drops_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("57.02 Notes.md");
        fs::write(&path, "").unwrap();

        let entry = VaultEntry::try_from(path.as_path()).unwrap();

        assert_eq!(entry.name(), "57.02 Notes");
    }

    #[test]
    fn walk_does_not_descend_into_skipped_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("skipped/nested")).unwrap();
        fs::write(dir.path().join("skipped/nested/note.md"), "").unwrap();
        fs::write(dir.path().join("kept.md"), "").unwrap();

        let visited = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let recorded = Arc::clone(&visited);
        let skip = move |path: &Path, _| {
            recorded.lock().unwrap().push(path.to_path_buf());
            path == Path::new("skipped")
        };

        let Ok(VaultEntry::Directory { entries, .. }) = VaultEntry::walk(dir.path(), skip) else {
            panic!("expected a directory");
        };

        assert_eq!(names(&entries), ["kept"]);
        let mut visited = visited.lock().unwrap().clone();
        visited.sort();
        visited.dedup();
        assert_eq!(
            visited,
            [PathBuf::from("kept.md"), PathBuf::from("skipped")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn walk_reads_a_symlinked_root() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("vault/folder")).unwrap();
        fs::write(dir.path().join("vault/folder/note.md"), "").unwrap();
        let link = dir.path().join("linked vault");
        symlink(dir.path().join("vault"), &link).unwrap();

        let Ok(VaultEntry::Directory { name, entries, .. }) = VaultEntry::walk(&link, |_, _| false)
        else {
            panic!("expected a directory");
        };

        assert_eq!(name, "linked vault");
        let note = link.join("folder/note.md");
        assert_eq!(
            entries.find_note(&note).map(Note::path),
            Some(note.as_path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn walk_does_not_follow_symlinked_directories() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("folder")).unwrap();
        fs::write(dir.path().join("folder/note.md"), "").unwrap();
        let loop_back_to_vault_root = dir.path().join("folder/loop");
        symlink(dir.path(), loop_back_to_vault_root).unwrap();
        symlink(dir.path().join("folder"), dir.path().join("linked folder")).unwrap();
        symlink(
            dir.path().join("folder/note.md"),
            dir.path().join("linked note.md"),
        )
        .unwrap();
        symlink(
            dir.path().join("missing.md"),
            dir.path().join("dangling.md"),
        )
        .unwrap();

        let Ok(VaultEntry::Directory { entries, .. }) = VaultEntry::walk(dir.path(), |_, _| false)
        else {
            panic!("expected a directory");
        };

        assert_eq!(names(&entries), ["folder", "linked note"]);
        let folder = entries.iter().find(|entry| entry.name() == "folder");
        let Some(VaultEntry::Directory { entries, .. }) = folder else {
            panic!("expected folder to be a directory");
        };
        assert_eq!(names(entries), ["note"]);
    }

    fn relative_paths(entry: &VaultEntry, root: &Path) -> Vec<String> {
        let relative = |path: &Path| path.strip_prefix(root).unwrap().display().to_string();
        match entry {
            VaultEntry::File(note) => vec![relative(note.path())],
            VaultEntry::Directory { path, entries, .. } => {
                let mut paths = vec![format!("{}/", relative(path))];
                paths.extend(entries.iter().flat_map(|entry| relative_paths(entry, root)));
                paths.sort();
                paths
            }
        }
    }

    #[test]
    fn walk_finds_every_entry_four_levels_deep() {
        let dir = tempdir().unwrap();
        let files = [
            "a.md",
            "1.0 level/b.md",
            "1.0 level/.hidden.md",
            "1.0 level/ignored/skipped.md",
            "1.0 level/2 level/c.md",
            "1.0 level/2 level/3 level/d.md",
            "1.0 level/2 level/3 level/4 level/e.md",
            "1.0 level/2 level/3 level/4 level/f.txt",
            ".obsidian/app.json",
        ];
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        let skip = |path: &Path, _| {
            let is_hidden = path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'));
            is_hidden || path == Path::new("1.0 level/ignored")
        };

        let root = VaultEntry::walk(dir.path(), skip).unwrap();

        assert_eq!(
            relative_paths(&root, dir.path()),
            [
                "/",
                "1.0 level/",
                "1.0 level/2 level/",
                "1.0 level/2 level/3 level/",
                "1.0 level/2 level/3 level/4 level/",
                "1.0 level/2 level/3 level/4 level/e.md",
                "1.0 level/2 level/3 level/4 level/f.txt",
                "1.0 level/2 level/3 level/d.md",
                "1.0 level/2 level/c.md",
                "1.0 level/b.md",
                "a.md",
            ]
        );
    }

    #[test]
    #[ignore = "slow: writes 16k notes, run with --run-ignored"]
    fn walk_finds_every_note_in_a_16k_note_vault() {
        let dir = tempdir().unwrap();
        let mut expected = Vec::new();
        for area in 0..20 {
            for category in 0..10 {
                for folder in 0..8 {
                    let folder = dir.path().join(format!(
                        "{area:02} area/{area:02}.{category:02} category/{folder}"
                    ));
                    fs::create_dir_all(&folder).unwrap();
                    for note in 0..10 {
                        let path = folder.join(format!("note {note}.md"));
                        fs::write(&path, "").unwrap();
                        expected.push(path);
                    }
                }
            }
        }

        let started = std::time::Instant::now();
        let root = VaultEntry::walk(dir.path(), |_, _| false).unwrap();
        println!("walked 16k notes in {:?}", started.elapsed());

        let mut found: Vec<PathBuf> = relative_paths(&root, dir.path())
            .into_iter()
            .filter(|path| path.ends_with(".md"))
            .map(|path| dir.path().join(path))
            .collect();
        found.sort();
        expected.sort();
        assert_eq!(found, expected);
    }
}
