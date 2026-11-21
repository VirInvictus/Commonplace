//! The export core (spec rows 69-70): file-out parity for the formats
//! Commonplace keeps. Text export is the body verbatim (byte-stable
//! through re-import); HTML export runs the render boundary after
//! transclusion expansion, wrapped in a minimal standalone document.
//! Filenames derive under the row-4 rules against the export
//! directory, colliding by `.N` suffix, never overwriting.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::fold::fold;
use crate::note::Note;
use crate::render::{Renderer, expand_transclusions_rooted};
use crate::store::Store;
use crate::store::derive_filename;

/// The kept export formats; the cut list (RTF/DOC/DOCX, Textile,
/// Stickies, .blor) lives in spec §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// The body verbatim, extension `md` (upstream `pathExtensionForFormat`,
    /// NotationPrefs.m:918-929).
    PlainText,
    /// The render boundary's HTML, extension `html`.
    Html,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::PlainText => "md",
            ExportFormat::Html => "html",
        }
    }
}

/// Directory creation fails the whole export; per-file write
/// failures become `skipped` entries in the batch verb so one bad
/// file cannot sink it, and are `Write` errors on the single-note
/// verb.
#[derive(Debug)]
pub enum ExportError {
    OutDirCreate(std::io::Error, PathBuf),
    Write(String),
}

#[derive(Debug, Default)]
pub struct ExportOutcome {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<SkippedExport>,
}

#[derive(Debug)]
pub struct SkippedExport {
    pub title: String,
    pub reason: String,
}

/// Export every note in the store to `out_dir` (created if missing),
/// in the store's path-sorted order.
pub fn export_store_to_dir(
    store: &Store,
    out_dir: &Path,
    format: ExportFormat,
    renderer: &dyn Renderer,
) -> Result<ExportOutcome, ExportError> {
    fs::create_dir_all(out_dir).map_err(|e| ExportError::OutDirCreate(e, out_dir.to_path_buf()))?;
    let mut outcome = ExportOutcome::default();
    let mut assigned: HashSet<String> = HashSet::new();
    for note in store.notes() {
        match write_note_file(store, note, out_dir, format, renderer, &mut assigned) {
            Ok(path) => outcome.written.push(path),
            Err(reason) => outcome.skipped.push(SkippedExport {
                title: note.display_title().to_owned(),
                reason,
            }),
        }
    }
    Ok(outcome)
}

/// Export one note; the same pipeline `export_store_to_dir` runs per
/// note. The filename never collides into an existing file: the
/// row-4 `.N` suffix resolves it.
pub fn export_note_to_dir(
    store: &Store,
    note: &Note,
    out_dir: &Path,
    format: ExportFormat,
    renderer: &dyn Renderer,
) -> Result<PathBuf, ExportError> {
    fs::create_dir_all(out_dir).map_err(|e| ExportError::OutDirCreate(e, out_dir.to_path_buf()))?;
    let mut assigned: HashSet<String> = HashSet::new();
    write_note_file(store, note, out_dir, format, renderer, &mut assigned)
        .map_err(ExportError::Write)
}

fn write_note_file(
    store: &Store,
    note: &Note,
    out_dir: &Path,
    format: ExportFormat,
    renderer: &dyn Renderer,
    assigned: &mut HashSet<String>,
) -> Result<PathBuf, String> {
    let extension = format.extension();
    let taken = |candidate: &str| {
        assigned.contains(&candidate.to_lowercase())
            || out_dir.join(format!("{candidate}.{extension}")).exists()
    };
    let filename = derive_filename(note.display_title(), extension, taken);
    assigned.insert(filename[..filename.len() - extension.len() - 1].to_lowercase());

    let content = match format {
        // Text is the body verbatim (row 70's byte stability); no
        // expansion, no rendering.
        ExportFormat::PlainText => note.body.as_str().to_owned(),
        // HTML runs the [CP] pipeline: expand transclusions
        // pre-render (row 67, rooted at this note), then render
        // (row 66), then wrap.
        ExportFormat::Html => {
            let expanded = expand_transclusions_rooted(
                note.body.as_str(),
                Some(note.display_title()),
                &|interior| resolve_transclusion(store, interior),
            );
            export_document_html(note.display_title(), &renderer.render_markdown(&expanded))
        }
    };

    let path = out_dir.join(&filename);
    fs::write(&path, content.as_bytes()).map_err(|e| format!("write failed: {e}"))?;
    Ok(path)
}

/// The engine-side transclusion resolver (row 67's closure shape):
/// folded exact-title equality over display titles.
fn resolve_transclusion(store: &Store, interior: &str) -> Option<String> {
    let key = fold(interior);
    store
        .notes()
        .iter()
        .find(|note| fold(note.display_title()) == key)
        .map(|note| note.body.as_str().to_owned())
}

/// The minimal standalone HTML document (row 69): title escaped into
/// `<title>`, rendered body inline. The preview-page templates and
/// their `{%...%}` slots are the Phase 15 surface; export stays a
/// plain document.
pub fn export_document_html(title: &str, rendered_body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        crate::render::escape_html(title),
        rendered_body
    )
}
