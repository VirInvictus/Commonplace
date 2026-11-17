//! The store's write side: filename derivation with upstream's exact
//! rules (spec row 4, `NotationFileManager.m:412-456`), atomic
//! create/save (row 5: temp file, fsync, `rename(2)` replace), rename
//! with revert-on-failure, the delete boundary (row 7), reconciliation
//! (row 63), and the two-handle write coalescer (row 62).

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::Store;
use crate::fold::fold;
use crate::note::{Note, NoteBody, NoteTitle};

/// The default extension for new notes: upstream's PlainTextFormat
/// default (`NotationPrefs.m:252-262`, `md` force-inserted at index 0
/// by 3.0.1's `ensureMarkdownPathExtension`).
pub const DEFAULT_EXTENSION: &str = "md";

/// Save temporaries are dot-prefixed inside the collection folder:
/// `rename(2)` needs the same directory for atomicity, and the row-1
/// admission skips dot-files, so a crash mid-write can never leave a
/// half-written note that the next scan admits (row 4's divergence
/// note; upstream relied on extension-less random names).
pub const TEMP_PREFIX: &str = ".commonplace-tmp-";

/// The filename cap in bytes, upstream's 255 units over HFS now
/// measured the way Linux measures name limits (row 4).
pub const FILENAME_CAP_BYTES: usize = 255;

/// A write-path failure. Values, never panics: every disk failure is
/// returned, and a failed rename leaves both the disk and the store
/// untouched (`rename(2)` is atomic, so upstream's mid-flow revert
/// (`NoteObject.m:817-823`) collapses into "nothing happened").
#[derive(Debug)]
pub enum WriteError {
    /// No note is loaded under this path.
    NoteNotFound { path: PathBuf },
    /// A rename to an empty (or whitespace-only) title: upstream
    /// rejects it in `_setTitleString:` (`NoteObject.m:793-795`).
    EmptyTitle,
    /// The underlying filesystem refused.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::NoteNotFound { path } => {
                write!(f, "no note loaded at {}", path.display())
            }
            WriteError::EmptyTitle => write!(f, "a note title cannot be empty"),
            WriteError::Io { path, source } => {
                write!(f, "{}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Truncate a title stem to the byte cap minus the reserve, at a char
/// boundary. Upstream: `filenameExpectingAdditionalCharCount:`
/// (`NSString_NV.m:223-229`) with the reserve `3 + ext + 2`
/// (`NotationFileManager.m:431`), counted in UTF-16 units over HFS;
/// here bytes over Linux (row 4).
fn truncate_to_cap(stem: &str, reserve: usize) -> String {
    let budget = FILENAME_CAP_BYTES.saturating_sub(reserve);
    if stem.len() <= budget {
        return stem.to_owned();
    }
    let mut cut = budget;
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    stem[..cut].to_owned()
}

/// Derive the filename for a title, under the row-4 rules:
/// `:` becomes `-`, a leading `.` becomes `_`, the title is truncated
/// to the byte cap minus a reserve of `3 + extension.len() + 2`, and
/// collisions are resolved by `.N` suffixes. `taken` answers whether
/// a candidate stem collides; the caller supplies the store's stems
/// (case-folded: upstream compared case-insensitively,
/// `NotationFileManager.m:444`) plus any disk existence check, and
/// excludes the note itself on rename (`note != aNote`, m:443).
pub fn derive_filename(
    title: &str,
    extension: &str,
    mut taken: impl FnMut(&str) -> bool,
) -> String {
    let mut stem = title.replace(':', "-");
    if stem.starts_with('.') {
        stem.replace_range(0..1, "_");
    }
    let reserve = 3 + extension.len() + 2;
    let stem = truncate_to_cap(&stem, reserve);

    let mut candidate = stem.clone();
    let mut iteration = 0u32;
    while taken(&candidate) {
        iteration += 1;
        candidate = format!("{stem}.{iteration}");
    }
    format!("{candidate}.{extension}")
}

impl Store {
    /// Rename a note to a new title: the filename derives under the
    /// row-4 rules (the note's own extension is kept when still
    /// admitted, per the keep rule at `NotationFileManager.m:423-428`),
    /// `rename(2)` moves the file, and the CONTENT'S FIRST LINE is
    /// rewritten to the new title so the rename survives a reload
    /// (row 55 derives display titles from content; upstream kept the
    /// title in its database and never touched content, a difference
    /// the files-first model cannot afford). The atomic chain means a
    /// failed rename reverts by construction: upstream's mid-flow
    /// revert (`NoteObject.m:817-823`) could leave title and file
    /// desynced; here nothing happens.
    ///
    /// A rename whose new title differs only by case is a real rename
    /// (upstream compared case-sensitively, `_setTitleString:`
    /// m:794-795); an identical title is a no-op.
    pub fn rename_note(&mut self, path: &Path, new_title: &str) -> Result<Note, WriteError> {
        if new_title.trim().is_empty() {
            return Err(WriteError::EmptyTitle);
        }
        let index = match self.index_of(path) {
            Some(index) => index,
            None => {
                return Err(WriteError::NoteNotFound {
                    path: path.to_owned(),
                });
            }
        };
        let old_display_title = self.notes[index].title.as_str().to_owned();
        if new_title == old_display_title {
            return Ok(self.notes[index].clone());
        }
        let old_path = self.notes[index].path.clone();

        // Keep the note's own extension when it is still admitted
        // (m:423-428); new names otherwise take the default.
        let extension = old_path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .filter(|e| {
                crate::store::PLAIN_TEXT_EXTENSIONS
                    .iter()
                    .any(|allowed| e.eq_ignore_ascii_case(allowed))
            })
            .unwrap_or_else(|| DEFAULT_EXTENSION.to_owned());
        let stems = self.taken_stems(Some(&old_path));
        let folder = self.folder.clone();
        let taken = |candidate: &str| {
            let folded_candidate = candidate.to_lowercase();
            stems.contains(&folded_candidate)
                || folder.join(format!("{candidate}.{extension}")).exists()
        };
        let filename = derive_filename(new_title, &extension, taken);
        let new_path = folder.join(&filename);
        let final_path = new_path.clone();

        if new_path != old_path {
            fs::rename(&old_path, &new_path).map_err(|source| WriteError::Io {
                path: old_path.clone(),
                source,
            })?;
        }

        // Rewrite the content's first line; everything after it,
        // including its line-ending style, stays byte for byte.
        let body = self.notes[index].body.as_str();
        let rest = match body.split_once('\n') {
            Some((_, rest)) => rest.to_owned(),
            None => String::new(),
        };
        let new_content = format!("{new_title}\n{rest}");
        atomic_write(&new_path, new_content.as_bytes())?;

        let note = &mut self.notes[index];
        note.path = new_path;
        note.body = NoteBody::of(&new_content);
        note.title = NoteTitle::from_body(note.body.as_str());
        note.folded_title = fold(note.title.as_str()).into_boxed_str();
        if let Ok(meta) = fs::metadata(&note.path) {
            if let Ok(mtime) = meta.modified() {
                note.mtime = mtime;
            }
            if let Ok(created) = meta.created() {
                note.created = created;
            }
        }
        self.reindex();
        // Row 65's [CP] improvement: inbound [[links]] matching the
        // old title fold to the new title. The extractor lands with
        // Phase 5; the hookup call is already here.
        self.rewrite_inbound_links(&old_display_title, new_title);
        match self.index_of(&final_path) {
            Some(i) => Ok(self.notes[i].clone()),
            None => Err(WriteError::NoteNotFound { path: final_path }),
        }
    }

    /// The rename-time link rewrite, called by [`Store::rename_note`]
    /// after a successful rename: notes whose bodies carry
    /// `[[links]]` matching the old display title (folded) are
    /// rewritten to the new title and atomically saved. The extractor
    /// lands with Phase 5 (row 65); until then the hookup is inert.
    pub(crate) fn rewrite_inbound_links(&mut self, _old_title: &str, _new_title: &str) {}

    /// Delete a note through the injected removal sink: the engine
    /// never links a trash dependency (row 7), so the caller decides
    /// freedesktop-trash (the app crate wires the `trash` crate at
    /// Phase 10+) or plain unlink (the CLI may). A sink failure
    /// leaves the note loaded; a success removes it from the arena.
    pub fn delete_note(
        &mut self,
        path: &Path,
        remove: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), WriteError> {
        let index = match self.index_of(path) {
            Some(index) => index,
            None => {
                return Err(WriteError::NoteNotFound {
                    path: path.to_owned(),
                });
            }
        };
        let disk_path = self.notes[index].path.clone();
        remove(&disk_path).map_err(|source| WriteError::Io {
            path: disk_path,
            source,
        })?;
        self.notes.remove(index);
        self.reindex();
        Ok(())
    }

    /// Create a note: the file's content is the title as the first
    /// line followed by the body, so a reload re-derives the same
    /// display title (row 55's content-derived rule). The filename
    /// comes from [`derive_filename`] against the store's stems and
    /// the folder's disk state; the write is atomic (row 5).
    pub fn create_note(&mut self, title: &str, body: &str) -> Result<Note, WriteError> {
        if title.trim().is_empty() {
            return Err(WriteError::EmptyTitle);
        }
        let stems = self.taken_stems(None);
        let folder = self.folder.clone();
        let taken = |candidate: &str| {
            let folded_candidate = candidate.to_lowercase();
            stems.contains(&folded_candidate)
                || folder
                    .join(format!("{candidate}.{DEFAULT_EXTENSION}"))
                    .exists()
        };
        let filename = derive_filename(title, DEFAULT_EXTENSION, taken);
        let path = folder.join(&filename);

        let content = format!("{title}\n{body}");
        atomic_write(&path, content.as_bytes())?;

        let note = build_note(
            path.clone(),
            &NoteBody::from_utf8(content.as_bytes()).unwrap(),
        );
        self.notes.push(note.clone());
        self.reindex();
        Ok(note)
    }

    /// Save a new body over an existing note: temp file, fsync,
    /// `rename(2)` replace (row 5), then the in-memory note updates
    /// from a re-stat. The display title re-derives from the new
    /// body's first line (row 55's invariant: the list always shows
    /// the content's first line) while the FILENAME stays put:
    /// upstream applied the first-line title at creation/import/sync
    /// only and never renamed on body edits (map section 4). Callers
    /// that also re-tag or re-link must re-run their indexes; a
    /// structural consumer (the Phase 3 filter state) must be reset.
    pub fn save_note(&mut self, path: &Path, new_body: &str) -> Result<Note, WriteError> {
        let index = match self.index_of(path) {
            Some(index) => index,
            None => {
                return Err(WriteError::NoteNotFound {
                    path: path.to_owned(),
                });
            }
        };
        atomic_write(path, new_body.as_bytes())?;
        let meta = fs::metadata(path).map_err(|source| WriteError::Io {
            path: path.to_owned(),
            source,
        })?;
        let note = &mut self.notes[index];
        note.body = NoteBody::of(new_body);
        note.title = NoteTitle::from_body(note.body.as_str());
        note.folded_title = fold(note.title.as_str()).into_boxed_str();
        note.created = meta
            .created()
            .unwrap_or(meta.modified().unwrap_or(SystemTime::UNIX_EPOCH));
        note.mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        Ok(note.clone())
    }
}

/// Atomic write per row 5: dot-prefixed temp in the same directory,
/// write + `sync_all` (fsync) before the rename, then `rename(2)`,
/// whose replace is atomic on Linux. A crash before the rename leaves
/// only the invisible temp.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), WriteError> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp = dir.join(format!("{TEMP_PREFIX}{}-{unique}.tmp", std::process::id()));
    let write = |temp: &Path| -> io::Result<()> {
        let mut file = fs::File::create(temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    };
    write(&temp)
        .and_then(|()| fs::rename(&temp, path))
        .map_err(|source| {
            let _ = fs::remove_file(&temp);
            WriteError::Io {
                path: path.to_owned(),
                source,
            }
        })
}

/// Build an in-memory `Note` from a file on disk: the shared tail of
/// load and create.
fn build_note(path: PathBuf, body: &NoteBody) -> Note {
    let title = NoteTitle::from_body(body.as_str());
    let meta = fs::metadata(&path).ok();
    let now = SystemTime::UNIX_EPOCH;
    Note {
        path,
        folded_title: fold(title.as_str()).into_boxed_str(),
        title,
        body: body.clone(),
        created: meta.as_ref().and_then(|m| m.created().ok()).unwrap_or(now),
        mtime: meta.as_ref().and_then(|m| m.modified().ok()).unwrap_or(now),
        size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
    }
}

/// What one reconciliation pass changed (row 63). Paths are the
/// collection-folder paths as loaded.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    /// Paths whose `(mtime, size)` changed on disk: the note was
    /// re-read and replaced.
    pub updated: Vec<PathBuf>,
    /// Load warnings from the freshly scanned entries.
    pub warnings: Vec<crate::store::LoadWarning>,
}

impl Store {
    /// Reconcile the store against the folder (row 63): one fresh
    /// admission-controlled scan, diffed by path. New paths join,
    /// vanished paths leave, and a path whose `(mtime, size)` moved is
    /// re-read from disk - upstream's change triple
    /// (`NotationDirectoryManager.m:358-360`) minus the attribute-mod
    /// leg, which has no Linux analog. `rename(2)`-based external
    /// saves arrive here as remove+add; `statx` btime survives, so
    /// `created` stays stable across such renames. The inotify layer
    /// (Phase 10) calls this as its fallback truth.
    pub fn reconcile(&mut self, folder: &Path) -> Result<ReconcileReport, super::StoreError> {
        let outcome = crate::store::load_folder(folder)?;
        let mut report = ReconcileReport {
            warnings: outcome.warnings,
            ..ReconcileReport::default()
        };

        // Diff by path: what the fresh scan sees vs what we hold.
        let mut kept: Vec<Note> = Vec::with_capacity(outcome.store.notes.len());
        let mut fresh_paths: Vec<PathBuf> = Vec::with_capacity(outcome.store.len());
        for fresh in outcome.store.notes() {
            fresh_paths.push(fresh.path.clone());
            match self.index_of(&fresh.path) {
                None => {
                    report.added.push(fresh.path.clone());
                    kept.push(fresh.clone());
                }
                Some(i) => {
                    let current = &self.notes[i];
                    if current.mtime != fresh.mtime || current.size != fresh.size {
                        report.updated.push(fresh.path.clone());
                        kept.push(fresh.clone());
                    } else {
                        // Nothing moved on disk; keep ours (equal by
                        // the key, and ours avoids re-decoding).
                        kept.push(current.clone());
                    }
                }
            }
        }
        for current in self.notes() {
            if !fresh_paths.contains(&current.path) {
                report.removed.push(current.path.clone());
            }
        }

        // The fresh store is the truth; adopt its arena and folder.
        self.notes = outcome.store.notes;
        self.by_path = outcome.store.by_path;
        self.folder = outcome.store.folder;
        Ok(report)
    }
}

/// The two-handle write coalescer (row 62), the port of upstream's
/// asymmetry (`NotationController.m:1082-1100`): an edit
/// cancel-and-rearms the 2.7 s write debounce but NEVER the 15 s
/// cadence timer ("next user change always invalidates queued write
/// from performSelector, but not queued write from timer", the
/// source comment at m:1086), so typing coalesces yet every change
/// reaches disk within 15 s of the first unsaved one. Pure state over
/// injected `Instant`s: headless to test, the app drives it with
/// `Instant::now()`.
#[derive(Debug, Default, Clone)]
pub struct WriteCoalescer {
    edit_deadline: Option<std::time::Instant>,
    cadence_deadline: Option<std::time::Instant>,
}

/// Which handle a [`WriteCoalescer::tick`] fired. Either means "flush
/// the pending writes now"; both handles clear with the fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceFire {
    /// The edit debounce ran out: the user stopped typing for 2.7 s.
    EditDebounce,
    /// The cadence ran out: 15 s elapsed since the first unsaved
    /// edit, regardless of ongoing typing.
    Cadence,
}

impl WriteCoalescer {
    /// Upstream: `performSelector ... afterDelay:2.7`
    /// (NotationController.m:1100).
    pub const EDIT_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(2700);
    /// Upstream: the `changeWritingTimer` at 15.0 s
    /// (NotationController.m:1082-1084).
    pub const CADENCE: std::time::Duration = std::time::Duration::from_secs(15);

    /// A user edit: rearm the debounce (cancelling any queued
    /// debounce write), and start the cadence if it is not already
    /// running (an edit never postpones a running cadence).
    pub fn note_edited(&mut self, now: std::time::Instant) {
        self.edit_deadline = Some(now + Self::EDIT_DEBOUNCE);
        if self.cadence_deadline.is_none() {
            self.cadence_deadline = Some(now + Self::CADENCE);
        }
    }

    /// Advance the clock: `Some(fire)` when a handle is due (flush
    /// now; both handles clear), `None` while both wait.
    pub fn tick(&mut self, now: std::time::Instant) -> Option<CoalesceFire> {
        let edit_due = self.edit_deadline.map(|d| d <= now);
        let cadence_due = self.cadence_deadline.map(|d| d <= now);
        let fire = match (edit_due, cadence_due) {
            (Some(true), Some(true)) => {
                let cadence_first = self.cadence_deadline.unwrap() < self.edit_deadline.unwrap();
                if cadence_first {
                    CoalesceFire::Cadence
                } else {
                    CoalesceFire::EditDebounce
                }
            }
            (Some(true), _) => CoalesceFire::EditDebounce,
            (_, Some(true)) => CoalesceFire::Cadence,
            _ => return None,
        };
        self.clear();
        Some(fire)
    }

    /// A completed flush (the caller wrote everything through): both
    /// handles clear, matching upstream's post-write timer
    /// invalidation.
    pub fn flushed(&mut self) {
        self.clear();
    }

    fn clear(&mut self) {
        self.edit_deadline = None;
        self.cadence_deadline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taken_of<'a>(existing: &[&'a str]) -> impl FnMut(&str) -> bool + 'a {
        let owned: Vec<String> = existing.iter().map(|s| s.to_lowercase()).collect();
        move |candidate: &str| owned.iter().any(|stem| *stem == candidate.to_lowercase())
    }

    // Golden tables for the row-4 derivation.

    #[test]
    fn colon_becomes_hyphen() {
        assert_eq!(derive_filename("a:b:c", "md", taken_of(&[])), "a-b-c.md");
    }

    #[test]
    fn leading_dot_becomes_underscore() {
        assert_eq!(
            derive_filename(".hidden", "md", taken_of(&[])),
            "_hidden.md"
        );
    }

    #[test]
    fn first_collision_takes_dot_one() {
        assert_eq!(
            derive_filename("hello", "md", taken_of(&["hello"])),
            "hello.1.md"
        );
    }

    #[test]
    fn stacked_collisions_increment() {
        assert_eq!(
            derive_filename("hello", "md", taken_of(&["hello", "hello.1", "hello.2"])),
            "hello.3.md"
        );
    }

    #[test]
    fn collisions_are_case_insensitive() {
        assert_eq!(
            derive_filename("hello", "md", taken_of(&["HELLO"])),
            "hello.1.md"
        );
        assert_eq!(
            derive_filename("HELLO", "md", taken_of(&["hello"])),
            "HELLO.1.md"
        );
    }

    #[test]
    fn long_titles_truncate_to_the_byte_cap_minus_reserve() {
        // ASCII: cap 255, reserve 3 + 2 + 2 = 7, stem budget 248
        let long = "a".repeat(400);
        let name = derive_filename(&long, "md", taken_of(&[]));
        let stem = name.strip_suffix(".md").unwrap();
        assert_eq!(stem.len(), 248);
        assert_eq!(name.len(), 248 + 3);
    }

    #[test]
    fn truncation_never_splits_a_char() {
        // 130 two-byte chars: byte budget 248 lands mid-char; the cut
        // must back up to a boundary
        let long = "é".repeat(130);
        let name = derive_filename(&long, "md", taken_of(&[]));
        let stem = name.strip_suffix(".md").unwrap();
        assert!(stem.chars().all(|c| c == 'é'));
        assert!(stem.len() <= 248);
        assert!(stem.len() + 2 > 246);
    }

    #[test]
    fn unicode_survives_sanitization() {
        assert_eq!(
            derive_filename("café_notes", "md", taken_of(&[])),
            "café_notes.md"
        );
    }

    // Create and save, against a real scratch folder.

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "commonplace-write-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_writes_title_first_body_and_indexes_it() {
        let dir = scratch("create");
        let outcome = crate::store::load_folder(&dir).unwrap();
        let mut store = outcome.store;
        let note = store
            .create_note("My New Note", "the body text\nline two")
            .unwrap();
        assert_eq!(note.title.as_str(), "My New Note");
        let on_disk = fs::read_to_string(&note.path).unwrap();
        assert_eq!(on_disk, "My New Note\nthe body text\nline two");
        // Reloaded from disk, the note is identical: the file IS it.
        let reloaded = crate::store::load_folder(&dir).unwrap();
        assert_eq!(reloaded.store.len(), 1);
        assert_eq!(reloaded.store.notes()[0].title.as_str(), "My New Note");
        // And the store sees it without a reload.
        assert_eq!(store.find_by_path(&note.path).unwrap().path, note.path);
        assert_eq!(store.len(), 1);
        cleanup(&dir);
    }

    #[test]
    fn create_collides_with_loaded_and_disk_names() {
        let dir = scratch("collide");
        let outcome = crate::store::load_folder(&dir).unwrap();
        let mut store = outcome.store;
        let first = store.create_note("Agenda", "one").unwrap();
        let second = store.create_note("Agenda", "two").unwrap();
        assert_eq!(
            first.path.file_name().unwrap().to_str().unwrap(),
            "Agenda.md"
        );
        assert_eq!(
            second.path.file_name().unwrap().to_str().unwrap(),
            "Agenda.1.md"
        );
        // A store that has not loaded the folder still respects the
        // disk: a fresh store colliding with the file on disk.
        let fresh = crate::store::load_folder(&dir).unwrap();
        let mut fresh_store = fresh.store;
        let third = fresh_store.create_note("Agenda", "three").unwrap();
        assert_eq!(
            third.path.file_name().unwrap().to_str().unwrap(),
            "Agenda.2.md"
        );
        cleanup(&dir);
    }

    #[test]
    fn create_rejects_empty_titles() {
        let dir = scratch("empty");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        assert!(matches!(
            store.create_note("   ", "body"),
            Err(WriteError::EmptyTitle)
        ));
        cleanup(&dir);
    }

    #[test]
    fn save_replaces_atomically_and_rederives_the_title() {
        let dir = scratch("save");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store.create_note("First Title", "original body").unwrap();
        let saved = store
            .save_note(&note.path, "Second Title\nnew body")
            .unwrap();
        assert_eq!(saved.title.as_str(), "Second Title");
        assert_eq!(saved.body.as_str(), "Second Title\nnew body");
        // The file name did not move (map section 4: body edits never
        // rename), and the store's index still finds it.
        assert_eq!(
            note.path.file_name().unwrap().to_str().unwrap(),
            "First Title.md"
        );
        assert!(store.find_by_path(&note.path).is_some());
        // The disk holds exactly the new bytes.
        assert_eq!(
            fs::read_to_string(&note.path).unwrap(),
            "Second Title\nnew body"
        );
        // No dot-temp survives the write.
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(TEMP_PREFIX))
            .collect();
        assert!(leftovers.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn save_unknown_path_is_an_error_not_a_create() {
        let dir = scratch("save404");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        assert!(matches!(
            store.save_note(&dir.join("ghost.md"), "x"),
            Err(WriteError::NoteNotFound { .. })
        ));
        cleanup(&dir);
    }

    /// Crash simulation: leave a dot-prefixed temp behind (what a
    /// kill between write and rename produces) and prove the scan
    /// never admits it while the real note survives untouched.
    #[test]
    fn crash_mid_write_leaves_no_admissible_wreckage() {
        let dir = scratch("crash");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store
            .create_note("Stable", "content that survives")
            .unwrap();
        // The wreckage of a crash between write_all and rename:
        fs::write(
            dir.join(format!("{TEMP_PREFIX}orphan-1.tmp")),
            "half written garbage\u{EF}",
        )
        .unwrap();
        let reloaded = crate::store::load_folder(&dir).unwrap();
        assert_eq!(reloaded.store.len(), 1);
        // The real note's full file text survives byte for byte.
        assert_eq!(
            reloaded
                .store
                .find_by_path(&note.path)
                .unwrap()
                .body
                .as_str(),
            "Stable\ncontent that survives"
        );
        assert!(reloaded.warnings.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn rename_moves_the_file_and_rewrites_the_first_line() {
        let dir = scratch("rename");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store.create_note("First Title", "original body").unwrap();
        let renamed = store.rename_note(&note.path, "Renamed Title").unwrap();
        // Old path gone from disk and store; new path holds the new
        // first line over the untouched rest.
        assert!(!note.path.exists());
        assert_eq!(
            renamed.path.file_name().unwrap().to_str().unwrap(),
            "Renamed Title.md"
        );
        assert_eq!(
            fs::read_to_string(&renamed.path).unwrap(),
            "Renamed Title\noriginal body"
        );
        assert_eq!(renamed.title.as_str(), "Renamed Title");
        assert!(store.find_by_path(&note.path).is_none());
        assert_eq!(
            store.find_by_path(&renamed.path).unwrap().body.as_str(),
            "Renamed Title\noriginal body"
        );
        cleanup(&dir);
    }

    #[test]
    fn rename_keeps_an_admitted_extension() {
        let dir = scratch("rename-ext");
        fs::write(dir.join("Paper.txt"), "Paper Title\nthe text").unwrap();
        let outcome = crate::store::load_folder(&dir).unwrap();
        let mut store = outcome.store;
        let renamed = store
            .rename_note(&dir.join("Paper.txt"), "New Paper Name")
            .unwrap();
        assert_eq!(
            renamed.path.file_name().unwrap().to_str().unwrap(),
            "New Paper Name.txt"
        );
        cleanup(&dir);
    }

    #[test]
    fn rename_excludes_itself_from_collisions() {
        // A case-only rename ("Agenda" -> "AGENDA") must not collide
        // with itself; upstream excluded the note (m:443).
        let dir = scratch("rename-self");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store.create_note("Agenda", "items").unwrap();
        let renamed = store.rename_note(&note.path, "AGENDA").unwrap();
        assert_eq!(
            renamed.path.file_name().unwrap().to_str().unwrap(),
            "AGENDA.md"
        );
        cleanup(&dir);
    }

    #[test]
    fn rename_rejects_an_empty_title() {
        let dir = scratch("rename-empty");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store.create_note("Keep", "me").unwrap();
        assert!(matches!(
            store.rename_note(&note.path, "   "),
            Err(WriteError::EmptyTitle)
        ));
        // Nothing changed.
        assert!(note.path.exists());
        assert!(store.find_by_path(&note.path).is_some());
        cleanup(&dir);
    }

    #[test]
    fn a_failed_rename_reverts_by_construction() {
        let dir = scratch("rename-fail");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store.create_note("Solid", "content").unwrap();
        // Deterministic failure injection: the collection folder
        // vanishes, so rename(2) gets ENOENT.
        fs::remove_dir_all(&dir).unwrap();
        assert!(matches!(
            store.rename_note(&note.path, "Moved Somewhere"),
            Err(WriteError::Io { .. })
        ));
        // The store's memory is untouched: the note is still loaded
        // under the old path with the old title.
        let still = store.find_by_path(&note.path).unwrap();
        assert_eq!(still.title.as_str(), "Solid");
        cleanup(&dir);
    }

    #[test]
    fn delete_goes_through_the_injected_sink() {
        let dir = scratch("delete");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let keep = store.create_note("Keeper", "stays").unwrap();
        let gone = store.create_note("Goner", "leaves").unwrap();
        let mut seen: Option<PathBuf> = None;
        store
            .delete_note(&gone.path, |p| {
                seen = Some(p.to_owned());
                fs::remove_file(p)
            })
            .unwrap();
        // The sink saw the note's disk path; the file is gone; the
        // store keeps only the survivor.
        assert_eq!(seen, Some(gone.path.clone()));
        assert!(!gone.path.exists());
        assert_eq!(store.len(), 1);
        assert!(store.find_by_path(&keep.path).is_some());
        cleanup(&dir);
    }

    #[test]
    fn reconcile_reports_added_removed_and_updated() {
        let dir = scratch("reconcile");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        fs::write(dir.join("a.md"), "alpha\noriginal").unwrap();
        fs::write(dir.join("b.md"), "bravo\ndoomed").unwrap();
        store.reconcile(&dir).unwrap();
        assert_eq!(store.len(), 2);

        // External edits: b.md vanishes, a.md grows, c.md appears.
        // Sleep long enough that statx mtime cannot tie with the
        // loaded state.
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(dir.join("a.md"), "alpha\noriginal\nnow longer").unwrap();
        fs::remove_file(dir.join("b.md")).unwrap();
        fs::write(dir.join("c.md"), "charlie\narrived").unwrap();
        let report = store.reconcile(&dir).unwrap();
        let names = |v: &[PathBuf]| {
            let mut v: Vec<String> = v
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            v.sort();
            v
        };
        assert_eq!(names(&report.added), ["c.md"]);
        assert_eq!(names(&report.removed), ["b.md"]);
        assert_eq!(names(&report.updated), ["a.md"]);
        // The store is the folder's truth afterwards.
        assert_eq!(store.len(), 2);
        assert!(store.find_by_path(&dir.join("c.md")).is_some());
        assert!(store.find_by_path(&dir.join("b.md")).is_none());
        assert_eq!(
            store.find_by_path(&dir.join("a.md")).unwrap().body.as_str(),
            "alpha\noriginal\nnow longer"
        );
        cleanup(&dir);
    }

    #[test]
    fn reconcile_keeps_untouched_notes_and_reports_warnings() {
        let dir = scratch("reconcile-warn");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        fs::write(dir.join("ok.md"), "fine\nstays").unwrap();
        store.reconcile(&dir).unwrap();
        // An undecodable file arrives; the untouched note reports
        // nothing, the bad one warns.
        fs::write(dir.join("bad.md"), b"caf\xE9 latin-1 bytes").unwrap();
        let report = store.reconcile(&dir).unwrap();
        assert_eq!(report.updated, Vec::<PathBuf>::new());
        assert_eq!(report.added, vec![dir.join("bad.md")]);
        assert_eq!(
            report.warnings,
            vec![crate::store::LoadWarning::InvalidUtf8 {
                path: dir.join("bad.md"),
            }]
        );
        assert_eq!(
            store
                .find_by_path(&dir.join("ok.md"))
                .unwrap()
                .body
                .as_str(),
            "fine\nstays"
        );
        cleanup(&dir);
    }

    // The two-handle coalescing matrix (row 62, upstream
    // NotationController.m:1082-1100). All instants are offsets from
    // ONE captured base: no real clock reads inside the assertions.

    use std::time::{Duration, Instant};

    fn coalesce_tests() -> (Instant, impl Fn(u64) -> Instant) {
        let base = Instant::now();
        (base, move |secs: u64| base + Duration::from_secs(secs))
    }

    #[test]
    fn edits_rearm_the_debounce_and_never_the_cadence() {
        let (_t0, at) = coalesce_tests();
        let mut c = WriteCoalescer::default();
        c.note_edited(at(0));
        // Another edit 1 s later pushes the debounce out but cannot
        // postpone the cadence that started at t0.
        c.note_edited(at(1));
        // 2.7 s after the LAST edit is t0+3.7: not yet at t0+3.
        assert_eq!(c.tick(at(3)), None);
        // And the debounce (not the cadence at t0+15) is what fires.
        assert_eq!(c.tick(at(4)), Some(CoalesceFire::EditDebounce));
    }

    #[test]
    fn the_cadence_fires_despite_continuous_typing() {
        let (_t0, at) = coalesce_tests();
        let mut c = WriteCoalescer::default();
        // One edit every second for 14 s: the debounce keeps sliding
        // (last deadline t0+16.7), the cadence cannot move.
        for s in 0..14 {
            c.note_edited(at(s));
            assert_eq!(c.tick(at(s)), None);
        }
        // t0+15 is the cadence deadline itself: due, inclusive.
        assert_eq!(c.tick(at(15)), Some(CoalesceFire::Cadence));
        // The fire cleared both handles, so nothing more.
        assert_eq!(c.tick(at(16)), None);
    }

    #[test]
    fn a_fire_clears_both_handles() {
        let (_t0, at) = coalesce_tests();
        let mut c = WriteCoalescer::default();
        c.note_edited(at(0));
        assert_eq!(
            c.tick(at(3) + Duration::from_millis(1)),
            Some(CoalesceFire::EditDebounce)
        );
        // Both gone: nothing fires afterwards without a new edit.
        assert_eq!(c.tick(at(60)), None);
        // A fresh edit re-arms BOTH (the cadence had cleared). Here
        // both handles come due together at t0+75; the tie fires the
        // earlier deadline (the debounce) - the flush is identical.
        c.note_edited(at(60));
        assert_eq!(c.tick(at(75)), Some(CoalesceFire::EditDebounce));
    }

    #[test]
    fn flushed_clears_like_a_fire() {
        let (_t0, at) = coalesce_tests();
        let mut c = WriteCoalescer::default();
        c.note_edited(at(0));
        c.flushed();
        assert_eq!(c.tick(at(60)), None);
    }

    #[test]
    fn a_failing_sink_keeps_the_note_loaded() {
        let dir = scratch("delete-fail");
        let mut store = crate::store::load_folder(&dir).unwrap().store;
        let note = store.create_note("Stays Put", "content").unwrap();
        let sink_error = || -> io::Result<()> { Err(io::Error::other("trash refused")) };
        assert!(matches!(
            store.delete_note(&note.path, |_| sink_error()),
            Err(WriteError::Io { .. })
        ));
        assert!(store.find_by_path(&note.path).is_some());
        cleanup(&dir);
    }
}
