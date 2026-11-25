//! This module provides note backup and restore operations using Obsidian's `.trash` directory.
use std::{
    fs, io,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use crate::obsidian::{Error, Result};

/// Name of the hidden directory Obsidian stores trashed/backed-up notes in.
const TRASH_DIR: &str = ".trash";

/// Returns the backup (`.trash`) directory path for the given vault path.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use basalt_core::obsidian::backup;
///
/// assert_eq!(backup::dir("/vault"), Path::new("/vault/.trash"));
/// ```
pub fn dir<T: AsRef<Path>>(vault_path: T) -> PathBuf {
    vault_path.as_ref().join(TRASH_DIR)
}

/// Lists all backed-up file paths inside `backup_dir`, sorted by path.
///
/// Returns an empty [`Vec`] when `backup_dir` does not exist yet.
///
/// # Errors
///
/// Returns an error if `backup_dir` exists and is not a directory, or if reading the directory
/// fails.
///
/// # Examples
///
/// ```
/// use std::fs;
/// use tempfile::tempdir;
/// use basalt_core::obsidian::backup;
///
/// let tmp_dir = tempdir()?;
/// let backup_dir = backup::dir(tmp_dir.path());
/// assert!(backup::file_entries(&backup_dir)?.is_empty());
///
/// let note_path = tmp_dir.path().join("Note.md");
/// fs::write(&note_path, "hello")?;
/// let backed_up = backup::create(&backup_dir, &note_path)?;
///
/// assert_eq!(backup::file_entries(&backup_dir)?, vec![backed_up]);
/// # Ok::<(), basalt_core::obsidian::Error>(())
/// ```
pub fn file_entries<T: AsRef<Path>>(backup_dir: T) -> Result<Vec<PathBuf>> {
    let backup_dir = backup_dir.as_ref();
    if !backup_dir.try_exists()? {
        return Ok(Vec::new());
    }
    if !backup_dir.is_dir() {
        return Err(Error::Io(io::ErrorKind::NotADirectory.into()));
    }

    let mut entries = Vec::new();
    collect_files(backup_dir, &mut entries)?;
    entries.sort();
    Ok(entries)
}

fn collect_files(dir: &Path, entries: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            collect_files(&path, entries)?;
        } else if file_type.is_file() || (file_type.is_symlink() && path.is_file()) {
            entries.push(path);
        }
    }

    Ok(())
}

/// Creates a backup copy of `filepath` inside `backup_dir` and returns the backup file path.
///
/// Creates `backup_dir` if it does not exist. If a file with the same name already exists in
/// `backup_dir`, a numbered suffix is appended (e.g. `Note 1.md`) instead of overwriting it.
///
/// # Errors
///
/// Returns an error if `filepath` does not exist, is not a file, has an invalid filename, all
/// numbered variants up to 999 already exist ([`Error::MaxAttemptsExceeded`]), or an I/O
/// operation fails.
///
/// # Examples
///
/// ```
/// use std::fs;
/// use tempfile::tempdir;
/// use basalt_core::obsidian::backup;
///
/// let tmp_dir = tempdir()?;
/// let backup_dir = backup::dir(tmp_dir.path());
/// let note_path = tmp_dir.path().join("Note.md");
/// fs::write(&note_path, "content")?;
///
/// let backup_path = backup::create(&backup_dir, &note_path)?;
/// assert_eq!(backup_path, backup_dir.join("Note.md"));
/// assert_eq!(fs::read_to_string(&backup_path)?, "content");
/// # Ok::<(), basalt_core::obsidian::Error>(())
/// ```
pub fn create<B: AsRef<Path>, F: AsRef<Path>>(backup_dir: B, filepath: F) -> Result<PathBuf> {
    let backup_dir = backup_dir.as_ref();
    let filepath = filepath.as_ref();

    ensure_existing_file(filepath)?;
    fs::create_dir_all(backup_dir)?;

    let target_path = available_backup_path(backup_dir, filepath)?;
    fs::copy(filepath, &target_path)?;

    Ok(target_path)
}

/// Restores a backed-up file at `filepath` into `to_dir` and returns the restored file path.
///
/// Creates `to_dir` if it does not exist and moves `filepath` out of the backup directory.
///
/// # Errors
///
/// Returns an error if `filepath` does not exist or is not a file, if a file with the same name
/// already exists in `to_dir`, or if an I/O operation fails.
///
/// # Examples
///
/// ```
/// use std::fs;
/// use tempfile::tempdir;
/// use basalt_core::obsidian::backup;
///
/// let tmp_dir = tempdir()?;
/// let backup_dir = backup::dir(tmp_dir.path());
/// let note_path = tmp_dir.path().join("Note.md");
/// fs::write(&note_path, "content")?;
///
/// let backup_path = backup::create(&backup_dir, &note_path)?;
/// fs::remove_file(&note_path)?;
///
/// let restored = backup::restore(tmp_dir.path(), &backup_path)?;
/// assert_eq!(restored, note_path);
/// assert_eq!(fs::read_to_string(&restored)?, "content");
/// assert!(!backup_path.exists());
/// # Ok::<(), basalt_core::obsidian::Error>(())
/// ```
pub fn restore<T: AsRef<Path>, F: AsRef<Path>>(to_dir: T, filepath: F) -> Result<PathBuf> {
    let to_dir = to_dir.as_ref();
    let filepath = filepath.as_ref();

    ensure_existing_file(filepath)?;

    let file_name = filepath
        .file_name()
        .ok_or_else(|| Error::InvalidPathName(filepath.to_path_buf()))?;
    let dest = to_dir.join(file_name);

    if fs::exists(&dest)? {
        return Err(Error::Io(io::ErrorKind::AlreadyExists.into()));
    }

    fs::create_dir_all(to_dir)?;

    if let Err(err) = fs::rename(filepath, &dest) {
        if err.kind() == io::ErrorKind::CrossesDevices {
            fs::copy(filepath, &dest)?;
            fs::remove_file(filepath)?;
        } else {
            return Err(err.into());
        }
    }

    Ok(dest)
}

fn ensure_existing_file(filepath: &Path) -> Result<()> {
    if !filepath.try_exists()? {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", filepath.display()),
        )));
    }
    if !filepath.is_file() {
        return Err(Error::Io(io::ErrorKind::IsADirectory.into()));
    }
    Ok(())
}

fn available_backup_path(backup_dir: &Path, filepath: &Path) -> Result<PathBuf> {
    let file_name = filepath
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::InvalidPathName(filepath.to_path_buf()))?;

    let candidate = backup_dir.join(file_name);
    if !fs::exists(&candidate)? {
        return Ok(candidate);
    }

    let stem = filepath
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| Error::InvalidPathName(filepath.to_path_buf()))?;
    let ext = filepath.extension().and_then(|ext| ext.to_str());

    const MAX: usize = 999;

    let result = (1..=MAX)
        .map(|n| match ext {
            Some(ext) => format!("{stem} {n}.{ext}"),
            None => format!("{stem} {n}"),
        })
        .try_fold((), |_, name| {
            let path = backup_dir.join(&name);
            match fs::exists(&path).map_err(Error::from) {
                Ok(false) => ControlFlow::Break(Ok(path)),
                Err(err) => ControlFlow::Break(Err(err)),
                Ok(true) => ControlFlow::Continue(()),
            }
        });

    match result {
        ControlFlow::Break(res) => res,
        ControlFlow::Continue(()) => Err(Error::MaxAttemptsExceeded {
            name: file_name.to_string(),
            max_attempts: MAX,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dir_returns_trash_subdir_under_vault() {
        assert_eq!(dir("/tmp/vault"), Path::new("/tmp/vault/.trash"));
    }

    #[test]
    fn file_entries_returns_empty_when_backup_dir_missing() {
        let tmp_dir = tempdir().unwrap();
        let backup_dir = dir(tmp_dir.path());

        assert_eq!(file_entries(&backup_dir).unwrap(), Vec::<PathBuf>::new());
    }

    #[test]
    fn file_entries_lists_nested_files_sorted() {
        let tmp_dir = tempdir().unwrap();
        let backup_dir = dir(tmp_dir.path());
        fs::create_dir_all(backup_dir.join("sub")).unwrap();
        fs::write(backup_dir.join("b.md"), "b").unwrap();
        fs::write(backup_dir.join("sub/a.md"), "a").unwrap();

        assert_eq!(
            file_entries(&backup_dir).unwrap(),
            vec![backup_dir.join("b.md"), backup_dir.join("sub/a.md")]
        );
    }

    #[test]
    fn create_handles_collisions_and_preserves_dots_in_stem() {
        let tmp_dir = tempdir().unwrap();
        let backup_dir = dir(tmp_dir.path());
        let note_path = tmp_dir.path().join("57.02 Notes.md");

        fs::write(&note_path, "first").unwrap();
        let first = create(&backup_dir, &note_path).unwrap();
        assert_eq!(first, backup_dir.join("57.02 Notes.md"));

        fs::write(&note_path, "second").unwrap();
        let second = create(&backup_dir, &note_path).unwrap();
        assert_eq!(second, backup_dir.join("57.02 Notes 1.md"));

        assert_eq!(fs::read_to_string(first).unwrap(), "first");
        assert_eq!(fs::read_to_string(second).unwrap(), "second");
    }

    #[test]
    fn create_errors_on_missing_or_directory_source() {
        let tmp_dir = tempdir().unwrap();
        let backup_dir = dir(tmp_dir.path());

        let missing = tmp_dir.path().join("missing.md");
        assert!(matches!(
            create(&backup_dir, &missing),
            Err(Error::Io(err)) if err.kind() == io::ErrorKind::NotFound
        ));

        let subdir = tmp_dir.path().join("folder");
        fs::create_dir(&subdir).unwrap();
        assert!(matches!(
            create(&backup_dir, &subdir),
            Err(Error::Io(err)) if err.kind() == io::ErrorKind::IsADirectory
        ));
    }

    #[test]
    fn restore_moves_file_and_errors_when_destination_exists() {
        let tmp_dir = tempdir().unwrap();
        let backup_dir = dir(tmp_dir.path());
        let note_path = tmp_dir.path().join("Note.md");
        fs::write(&note_path, "original").unwrap();

        let backed_up = create(&backup_dir, &note_path).unwrap();
        fs::write(&note_path, "conflict").unwrap();

        assert!(matches!(
            restore(tmp_dir.path(), &backed_up),
            Err(Error::Io(err)) if err.kind() == io::ErrorKind::AlreadyExists
        ));
        assert!(backed_up.exists());

        fs::remove_file(&note_path).unwrap();
        let restored = restore(tmp_dir.path(), &backed_up).unwrap();
        assert_eq!(restored, note_path);
        assert_eq!(fs::read_to_string(&restored).unwrap(), "original");
        assert!(!backed_up.exists());
    }
}
