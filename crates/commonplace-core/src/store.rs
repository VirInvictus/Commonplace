//! The note store, read side: load a notes folder into memory the way
//! nvALT reads one (`research/nv-source-map.md` §1.1
//! `NotationDirectoryManager`, §4 storage rules) and expose it as a
//! sorted stable view for the query and search phases.
//!
//! Read-side contracts implemented here:
//! - admission: the upstream PlainText extension table, dot-files and
//!   the two upstream metadata filenames skipped
//!   (`NotationPrefs.m:1083-1112`);
//! - display title: derived from the content's first line at load, a
//!   recorded [CP] divergence from upstream's filename-derived load
//!   title (spec §5 row 55);
//! - encoding: UTF-8 always, BOM stripped, invalid UTF-8 recorded as
//!   a per-file warning, never lossily replaced (spec §5 rows 54/56);
//! - no line-ending normalization and no truncation on read.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::note::{Note, NoteBody, NoteTitle};

/// The plain-text extensions admitted at scan time, nvALT's
/// PlainTextFormat set (`NotationPrefs.m:252-262`). Matching is
/// case-insensitive (`pathExtensionAllowed:`,
/// `NotationPrefs.m:1072-1081`). `md` is upstream's default for new
/// notes; as a read-side table every entry admits equally.
pub const PLAIN_TEXT_EXTENSIONS: [&str; 5] = ["md", "txt", "text", "utf8", "taskpaper"];

/// Upstream metadata filenames skipped at scan, matched exactly and
/// case-sensitively (`catalogEntryAllowed:`,
/// `NotationPrefs.m:1093-1098`). A file named `Notes & Settings.md`
/// is a different filename and is admitted like any note.
const SKIPPED_FILENAMES: [&str; 2] = ["Notes & Settings", "Interim Note-Changes"];

/// A non-fatal, per-file problem found while loading a folder. Where
/// the file's metadata was readable the note is still admitted with an
/// empty body, mirroring upstream's blank-note-when-undecodable
/// behavior (`NoteObject.m:588-595`); nothing is ever dropped
/// silently (spec §5 row 56).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWarning {
    /// The file's bytes are not valid UTF-8. Admitted with an empty
    /// body; the derived title is "Untitled Note" (upstream parallel:
    /// `updateFromData:` fails at `NoteObject.m:1577-1580`, and the
    /// blank note `initWithCatalogEntry:` keeps is m:588-595).
    InvalidUtf8 { path: PathBuf },
    /// The file could not be read (permissions, vanished mid-scan).
    /// Admitted with an empty body when stat succeeded; entries that
    /// cannot even be stat-ed are skipped (a broken symlink has no
    /// file to build a note from).
    ReadFailed { path: PathBuf, reason: String },
}

impl fmt::Display for LoadWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadWarning::InvalidUtf8 { path } => {
                write!(f, "not valid UTF-8, loaded empty: {}", path.display())
            }
            LoadWarning::ReadFailed { path, reason } => {
                write!(f, "read failed: {}: {}", path.display(), reason)
            }
        }
    }
}

/// A folder-level load failure: the only fatal read-side error. Every
/// per-file problem degrades to a [`LoadWarning`] instead.
#[derive(Debug)]
pub enum StoreError {
    /// The folder could not be opened or read as a directory (missing,
    /// not a directory, permission denied).
    FolderUnreadable { path: PathBuf, source: io::Error },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::FolderUnreadable { path, source } => {
                write!(f, "notes folder unreadable: {}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::FolderUnreadable { source, .. } => Some(source),
        }
    }
}

/// The in-memory collection of loaded notes.
///
/// `notes` is the arena, kept path-sorted: the sorted stable view of
/// Phase 1, deterministic across loads. Column sorts (title, dates)
/// are Phase 3 and will layer on top of this order (spec §5 row 16).
/// The path index maps each note's file path to its arena slot; note
/// ids stay plain arena indexes until the write phase introduces
/// generation-checked ones (`research/rust-model-notes.md`).
#[derive(Debug, Clone)]
pub struct Store {
    notes: Vec<Note>,
    by_path: HashMap<PathBuf, usize>,
}

impl Store {
    /// The loaded notes, in the sorted stable (path-sorted) order.
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// The note backed by exactly this path, if loaded.
    pub fn find_by_path(&self, path: &Path) -> Option<&Note> {
        self.by_path.get(path).map(|&i| &self.notes[i])
    }
}

/// Everything one [`load_folder`] produced.
#[derive(Debug)]
pub struct LoadOutcome {
    pub store: Store,
    /// Per-file warnings, in path-sorted order (the scan processes
    /// entries in sorted order, so this is deterministic).
    pub warnings: Vec<LoadWarning>,
}

/// Load a notes folder: one flat scan (upstream scans depth-1 only;
/// subfolders arrive with Phase 16), files admitted per the upstream
/// plain-text table, each decoded UTF-8 with the recorded divergence
/// rules.
pub fn load_folder(folder: &Path) -> Result<LoadOutcome, StoreError> {
    let mut entries: Vec<(OsString, PathBuf)> = Vec::new();
    let reader = fs::read_dir(folder).map_err(|source| StoreError::FolderUnreadable {
        path: folder.to_owned(),
        source,
    })?;
    for entry in reader {
        let entry = entry.map_err(|source| StoreError::FolderUnreadable {
            path: folder.to_owned(),
            source,
        })?;
        entries.push((entry.file_name(), entry.path()));
    }
    // Deterministic order: the OS gives no stable order, so entries
    // are processed by file name; notes and warnings then both come
    // out path-sorted. read_dir yields each entry once, so names are
    // unique within the folder.
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut store = Store {
        notes: Vec::new(),
        by_path: HashMap::new(),
    };
    let mut warnings = Vec::new();
    for (name, path) in entries {
        if let Some(note) = load_entry(&name, &path, &mut warnings) {
            store.by_path.insert(note.path.clone(), store.notes.len());
            store.notes.push(note);
        }
    }
    Ok(LoadOutcome { store, warnings })
}

/// Admission and decode for one directory entry. `None` for entries
/// that are not notes: dot-files, the upstream metadata filenames,
/// unadmitted extensions, and anything that is not a regular file.
fn load_entry(name: &OsStr, path: &Path, warnings: &mut Vec<LoadWarning>) -> Option<Note> {
    let name_bytes = name.as_encoded_bytes();
    // Hidden files (catalogEntryAllowed:, NotationPrefs.m:1090-1091).
    if name_bytes.first() == Some(&b'.') {
        return None;
    }
    // The single-DB archive and WAL filenames (m:1093-1098).
    if SKIPPED_FILENAMES
        .iter()
        .any(|skip| name_bytes == skip.as_bytes())
    {
        return None;
    }
    // Extension admission, case-insensitive (the call at
    // NotationPrefs.m:1100 + pathExtensionAllowed: m:1072-1081).
    // Compared on raw bytes so a non-UTF-8 file name with an ASCII
    // extension still admits losslessly. Upstream's second admission
    // channel, legacy Mac OSTypes (m:1103-1108), has no Linux
    // equivalent and is not ported.
    let extension_admitted = path
        .extension()
        .map(|ext| {
            PLAIN_TEXT_EXTENSIONS.iter().any(|allowed| {
                ext.as_encoded_bytes()
                    .eq_ignore_ascii_case(allowed.as_bytes())
            })
        })
        .unwrap_or(false);
    if !extension_admitted {
        return None;
    }
    // Only regular files: a directory named "folder.md" is not a note.
    // Symlinks resolve (a link to a file is a note); a link that
    // cannot be stat-ed is recorded and skipped, since there is no
    // file to build a note from.
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(reason) => {
            warnings.push(LoadWarning::ReadFailed {
                path: path.to_owned(),
                reason: reason.to_string(),
            });
            return None;
        }
    };
    if !meta.is_file() {
        return None;
    }
    // modified() has no failure mode on Linux statx; the UNIX_EPOCH
    // fallback exists for exotic platforms only. Phase 1 never sorts
    // or reconciles on mtime, and the Phase 4 reconcile pass re-stats
    // by path before trusting it, so a placeholder is safe there.
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    // Birth time where the filesystem records one (statx btime);
    // otherwise the modified time stands in (spec row 60).
    let created = meta.created().unwrap_or(mtime);

    // Decode. Stat-able but unreadable or undecodable files are
    // admitted with an empty body and a warning (upstream parallel:
    // the blank note left behind when updateFromData: fails,
    // NoteObject.m:588-595).
    let (body, warning) = match fs::read(path) {
        Ok(bytes) => match NoteBody::from_utf8(&bytes) {
            Ok(body) => (body, None),
            Err(_) => (
                NoteBody::empty(),
                Some(LoadWarning::InvalidUtf8 {
                    path: path.to_owned(),
                }),
            ),
        },
        Err(reason) => (
            NoteBody::empty(),
            Some(LoadWarning::ReadFailed {
                path: path.to_owned(),
                reason: reason.to_string(),
            }),
        ),
    };
    if let Some(warning) = warning {
        warnings.push(warning);
    }
    // The [CP] load-title rule (spec row 55): display title from the
    // content's first line; the file's own name is only its path.
    let title = NoteTitle::from_body(body.as_str());
    let folded_title = crate::fold::fold(title.as_str()).into_boxed_str();
    Some(Note {
        path: path.to_owned(),
        title,
        folded_title,
        body,
        created,
        mtime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_reports_len_and_order() {
        let folder = tempfile("unit-store");
        for name in ["c.md", "a.md", "b.md"] {
            fs::write(folder.path().join(name), "Title\n").unwrap();
        }
        let outcome = load_folder(folder.path()).unwrap();
        let names: Vec<_> = outcome
            .store
            .notes()
            .iter()
            .map(|n| n.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, ["a.md", "b.md", "c.md"]);
        assert_eq!(outcome.store.len(), 3);
        assert!(!outcome.store.is_empty());
    }

    #[test]
    fn find_by_path_hits_and_misses() {
        let folder = tempfile("unit-find");
        let a = folder.write("a.md", b"Alpha\n");
        let outcome = load_folder(folder.path()).unwrap();
        assert_eq!(
            outcome.store.find_by_path(&a).unwrap().title.as_str(),
            "Alpha"
        );
        assert!(
            outcome
                .store
                .find_by_path(Path::new("/nowhere.md"))
                .is_none()
        );
    }

    /// A scratch notes folder under the OS temp dir, removed on drop.
    /// Hand-rolled: the engine stays dependency-free and the test
    /// surface with it.
    pub(crate) struct TempFolder {
        path: PathBuf,
    }

    impl TempFolder {
        pub(crate) fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let unique = format!(
                "commonplace-{}-{}-{}-{}",
                tag,
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0),
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).unwrap();
            TempFolder { path }
        }

        pub(crate) fn path(&self) -> &Path {
            &self.path
        }

        pub(crate) fn write(&self, name: &str, contents: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, contents).unwrap();
            path
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Shorthand so one-line tests stay one line.
    fn tempfile(tag: &str) -> TempFolder {
        TempFolder::new(tag)
    }
}
