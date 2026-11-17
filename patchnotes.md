# Patchnotes

Newest first.

## v0.2.0 (2026-11-17)

- The engine arc, Phases 1-3 of the roadmap: `commonplace-core` now
  loads a notes folder the way nvALT reads one and powers the
  find-as-you-type search headlessly. GUI and CLI surfaces are still
  ahead (Phases 4+).
- Note store read side (Phase 1): `store::load_folder` scans a flat
  folder with nvALT's plain-text admission table (`md txt text utf8
  taskpaper`, case-insensitive; dot-files and the upstream metadata
  filenames skipped), decodes UTF-8 with the BOM stripped, and never
  lossily replaces undecodable files: they load as "Untitled Note"
  with a per-file `LoadWarning`. Display titles derive from the
  content's first line under the upstream rule (60-char cap with
  break-at-space, "Untitled Note"), a recorded divergence from
  upstream's filename-derived load titles.
- Query language (Phase 2): one parser over the NV, nvALT, and
  nvUltra vocabularies: bare words with implicit AND, `:` as a token
  separator, quoted exact phrases, `#tag` words, nvUltra booleans
  (AND/OR/NOT with `&&`/`&`, `||`/`|`, `!` aliases and parentheses,
  uppercase keywords), the full-name `tag:` operator, and case plus
  diacritic folding (`bucher` finds `Bücher`). Grammar failures are
  error values; the empty query filters nothing.
- Search machinery (Phase 3): the incremental keystroke filter with
  the prefix-resume trick, stable two-pass column sorts (title, date
  modified, date created; reverse flips the tiebreaks; defaults
  match upstream's date-modified-reversed), and the autocomplete
  selection rule with prefix parents. Parser and scanner share one
  tokenizer, so `tag:`, quote-segment, and non-breaking-space queries
  filter identically fresh or incrementally. Property tests pin
  incremental == naive at every step; the 10k-note benchmark gates
  keystrokes at 50 ms on release builds (measured: 13-16 ms cold,
  65-90 µs warm).
- The engine stays dependency-free: zero crates in
  `commonplace-core`, hand-rolled folding (a 488-entry Unicode
  canonical-decomposition table), hand-rolled error types, headless
  tests only.

## v0.1.0 (2026-11-17)

- Initial skeleton: three-crate workspace (the `commonplace-core`
  engine, the `commonplace` GTK4 shell, `commonplace-cli`), CI shaped
  from Docket's hardened workflow, the committed research notes under
  `research/`, and the port roadmap in `roadmap.md`. Two local-only,
  gitignored pieces live on the machine but not in the tree: the
  read-only upstream clones under `.repos/` and the page snapshots
  under `research/raw/`. No features yet; this is Phase 0 of the
  roadmap.
