//! Golden tests for the write path and the tag/link layer against a
//! real scratch folder: the behaviors that span modules (rename
//! rewriting inbound links, backlink indexes under rename, conflict
//! admission, the Title: override at load, transclusion cycles).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use commonplace_core::store::{LoadWarning, load_folder};

/// A scratch notes folder under the OS temp dir, removed on drop.
struct TempFolder {
    path: PathBuf,
}

impl TempFolder {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "commonplace-write-golden-{}-{}-{}-{}",
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

    fn write(&self, name: &str, contents: &str) -> PathBuf {
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

/// The recorded [CP] improvement (rows 25/65): renaming a note
/// rewrites the inbound [[links]] in every other note, and the links
/// stay durable on disk. Upstream's hook was an empty stub, so old
/// links only ever worked as searches for the old title.
#[test]
fn rename_rewrites_inbound_links_on_disk_and_in_the_store() {
    let folder = TempFolder::new("rename-links");
    folder.write("target.md", "Target Note\nthe note everything points at");
    folder.write(
        "Linking Note.md",
        "Linking Note\nsee [[Target Note]] and also [[Target Note]] twice",
    );
    folder.write("Bystander.md", "Bystander\n[[Unrelated]] stays put");
    let outcome = load_folder(folder.path()).unwrap();
    let mut store = outcome.store;

    let target = store
        .find_by_path(&folder.path().join("target.md"))
        .unwrap()
        .path
        .clone();
    let renamed = store.rename_note(&target, "Renamed Subject").unwrap();
    assert_eq!(renamed.title.as_str(), "Renamed Subject");

    // The linker's FILE was rewritten: both links now name the new
    // title, the bystander is untouched.
    let linker_text = fs::read_to_string(folder.path().join("Linking Note.md")).unwrap();
    assert_eq!(
        linker_text,
        "Linking Note\nsee [[Renamed Subject]] and also [[Renamed Subject]] twice"
    );
    let bystander_text = fs::read_to_string(folder.path().join("Bystander.md")).unwrap();
    assert!(bystander_text.contains("[[Unrelated]]"));

    // The store sees the rewritten bodies too.
    let linker = store
        .find_by_path(&folder.path().join("Linking Note.md"))
        .unwrap();
    assert!(linker.body.as_str().contains("[[Renamed Subject]]"));

    // And a fresh load agrees (the rewrite is durable, not just
    // in-memory).
    let reloaded = load_folder(folder.path()).unwrap();
    let reloaded_linker = &reloaded.store.notes()[1];
    assert!(
        reloaded_linker
            .body
            .as_str()
            .contains("[[Renamed Subject]]")
    );
}

/// Backlink correctness under rename: the derived index follows the
/// title before and after.
#[test]
fn backlinks_follow_the_rename() {
    let folder = TempFolder::new("backlinks");
    folder.write("target.md", "Hub Note\nthe hub");
    folder.write("a.md", "A\nlinks to [[Hub Note]]");
    folder.write("b.md", "B\n[[Somewhere Else]]");
    let outcome = load_folder(folder.path()).unwrap();
    let mut store = outcome.store;

    let index = store.link_index();
    assert_eq!(index.backlinks_to("Hub Note"), [0]);

    let target = store
        .find_by_path(&folder.path().join("target.md"))
        .unwrap()
        .path
        .clone();
    store.rename_note(&target, "Renamed Hub").unwrap();
    // The rename rewrote the inbound link, so the index rebuilt over
    // the new bodies maps the NEW title.
    let index = store.link_index();
    assert_eq!(index.backlinks_to("Renamed Hub"), [1]);
    assert_eq!(index.backlinks_to("Hub Note"), &[] as &[usize]);
}

/// Row 61: conflict files are ordinary notes, and a create colliding
/// with one takes the row-4 suffix rather than disturbing it.
#[test]
fn syncthing_conflict_files_are_ordinary_notes() {
    let folder = TempFolder::new("conflict");
    folder.write(
        "Agenda.sync-conflict-20261117-123456.md",
        "Agenda\nthe conflicted twin",
    );
    let outcome = load_folder(folder.path()).unwrap();
    assert_eq!(outcome.store.len(), 1);
    assert_eq!(outcome.store.notes()[0].title.as_str(), "Agenda");
    assert_eq!(outcome.warnings, vec![]);

    // A create whose stem differs from the conflict file's stem
    // coexists with it (the row-4 collision compares stems, and
    // "Agenda" != "Agenda.sync-conflict-..."); the suffix applies
    // only on a true stem collision, covered by the create tests.
    let mut store = outcome.store;
    let created = store.create_note("Agenda", "the fresh one").unwrap();
    assert_eq!(
        created.path.file_name().unwrap().to_str().unwrap(),
        "Agenda.md"
    );
    assert_eq!(store.len(), 2);
    // The conflict note is untouched, still loaded, still itself.
    let conflict_name = store
        .notes()
        .iter()
        .find(|n| n.path.to_string_lossy().contains("sync-conflict"))
        .map(|n| n.title.as_str().to_owned())
        .unwrap_or_default();
    assert_eq!(conflict_name, "Agenda");
}

/// Row 53 at load: the Title: override wins the display title and the
/// folded search key; the raw first line stays the filename source.
/// Saving new metadata updates the override.
#[test]
fn title_metadata_overrides_and_survives_saves() {
    let folder = TempFolder::new("title-override");
    folder.write("real-file.md", "Title: Real Display Name\n\nbody text");
    let outcome = load_folder(folder.path()).unwrap();
    let note = &outcome.store.notes()[0];
    assert_eq!(note.display_title(), "Real Display Name");
    // The fallback title (the raw first line) is metadata-blind at the
    // derivation level and would have shown "Title: Real Display Name"
    // under nvALT parity (formats section 1.7).
    assert_eq!(note.title.as_str(), "Title: Real Display Name");
    assert_eq!(note.title_override.as_deref(), Some("Real Display Name"));
    // Search keys fold the display title.
    assert_eq!(note.folded_title.as_ref(), "real display name");

    let mut store = outcome.store;
    let path = store.notes()[0].path.clone();
    let saved = store
        .save_note(&path, "Title: Better Name\n\nbody text")
        .unwrap();
    assert_eq!(saved.display_title(), "Better Name");

    // A save WITHOUT metadata drops the override: the first line
    // rules again.
    let saved = store.save_note(&path, "Plain First Line\nbody").unwrap();
    assert_eq!(saved.title_override, None);
    assert_eq!(saved.display_title(), "Plain First Line");
}

/// Row 65: a metadata-titled note renames through its Title: key;
/// every other key and the fences survive.
#[test]
fn renaming_a_metadata_note_updates_its_title_key() {
    let folder = TempFolder::new("rename-meta");
    folder.write(
        "old-name.md",
        "---\nTitle: Old Display\nAuthor: Someone\n---\n\nthe body",
    );
    let outcome = load_folder(folder.path()).unwrap();
    let mut store = outcome.store;
    let note = &store.notes()[0];
    assert_eq!(note.display_title(), "Old Display");
    let path = note.path.clone();

    let renamed = store.rename_note(&path, "New Display").unwrap();
    assert_eq!(renamed.display_title(), "New Display");
    // Upstream's sanitizer only replaces ':' and a leading '.';
    // spaces stay in filenames.
    assert_eq!(
        renamed.path.file_name().unwrap().to_str().unwrap(),
        "New Display.md"
    );
    let on_disk = fs::read_to_string(&renamed.path).unwrap();
    // The fences, the other key, and the body all survive; only the
    // Title line's value changed.
    assert_eq!(
        on_disk,
        "---\nTitle: New Display\nAuthor: Someone\n---\n\nthe body"
    );

    // And a block WITHOUT a Title key gains one at its top.
    folder.write("tagged.md", "tags: inbox\n\nstuff");
    let outcome = load_folder(folder.path()).unwrap();
    let path = folder.path().join("tagged.md");
    let mut store = outcome.store;
    let renamed = store.rename_note(&path, "Tagged Now").unwrap();
    assert_eq!(renamed.display_title(), "Tagged Now");
    let on_disk = fs::read_to_string(&renamed.path).unwrap();
    assert_eq!(on_disk, "Title: Tagged Now\ntags: inbox\n\nstuff");
}

/// Row 65: transclusion cycles are detected, acyclic graphs are not.
#[test]
fn transclusion_cycles_are_detected() {
    let folder = TempFolder::new("transclude");
    folder.write("a.md", "Note A\n{{Note B}}");
    folder.write("b.md", "Note B\n{{Note A}}");
    folder.write("c.md", "Note C\nstraight");
    folder.write("d.md", "Note D\n{{Note C}}");
    let outcome = load_folder(folder.path()).unwrap();
    let store = outcome.store;

    let a = folder.path().join("a.md");
    let d = folder.path().join("d.md");

    // A -> B -> A: a cycle.
    let cycle = store.transclusion_cycle_from(&a).unwrap();
    assert_eq!(cycle.len(), 3); // a, b, and the repeated a
    assert_eq!(cycle[0], a);

    // D -> C: acyclic.
    assert_eq!(store.transclusion_cycle_from(&d), None);
}

/// Row 56 stays true through the write paths: warnings flow out of
/// reconcile too (a Latin-1 file appearing between reconciles).
#[test]
fn reconcile_surfaces_new_arrivals_warnings() {
    let folder = TempFolder::new("reconcile-warn-golden");
    let outcome = load_folder(folder.path()).unwrap();
    let mut store = outcome.store;
    fs::write(folder.path().join("legacy.md"), b"caf\xE9 latin-1").unwrap();
    let report = store.reconcile(folder.path()).unwrap();
    assert_eq!(
        report.warnings,
        vec![LoadWarning::InvalidUtf8 {
            path: folder.path().join("legacy.md"),
        }]
    );
    assert_eq!(report.added, vec![folder.path().join("legacy.md")]);
}
