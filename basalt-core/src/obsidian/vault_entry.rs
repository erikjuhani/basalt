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
    /// `skip` is called with the path of each directory entry, relative to `path`, before the
    /// entry is read. A skipped directory is never descended into.
    pub(crate) fn walk(path: &Path, skip: &dyn Fn(&Path) -> bool) -> Result<Self> {
        Self::walk_from(path, path, skip)
    }

    fn walk_from(root: &Path, path: &Path, skip: &dyn Fn(&Path) -> bool) -> Result<Self> {
        let name = path
            .with_extension("")
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())
            .ok_or_else(|| Error::EmptyFileName(path.to_path_buf()))?;

        if path.is_file() {
            let note = Note::try_from((name, path.to_path_buf()))?;
            Ok(VaultEntry::File(note))
        } else {
            Ok(VaultEntry::Directory {
                name,
                path: path.to_path_buf(),
                entries: fs::read_dir(path)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| {
                        let path = entry.ok()?.path();
                        if skip(path.strip_prefix(root).unwrap_or(&path)) {
                            return None;
                        }
                        // NOTE: Might want to propagate the walk errors further up
                        Self::walk_from(root, &path, skip).ok()
                    })
                    .collect(),
            })
        }
    }
}

impl TryFrom<&Path> for VaultEntry {
    type Error = Error;
    fn try_from(value: &Path) -> Result<Self> {
        Self::walk(value, &|_| false)
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
        let skip = |path: &Path| {
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
}
