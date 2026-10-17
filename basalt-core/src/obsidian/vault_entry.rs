use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{Error, Note, Result};

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
    /// Walks `path` into a [`VaultEntry`], recursing into directories.
    ///
    /// `skip` is called with the path of each directory entry, relative to `path`, and whether
    /// the entry is a directory, before the entry is read. A skipped directory is never
    /// descended into.
    ///
    /// Symbolic links below `path` are not followed into directories: a symlinked file is
    /// included as a note, while a symlinked directory or a dangling symlink is left out. This
    /// keeps the walk inside the vault and safe from symlink loops. `path` itself may be a
    /// symlink.
    pub(crate) fn walk(path: &Path, skip: &dyn Fn(&Path, bool) -> bool) -> Result<Self> {
        Self::walk_from(path, path, !path.is_file(), skip)
    }

    fn walk_from(
        root: &Path,
        path: &Path,
        is_dir: bool,
        skip: &dyn Fn(&Path, bool) -> bool,
    ) -> Result<Self> {
        let name = path
            .with_extension("")
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())
            .ok_or_else(|| Error::EmptyFileName(path.to_path_buf()))?;

        if !is_dir {
            // Note::try_from follows symlinks, so it accepts a symlinked file but rejects a
            // symlinked directory, a dangling symlink or any other non-file.
            let note = Note::try_from((name, path.to_path_buf()))?;
            return Ok(VaultEntry::File(note));
        }

        Ok(VaultEntry::Directory {
            name,
            path: path.to_path_buf(),
            entries: fs::read_dir(path)
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    // DirEntry::file_type does not follow symlinks, so a symlinked directory
                    // is not a directory here.
                    let is_dir = entry.file_type().ok()?.is_dir();
                    if skip(path.strip_prefix(root).unwrap_or(&path), is_dir) {
                        return None;
                    }
                    // NOTE: Might want to propagate the walk errors further up
                    Self::walk_from(root, &path, is_dir, skip).ok()
                })
                .collect(),
        })
    }
}

impl TryFrom<&Path> for VaultEntry {
    type Error = Error;
    fn try_from(value: &Path) -> Result<Self> {
        Self::walk(value, &|_, _| false)
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
    use std::{cell::RefCell, path::PathBuf};
    use tempfile::tempdir;

    fn names(entries: &[VaultEntry]) -> Vec<&str> {
        let mut names: Vec<&str> = entries.iter().map(VaultEntry::name).collect();
        names.sort();
        names
    }

    #[test]
    fn walk_does_not_descend_into_skipped_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("skipped/nested")).unwrap();
        fs::write(dir.path().join("skipped/nested/note.md"), "").unwrap();
        fs::write(dir.path().join("kept.md"), "").unwrap();

        let visited = RefCell::new(Vec::<PathBuf>::new());
        let skip = |path: &Path, _| {
            visited.borrow_mut().push(path.to_path_buf());
            path == Path::new("skipped")
        };

        let Ok(VaultEntry::Directory { entries, .. }) = VaultEntry::walk(dir.path(), &skip) else {
            panic!("expected a directory");
        };

        assert_eq!(names(&entries), ["kept"]);
        let mut visited = visited.into_inner();
        visited.sort();
        assert_eq!(
            visited,
            [PathBuf::from("kept.md"), PathBuf::from("skipped")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn walk_does_not_follow_symlinked_directories() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("folder")).unwrap();
        fs::write(dir.path().join("folder/note.md"), "").unwrap();
        // A loop back to the vault root, which would recurse forever if followed.
        symlink(dir.path(), dir.path().join("folder/loop")).unwrap();
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

        let Ok(VaultEntry::Directory { entries, .. }) = VaultEntry::walk(dir.path(), &|_, _| false)
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
}
