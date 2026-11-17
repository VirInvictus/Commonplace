# Rust model notes: core types and the index-units contract

Design notes, 2026-11-17. Pre-development planning for Phases 1-4;
these are NOTES, not the contract: `spec.md` owns behavior, and the
code supersedes these sketches once it exists. Feeds the Phase 1-3
opening work so type shapes are decided before the first test file.

## The index-units contract (the subtle one)

Upstream counts everything in **UTF-16 code units** (NSString): the
300KB cap is `300 * 1024` UTF-16 units (map §1.4), preview slices are
±64 * 1024 UTF-16 units, title caps are 60 chars, editor offsets are
UTF-16 positions. Rust strings are UTF-8 **bytes**; GTK text
iterators are **chars**; comrak `sourcepos` is **bytes**. A port that
mixes these silently corrupts cursor math, slicing windows, and cap
comparisons.

**Pinned rule (recommendation, confirm at Phase 1):**

- All in-engine offsets and windows count **`char`s** (Unicode scalar
  values), never bytes: `char` ≈ UTF-16 unit except astral planes
  (emoji), where UTF-16 counts 2 and we count 1: a documented,
  one-sided divergence that only widens a cap slightly for emoji-
  heavy notes.
- All caps compare against **chars**: `NV_CAP: usize = 300 * 1024`
  chars for the highlight/skip cap, `PREVIEW_SLICE: usize = 64 * 1024`
  chars, title cap 60 chars. Byte-based APIs (comrak sourcepos) are
  converted at the boundary and never leak inward.
- File sizes on disk remain bytes (mtime/size reconciliation keys).
- The search matcher is **lowercased substring over `char`s** via
  `str::match_indices` on the lowercased cache (upstream: byte strstr
  over UTF-8 after lowercasing: equivalent for the case-folded prefix;
  no diacritic folding in either).

## Core type sketches

```rust
// commonplace-core::note
pub struct Note {
    pub path: PathBuf,        // file in the collection folder
    pub title: String,        // derived first-line title (Phase 1 rules)
    pub title_override: Option<String>, // Phase 5 `Title:` metadata (spec row 53)
    pub body: String,         // full text, no trailing-newline games
    pub mtime: SystemTime,    // reconciliation key (with size)
}
// Display title = title_override.unwrap_or(title). Filename is always
// derived from the raw first-line title (the file IS the note).

// commonplace-core::store
pub struct Store {
    notes: Vec<Note>,                // the arena; ids are indexes
    by_path: HashMap<PathBuf, u32>,  // load/rename/dispatch
    gen: u32,                        // bumped on structural mutation
}
// NoteId(u32) is generation-checked; filtering compacts a Vec<NoteId>,
// never the notes themselves. Single-threaded, no locks: the app owns
// the Store on the main loop and the CLI builds one per invocation.
// Folder rescans (Phase 4) produce a diff the store applies in place.

// commonplace-core::query  (Phase 2 grammar)
pub enum Expr {
    Term(String),              // lowercased needle
    Phrase(String),            // quoted, exact ordered
    TagWord { name: String, prefix: bool }, // #tag (nvALT) / tag: full (ULTRA flag)
    And(Vec<Expr>), Or(Vec<Expr>), Not(Box<Expr>),
}
// precedence: parens > NOT > AND > OR; implicit AND at parse level.

// commonplace-core::search  (Phase 3)
pub struct FilterState {
    query: String,             // the raw field text
    candidate: Vec<NoteId>,    // the currently filtered set
    resume: Resume,            // per-token char-offset caches (the FoundPtr trick)
}
// Upstream resumes strstr mid-note per token (map §2 useCachedPositions);
// the Rust shape: for the newest token only, keep (note_id, char_offset)
// pairs and continue match_indices from there. Any structural change
// (edit, rename, disk event) invalidates resume entirely: refilter from
// allNotes. Prefix-extension of the query keeps the candidate set.

// commonplace-core::cache  (lazily built, always rebuildable)
pub struct SearchCaches {
    lower_title: Box<str>,
    lower_body: Option<Box<str>>,   // built on first body search
    lower_labels: Option<Box<str>>, // Phase 5
}
// Upstream strdup-lowercases on edit (map §2); ours builds lazily on
// first search touching that field and drops on edit. Memory: ~2x note
// text while a search is live; acceptable at 10k notes (Phase 3 bench
// owns the proof).
```

### Phase 1 outcome (2026-11-17): what the code decided

The sketches above are superseded where Phase 1's shipped code differs
(`crates/commonplace-core/src/{note,store}.rs`):

- **Landed**: `Note { path, title: NoteTitle, body: NoteBody, mtime }`.
  Title and body are newtypes, per the crosswalk's intrinsic-behavior
  rule (`research/objc-rust-crosswalk.md` §3): `NoteTitle::from_body`
  carries the first-line rule, `NoteBody::from_utf8` the UTF-8/BOM
  decode. `title_override` waits for Phase 5.
- **Landed**: `store::load_folder(folder) -> Result<LoadOutcome,
  StoreError>` where `LoadOutcome { store, warnings }` adds a
  `Vec<LoadWarning>` the sketch did not have (spec §5 rows 54-56).
- **Deferred, not dropped**: `NoteId` and the `gen` counter. Nothing
  mutates the arena in the read-only phase, so a generation check has
  nothing to guard; both arrive with the write phase's structural
  mutation. The Vec arena + path index landed as sketched, kept
  path-sorted (the Phase 1 deterministic view; column sorts are
  Phase 3, where upstream's date-modified-descending default also
  lands).
- **Untouched**: the `Expr` / `FilterState` / `SearchCaches` sketches
  remain the Phase 2-3 starting points.

## Deliberately dropped from upstream's NoteObject

The upstream note carries ~40 ivars (map §1.1): attributed-body
variants, per-format writers, encoding xattrs, Finder-tag mirrors,
sync keys, WAL integration, tombstones-in-model. Commonplace drops:
attributed variants (layer 3 of the translation map), xattr encoding
bookkeeping (Phase 1 divergence), sync keys (no sync: invariant 5),
the in-model tombstone (Phase 4: trash-on-disk + undo history in the
app layer instead). Everything dropped is recorded in the spec's
parity table rows or §4 non-goals, never silently.

## Error and result shape

Engine is dependency-free including error plumbing: a hand-rolled
`Error` enum per module with `Display` (no thiserror). Public APIs
return `Result<T, Error>`; the CLI maps errors to the exit taxonomy
(spec §6); the app maps them to toasts. No panics across the public
API: parse failures and I/O failures are values.

## Testing posture (per phase, from the contract tables)

Golden tests come from the research corpus, not from implementation:
Phase 1 fixture folders encode map §4's rules; Phase 2's golden
queries are the nvUltra manual examples plus the NV tokenization
edge cases (quotes, colons); Phase 3's property test is "incremental
== naive at every step"; Phase 11's behavior table is contract §1J
row by row. Each phase's roadmap checklist already carries the test
box: tests precede implementation.
