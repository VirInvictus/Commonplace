//! Golden fixture-folder tests for the note-store read side. Every
//! fixture is built at runtime in a scratch folder with exact bytes,
//! so the suite is headless and independent of checkout line-ending
//! mangling. The contract is `spec.md` §5 rows 1-4, 54-56 and the
//! verified upstream tables in `research/nv-source-map.md` §4.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use commonplace_core::store::PLAIN_TEXT_EXTENSIONS;
use commonplace_core::store::{LoadWarning, StoreError, load_folder};

/// A scratch notes folder under the OS temp dir, removed on drop.
struct TempFolder {
    path: PathBuf,
}

impl TempFolder {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "commonplace-golden-{}-{}-{}-{}",
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

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, name: &str, contents: &[u8]) -> PathBuf {
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

fn names(outcome: &commonplace_core::store::LoadOutcome) -> Vec<String> {
    outcome
        .store
        .notes()
        .iter()
        .map(|n| n.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

// --- Extension admission (map §4, NotationPrefs.m:252-271, 1083-1112)

#[test]
fn every_plain_text_extension_is_admitted() {
    let folder = TempFolder::new("exts");
    folder.write("a.md", b"md note\n");
    folder.write("b.txt", b"txt note\n");
    folder.write("c.text", b"text note\n");
    folder.write("d.utf8", b"utf8 note\n");
    folder.write("e.taskpaper", b"taskpaper note:\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(
        names(&outcome),
        ["a.md", "b.txt", "c.text", "d.utf8", "e.taskpaper"]
    );
    assert_eq!(outcome.warnings, vec![]);
}

#[test]
fn extension_table_matches_upstream() {
    assert_eq!(
        PLAIN_TEXT_EXTENSIONS,
        ["md", "txt", "text", "utf8", "taskpaper"]
    );
}

#[test]
fn other_extensions_and_extensionless_files_are_skipped() {
    let folder = TempFolder::new("rejects");
    folder.write("notes.rtf", b"rtf\n");
    folder.write("notes.html", b"<p>html</p>\n");
    folder.write("notes.org", b"org\n");
    folder.write("noext", b"no extension\n");
    folder.write("trailingdot.", b"empty extension\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 0);
    assert_eq!(outcome.warnings, vec![]);
}

#[test]
fn extension_matching_is_case_insensitive() {
    let folder = TempFolder::new("caseins");
    folder.write("UPPER.MD", b"upper\n");
    folder.write("mixed.TxT", b"mixed\n");
    folder.write("task.TASKPAPER", b"task\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(names(&outcome), ["UPPER.MD", "mixed.TxT", "task.TASKPAPER"]);
}

#[test]
fn dot_files_are_skipped() {
    let folder = TempFolder::new("dotfiles");
    folder.write(".hidden.md", b"hidden\n");
    folder.write(".not_a_note", b"plain dotfile\n");
    folder.write("visible.md", b"visible\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(names(&outcome), ["visible.md"]);
}

#[test]
fn upstream_metadata_filenames_are_skipped_exactly() {
    let folder = TempFolder::new("metadata");
    folder.write("Notes & Settings", b"[nvALT database]\n");
    folder.write("Interim Note-Changes", b"[nvALT WAL]\n");
    // The skip is an exact filename match: with an admitted extension
    // this is an ordinary note (catalogEntryAllowed checks equality).
    folder.write("Notes & Settings.md", b"Actually a note\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(names(&outcome), ["Notes & Settings.md"]);
    assert_eq!(outcome.store.notes()[0].title.as_str(), "Actually a note");
}

#[test]
fn a_directory_with_a_note_extension_is_not_a_note() {
    let folder = TempFolder::new("dir-as-note");
    fs::create_dir(folder.path().join("folder.md")).unwrap();
    folder.write("real.md", b"real\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(names(&outcome), ["real.md"]);
    assert_eq!(outcome.warnings, vec![]);
}

#[test]
#[cfg(unix)]
fn symlink_to_a_note_file_is_admitted() {
    use std::os::unix::fs::symlink;
    let folder = TempFolder::new("symlinks");
    let target = folder.write("target.md", b"Linked note\n");
    symlink(&target, folder.path().join("alias.md")).unwrap();
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(names(&outcome), ["alias.md", "target.md"]);
}

#[test]
#[cfg(unix)]
fn broken_symlink_is_recorded_and_skipped() {
    use std::os::unix::fs::symlink;
    let folder = TempFolder::new("broken-link");
    symlink(
        folder.path().join("vanished.md"),
        folder.path().join("dangling.md"),
    )
    .unwrap();
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 0);
    assert_eq!(
        outcome.warnings,
        vec![LoadWarning::ReadFailed {
            path: folder.path().join("dangling.md"),
            reason: outcome
                .warnings
                .first()
                .map(|w| match w {
                    LoadWarning::ReadFailed { reason, .. } => reason.clone(),
                    _ => unreachable!(),
                })
                .unwrap(),
        }]
    );
}

// --- The load-side contract per file

#[test]
fn empty_file_is_an_untitled_empty_note() {
    let folder = TempFolder::new("empty-file");
    folder.write("empty.md", b"");
    let outcome = load_folder(folder.path()).unwrap();
    let note = outcome
        .store
        .find_by_path(&folder.path().join("empty.md"))
        .unwrap();
    assert_eq!(note.title.as_str(), "Untitled Note");
    assert_eq!(note.body.as_str(), "");
}

#[test]
fn file_without_trailing_newline() {
    let folder = TempFolder::new("no-nl");
    folder.write("tight.md", b"Only a title");
    let outcome = load_folder(folder.path()).unwrap();
    let note = &outcome.store.notes()[0];
    assert_eq!(note.title.as_str(), "Only a title");
    assert_eq!(note.body.as_str(), "Only a title");
}

#[test]
fn crlf_file_keeps_its_line_endings() {
    let folder = TempFolder::new("crlf");
    folder.write("crlf.md", b"Hello\r\nworld\r\n");
    let outcome = load_folder(folder.path()).unwrap();
    let note = &outcome.store.notes()[0];
    // Title breaks at the \r; the body keeps the \r\n exactly as read
    // (upstream does no line-ending normalization: map §4).
    assert_eq!(note.title.as_str(), "Hello");
    assert_eq!(note.body.as_str(), "Hello\r\nworld\r\n");
}

#[test]
fn bom_is_stripped_from_body_and_title_derivation() {
    let folder = TempFolder::new("bom");
    folder.write("bom.md", b"\xEF\xBB\xBF# Title\nbody\n");
    let outcome = load_folder(folder.path()).unwrap();
    let note = &outcome.store.notes()[0];
    assert_eq!(note.title.as_str(), "# Title");
    assert_eq!(note.body.as_str(), "# Title\nbody\n");
}

#[test]
fn invalid_utf8_is_admitted_empty_with_a_warning() {
    let folder = TempFolder::new("latin1");
    folder.write("old.md", b"caf\xE9 notes\nbody\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 1);
    let note = &outcome.store.notes()[0];
    assert_eq!(note.title.as_str(), "Untitled Note");
    assert_eq!(note.body.as_str(), "");
    assert_eq!(
        outcome.warnings,
        vec![LoadWarning::InvalidUtf8 {
            path: folder.path().join("old.md"),
        }]
    );
}

/// The UTF-8-always posture (spec §5 row 54) pins this divergence:
/// upstream decodes a UTF-16 BOM file happily
/// (NSData_transformations.m:313-321); Commonplace records it as
/// invalid UTF-8 instead of silently transcoding, because a loader
/// that guesses encodings is exactly what the Linux port removed.
#[test]
fn utf16_bom_file_is_recorded_as_invalid_utf8() {
    let folder = TempFolder::new("utf16");
    folder.write("legacy.md", b"\xFE\xFF\x00H\x00i");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 1);
    assert_eq!(outcome.store.notes()[0].body.as_str(), "");
    assert!(matches!(
        outcome.warnings.as_slice(),
        [LoadWarning::InvalidUtf8 { .. }]
    ));
}

#[test]
#[cfg(unix)]
fn unreadable_file_is_admitted_empty_with_a_warning() {
    use std::os::unix::fs::PermissionsExt;
    let folder = TempFolder::new("unreadable");
    let path = folder.write("secret.md", b"top secret\nbody\n");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&path).is_ok() {
        // Premise failed (running as root): nothing to assert here.
        eprintln!("skipping unreadable-file test: file still readable");
        return;
    }
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 1);
    assert_eq!(outcome.store.notes()[0].body.as_str(), "");
    assert!(matches!(
        outcome.warnings.as_slice(),
        [LoadWarning::ReadFailed { .. }]
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
}

/// The recorded [CP] divergence (spec §5 row 55): display title comes
/// from the content's first line; the file's own name is only its
/// path. Upstream instead titled scanned files by filename minus
/// extension (initWithCatalogEntry:, NoteObject.m:581-584).
#[test]
fn externally_named_file_titles_from_content() {
    let folder = TempFolder::new("external-name");
    folder.write("my-vim-file-name.md", b"Actual Display Title\nbody text\n");
    let outcome = load_folder(folder.path()).unwrap();
    let note = outcome
        .store
        .find_by_path(&folder.path().join("my-vim-file-name.md"))
        .unwrap();
    assert_eq!(note.title.as_str(), "Actual Display Title");
    assert_ne!(note.title.as_str(), "my-vim-file-name");
}

#[test]
#[cfg(unix)]
fn non_utf8_filename_with_admitted_extension_loads_losslessly() {
    use std::os::unix::ffi::OsStrExt;
    let folder = TempFolder::new("raw-name");
    let raw = folder.path.join(std::ffi::OsStr::from_bytes(b"caf\xE9.md"));
    fs::write(&raw, b"Content title\n").unwrap();
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 1);
    let note = outcome.store.find_by_path(&raw).unwrap();
    assert_eq!(note.title.as_str(), "Content title");
}

#[test]
fn mtime_matches_the_file_on_disk() {
    let folder = TempFolder::new("mtime");
    let path = folder.write("stamped.md", b"Stamp\n");
    let outcome = load_folder(folder.path()).unwrap();
    let expected = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(outcome.store.notes()[0].mtime, expected);
}

// --- The sorted stable view

#[test]
fn view_is_path_sorted_regardless_of_creation_order() {
    let folder = TempFolder::new("sorted");
    folder.write("zebra.md", b"Z\n");
    folder.write("apple.md", b"A\n");
    folder.write("mango.md", b"M\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(names(&outcome), ["apple.md", "mango.md", "zebra.md"]);
}

#[test]
fn loading_twice_is_deterministic() {
    let folder = TempFolder::new("determinism");
    folder.write("b.md", b"B\n");
    folder.write("a.md", b"A\n");
    let first = load_folder(folder.path()).unwrap();
    let second = load_folder(folder.path()).unwrap();
    assert_eq!(
        first
            .store
            .notes()
            .iter()
            .map(|n| &n.path)
            .collect::<Vec<_>>(),
        second
            .store
            .notes()
            .iter()
            .map(|n| &n.path)
            .collect::<Vec<_>>()
    );
    assert_eq!(first.warnings, second.warnings);
}

#[test]
fn warnings_come_out_path_sorted() {
    let folder = TempFolder::new("warn-order");
    // created in reverse path order; warnings must still be sorted
    folder.write("zz-latin1.md", b"caf\xE9\n");
    folder.write("aa-latin1.md", b"caf\xE9\n");
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(
        outcome.warnings,
        vec![
            LoadWarning::InvalidUtf8 {
                path: folder.path().join("aa-latin1.md"),
            },
            LoadWarning::InvalidUtf8 {
                path: folder.path().join("zz-latin1.md"),
            },
        ]
    );
}

// --- Folder-level outcomes

#[test]
fn empty_folder_loads_to_an_empty_store() {
    let folder = TempFolder::new("empty-folder");
    let outcome = load_folder(folder.path()).unwrap();
    assert!(outcome.store.is_empty());
    assert_eq!(outcome.store.len(), 0);
    assert_eq!(outcome.warnings, vec![]);
}

#[test]
fn missing_folder_is_a_store_error() {
    let missing = std::env::temp_dir().join("commonplace-golden-does-not-exist");
    let err = load_folder(&missing).unwrap_err();
    match &err {
        StoreError::FolderUnreadable { path, .. } => assert_eq!(path, &missing),
    }
    assert!(err.to_string().contains("notes folder unreadable"));
}

#[test]
fn a_file_path_is_not_a_folder() {
    let folder = TempFolder::new("file-as-folder");
    let path = folder.write("plain.md", b"note\n");
    assert!(matches!(
        load_folder(&path),
        Err(StoreError::FolderUnreadable { .. })
    ));
}

// --- The load benchmark stub (opt-in)

/// Locally skipped by default; opt in with:
/// `cargo test -p commonplace-core --test store_golden -- --ignored load_10k`
/// Phase 3 owns the real latency gate (roadmap); this stub only proves
/// the loader handles the 10k-note folder and returns it sorted.
#[test]
#[ignore = "opt-in load benchmark (10k files)"]
fn load_10k_note_folder_benchmark() {
    let folder = TempFolder::new("bench10k");
    for i in 0..10_000 {
        folder.write(
            &format!("note-{i:05}.md"),
            format!("Note number {i}\nbody line for note {i}\n").as_bytes(),
        );
    }
    let start = std::time::Instant::now();
    let outcome = load_folder(folder.path()).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(outcome.store.len(), 10_000);
    assert!(outcome.warnings.is_empty());
    assert!(
        outcome
            .store
            .notes()
            .windows(2)
            .all(|pair| pair[0].path < pair[1].path)
    );
    println!("loaded 10,000 notes in {elapsed:?}");
}
