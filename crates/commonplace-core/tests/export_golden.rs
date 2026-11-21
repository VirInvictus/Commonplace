//! Golden tests for the export core (spec rows 69-70): text round-trip
//! byte stability, HTML well-formedness, row-4 export filenames with
//! collisions, and the expand-then-render pipeline for HTML.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use commonplace_core::export::{ExportFormat, export_note_to_dir, export_store_to_dir};
use commonplace_core::render::StubRenderer;
use commonplace_core::store::{LoadOutcome, load_folder};

/// A scratch folder under the OS temp dir, removed on drop.
struct TempFolder {
    path: PathBuf,
}

impl TempFolder {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "commonplace-export-golden-{}-{}-{}-{}",
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
        let file = self.path.join(name);
        fs::write(&file, contents).unwrap();
        file
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn load(path: &Path) -> LoadOutcome {
    load_folder(path).unwrap()
}

/// Dependency-free well-formedness check over the emitted documents
/// (row 70): every opened tag closes in order, ignoring void elements
/// and self-closed tags.
fn well_formed(html: &str) -> bool {
    const VOID: [&str; 9] = [
        "br", "hr", "img", "meta", "link", "input", "col", "area", "source",
    ];
    let mut stack: Vec<String> = Vec::new();
    let bytes = html.as_bytes();
    let mut scan = 0usize;
    while let Some(lt_rel) = bytes[scan..].iter().position(|&b| b == b'<') {
        let lt = scan + lt_rel;
        let Some(gt_rel) = bytes[lt..].iter().position(|&b| b == b'>') else {
            return false;
        };
        let gt = lt + gt_rel;
        let tag_body = &html[lt + 1..gt];
        scan = gt + 1;
        if tag_body.starts_with('!') || tag_body.ends_with('/') {
            continue;
        }
        if let Some(name) = tag_body.strip_prefix('/') {
            let Some(open) = stack.pop() else {
                return false;
            };
            if !open.eq_ignore_ascii_case(name.trim()) {
                return false;
            }
        } else {
            let name: String = tag_body
                .split(|c: char| c.is_ascii_whitespace() || c == '>')
                .next()
                .unwrap_or("")
                .to_owned();
            if name.is_empty() || VOID.contains(&name.to_ascii_lowercase().as_str()) {
                continue;
            }
            stack.push(name);
        }
    }
    stack.is_empty()
}

#[test]
fn text_export_is_the_body_verbatim() {
    let notes = TempFolder::new("text-verbatim");
    notes.write("Alpha.md", "Alpha\n\nbody text & <more>\n");
    let outcome = load(notes.path());
    let store = outcome.store;

    let out = TempFolder::new("text-out");
    let written = export_note_to_dir(
        &store,
        &store.notes()[0].clone(),
        out.path(),
        ExportFormat::PlainText,
        &StubRenderer,
    )
    .unwrap();
    assert_eq!(written.file_name().unwrap(), "Alpha.md");
    assert_eq!(
        fs::read_to_string(&written).unwrap(),
        "Alpha\n\nbody text & <more>\n",
        "the body bytes, nothing added"
    );
}

#[test]
fn html_export_wraps_rendered_content_with_escaped_title() {
    let notes = TempFolder::new("html-wrap");
    notes.write("Weird <Title>.md", "Weird <Title>\n\nplain para\n");
    let outcome = load(notes.path());
    let store = outcome.store;

    let out = TempFolder::new("html-out");
    let written = export_note_to_dir(
        &store,
        &store.notes()[0].clone(),
        out.path(),
        ExportFormat::Html,
        &StubRenderer,
    )
    .unwrap();
    let html = fs::read_to_string(&written).unwrap();
    assert!(
        html.contains("<title>Weird &lt;Title&gt;</title>"),
        "{html}"
    );
    assert!(html.contains("<p>plain para</p>"), "{html}");
    assert!(well_formed(&html), "document is well-formed: {html}");
}

#[test]
fn html_export_expands_transclusions_before_rendering() {
    let notes = TempFolder::new("html-transclude");
    notes.write("A.md", "A\n\nfrom A {{B}}\n");
    notes.write("B.md", "B\n\nfrom B\n");
    let outcome = load(notes.path());
    let store = outcome.store;

    let out = TempFolder::new("html-transclude-out");
    let note = store
        .notes()
        .iter()
        .find(|n| n.display_title() == "A")
        .unwrap()
        .clone();
    let written =
        export_note_to_dir(&store, &note, out.path(), ExportFormat::Html, &StubRenderer).unwrap();
    let html = fs::read_to_string(&written).unwrap();
    // The target's whole body is inlined verbatim, title line first.
    assert!(
        html.contains("<p>from A B</p>\n<p>from B</p>"),
        "expanded: {html}"
    );
}

#[test]
fn text_export_does_not_expand_transclusions() {
    // Text is the body verbatim (row 69/70): expansion is a render
    // pipeline step, and expanding would break the byte-stable
    // round-trip.
    let notes = TempFolder::new("text-no-expand");
    notes.write("A.md", "A\n\nkeep {{B}} literal\n");
    let outcome = load(notes.path());
    let store = outcome.store;

    let out = TempFolder::new("text-no-expand-out");
    let note = store.notes()[0].clone();
    let written = export_note_to_dir(
        &store,
        &note,
        out.path(),
        ExportFormat::PlainText,
        &StubRenderer,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(&written).unwrap(),
        "A\n\nkeep {{B}} literal\n"
    );
}

#[test]
fn multi_note_export_uses_row4_filenames_and_collisions() {
    let notes = TempFolder::new("multi");
    notes.write("Alpha.md", "Alpha\n\none\n");
    notes.write("ALPHA.md", "ALPHA\n\ntwo\n");
    notes.write("a:b.md", "a:b\n\ncolon\n");
    // Subfolders are Phase 16's flat display; the slash-fold case comes
    // from a content-derived title instead.
    notes.write("slashy.md", "sub/dir title\n\nslash\n");
    let outcome = load(notes.path());
    let store = outcome.store;
    assert_eq!(
        store.len(),
        4,
        "{:?}",
        store
            .notes()
            .iter()
            .map(|n| n.display_title())
            .collect::<Vec<_>>()
    );

    let out = TempFolder::new("multi-out");
    let result =
        export_store_to_dir(&store, out.path(), ExportFormat::PlainText, &StubRenderer).unwrap();
    assert_eq!(result.written.len(), 4, "{:?}", result.written);
    assert!(result.skipped.is_empty(), "{:?}", result.skipped);

    let names: Vec<String> = result
        .written
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    // Path order (byte sort) gives the bare stem to ALPHA; Alpha
    // collision-suffixes. Both keep their own title case.
    assert!(names.contains(&String::from("ALPHA.md")), "{names:?}");
    assert!(names.contains(&String::from("Alpha.1.md")), "{names:?}");
    assert!(names.contains(&String::from("a-b.md")), "{names:?}");
    assert!(
        names.contains(&String::from("sub-dir title.md")),
        "{names:?}"
    );
}

#[test]
fn export_reimport_fixed_point_is_byte_stable() {
    let notes = TempFolder::new("fixed-point");
    notes.write("One.md", "One\n\nfirst & only\n");
    notes.write("Two.md", "Two\n\nsecond\n\nwith blank lines\n");
    let first = load(notes.path());

    let round1 = TempFolder::new("round1");
    let result = export_store_to_dir(
        &first.store,
        round1.path(),
        ExportFormat::PlainText,
        &StubRenderer,
    )
    .unwrap();
    assert!(result.skipped.is_empty());

    let second = load(round1.path());
    let round2 = TempFolder::new("round2");
    export_store_to_dir(
        &second.store,
        round2.path(),
        ExportFormat::PlainText,
        &StubRenderer,
    )
    .unwrap();

    // Same file set, byte-identical contents.
    let names_of = |dir: &Path| -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_file())
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    fs::read_to_string(&p).unwrap(),
                )
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(names_of(round1.path()), names_of(round2.path()));

    // And the re-import derived the same display titles.
    let titles: Vec<&str> = second
        .store
        .notes()
        .iter()
        .map(|n| n.display_title())
        .collect();
    assert!(
        titles.contains(&"One") && titles.contains(&"Two"),
        "{titles:?}"
    );
}

#[test]
fn empty_store_export_creates_the_dir_and_writes_nothing() {
    let notes = TempFolder::new("empty");
    let outcome = load(notes.path());
    let out = TempFolder::new("empty-out").path().join("created");
    let result =
        export_store_to_dir(&outcome.store, &out, ExportFormat::PlainText, &StubRenderer).unwrap();
    assert!(result.written.is_empty());
    assert!(out.is_dir());
}

#[test]
fn export_collision_never_overwrites_a_preexisting_file() {
    let notes = TempFolder::new("preexisting");
    notes.write("Alpha.md", "Alpha\n\nfresh\n");
    let outcome = load(notes.path());
    let store = outcome.store;

    let out = TempFolder::new("preexisting-out");
    fs::write(out.path().join("Alpha.md"), "sentinel").unwrap();
    let written = export_note_to_dir(
        &store,
        &store.notes()[0].clone(),
        out.path(),
        ExportFormat::PlainText,
        &StubRenderer,
    )
    .unwrap();
    assert_eq!(
        written.file_name().unwrap(),
        "Alpha.1.md",
        "collided, not overwritten"
    );
    assert_eq!(
        fs::read_to_string(out.path().join("Alpha.md")).unwrap(),
        "sentinel"
    );
}
