# Patchnotes

Newest first.

## v0.6.1 (2026-11-20)

**vir-gtk 1.5.0 adoption: the app-tier quiet-toggle shim is gone; the
upstream default now does what it was fighting for.**

*   **Dropped the `.cp-quiet-toggle` shim.** The sort-direction
    "Reverse" toggle carried an app-tier stylesheet (`style.rs`) whose
    only content was quieting the checked/active paint that vir-gtk's
    base sheet put on every checked button, because the dragon red
    reads pink at button scale and a binary-state toggle is not a
    suggested action. That ask was filed upstream as vir-gtk issue #1,
    and vir-gtk 1.5.0 amended the default: the base sheet's
    `button:active`/`:checked` rule now lifts to the raised tone and
    spends no accent. The shim module, its install call, and the CSS
    class on the toggle are deleted. One deliberate one-step surface
    shift: the toggle's checked background moves from the card tone
    (what the shim painted) to the raised tone (what the base now
    paints); text and border are identical.
*   **vir-gtk lock moved 1.4.3 → 1.5.0** (`d37ef6e`).

## v0.6.0 (2026-11-20)

- The app rises, Phases 10-11 of the roadmap: the first GTK4 window,
  the live list, and the search-field state machine that is the whole
  reason the port exists. The engine and CLI surfaces are unchanged;
  this release is the shell over them.
- The two-pane shell (Phase 10): the NV layout with the search field
  above the note list and the editor pane right; a sort picker
  (Date Modified / Date Created / Title, with the upstream defaults
  date-modified-reversed); an empty state; the degraded window that
  names a missing notes folder instead of crashing. The list runs the
  house shape: `GtkListView` + `SingleSelection` (autoselect off,
  can-unselect on) over a `gio::ListStore` of note GObjects, rows
  through a `SignalListItemFactory` with bind-time recycling, and the
  body preview hard-capped at 400 characters before Pango sees it.
- Styling is vir-gtk (the house styling layer, the libadwaita
  replacement) with the Kanagawa dragon/lotus palettes following the
  system dark/light setting through the portal, re-splicing on every
  flip. vir-gtk joins as a git dependency per its own documented
  consumption model; the app-tier stylesheet carries the quiet-toggle
  override while the crate's accent-as-checked-default stands (filed
  upstream as vir-gtk#1).
- File watching (Phase 10): notify 8.2 + notify-debouncer-full 0.7 in
  the app crate only; every coalesced event batch is one
  reconcile-then-repopulate pass, with the Phase 4 reconcile (path +
  re-stat keying) remaining the truth of what changed. Watch behavior
  tests headless: external temp-plus-rename writes and renames signal
  reconcile.
- The search-field state machine (Phase 11, the defining UX ported
  1:1): typing filters incrementally; a title prefix completes into
  the field with the remainder selected; Return edits the selected
  note or creates one titled with the field text; Escape clears
  field, selection, and refocuses; Up/Down walk the list with the
  field's text kept selected; Ctrl-Up/Down jump to the ends; Tab eats
  when empty, re-completes on an unmatched query with candidates, and
  focuses the body when matched; Ctrl-U clears the line; the field
  keeps its typed text when the selection changes; and typing while
  the list is focused lands in the field. Launch restores the last
  search and last note from the config (`last_query`, `last_note`).
  The snapback machinery follows DualField's actual split: a
  typed-search slot beside a followed-links history that only route
  navigation pushes and only user typing wipes.
- The state machine is a headless model that answers events with
  actions; the GTK shell applies them. The whole behavior table tests
  without GTK (18 unit tests), and the live flow (type, complete,
  Return, relaunch-restore) was verified end to end on the real
  desktop with captures.

## v0.5.0 (2026-11-20)

- The CLI arc, Phases 8-9 of the roadmap: the first runnable
  artifact. `commonplace-cli` now drives the whole engine from the
  shell, and the M1 documentation catch-up milestone fired at the end
  of the arc (the five surfaces re-read against the shipped verbs,
  every README example smoke-run through the real binary).
- Twelve verbs: `search`, `list`, `open`, `new`, `append`, `export`,
  `tags`, `cat`, `render`, `config`, plus the two [CP] additions
  pinned at this phase, `completions` (bash/zsh/fish scripts over the
  pinned surface) and `scheme` (the find/make/markdownify route entry
  points sharing Phase 18's routes; find and make are functional, the
  markdownify/html/tags arms report that the converter lands with the
  import phase). Arg parsing is hand-rolled stdlib (the standing
  decision); the JSON envelope writer is hand-rolled too, so the
  workspace's only external dependency is still comrak.
- Exit taxonomy (spec §6, pinned): 0 success including empty reports;
  1 trouble (runtime failures and failed resolutions where a target
  was required); 2 usage. `--json` prints exactly one envelope per
  verb, shapes pinned by snapshot tests and additive from here.
- Config (spec §6): `~/.config/commonplace/config.toml` parsed and
  written by a hand-rolled TOML subset in the engine (quoted-string
  key/values, one section, three escapes, no serde; foreign keys
  survive every write). `notes_dir` defaults to `~/Documents/Notes`
  [CP]; `saved_searches` carries the nvUltra hotkey slots `0`-`9`,
  shared with the app, invocable as `search --saved N`. Unset slots
  are usage errors; the `config` verb never opens the notes folder,
  so `--set notes_dir` works on a machine that has none yet.
- Resolution semantics: `open`/`cat`/`render`/`append`/`scheme find`
  resolve a query to one note through the Phase 3 selection rule
  (first title-prefix match, prefix parent wins); `--sort` names an
  ascending column and `--reverse` flips it, while the unnamed
  default stays upstream's date-modified-reversed. M1's smoke pass
  caught two real bugs before they shipped: the selection position
  was being read as an arena index (every resolution verb silently
  targeted the first note in path order), and the `config` verb
  opened the notes folder it was supposed to configure.

## v0.4.0 (2026-11-20)

- The render boundary and export core, Phases 6-7 of the roadmap:
  with this release the engine is feature-complete (it loads,
  searches, sorts, filters, writes, tags, links, renders, and exports
  a notes folder, all headless), and the CLI and GUI arcs are next.
- The render boundary (Phase 6): Markdown-to-HTML is a
  `render::Renderer` trait in `commonplace-core`, dependency-free,
  with a plain-text stub renderer as the fixed test oracle. comrak
  1.0.0 lands as the one sanctioned frontend dependency (pinned
  `=1.0.0` in `commonplace-cli`, where the export pipeline consumes
  it; the engine itself still carries zero crates). Wikilinks are
  deliberately un-parsed by the renderer so `[[X]]` survives as
  literal text (upstream Apex parity); math dollars are enabled, and
  the adapter normalizes comrak's `data-math-style` markup to the
  upstream `class="math"` shape so the detection needles stay
  byte-exact.
- Transclusion expansion (Phase 6, the recorded [CP] pipeline):
  `{{title}}` expands depth-first before anything renders, matching
  by folded exact display title; a cycle rejects that branch with the
  offending literal left in place, an unresolvable target stays
  literal without stopping the walk, and depth caps at 16.
- The preview-prep pass (Phase 6): nvALT's
  `NVPreviewHTMLPreparingContent` ported as hand-rolled, regex-free
  scans over rendered HTML: mermaid fences rewrite into
  `<pre class="mermaid">` blocks (both the `language-mermaid` and
  `lang="mermaid"` shapes, case-insensitive), the math/mermaid/wiki
  detection flags compute over the prepared result, completed task
  items gain the `nv-task-done` class with the exact upstream task
  CSS prepended, and code-highlight detection excludes both mermaid
  blocks and inline code.
- Export core (Phase 7): dir-to-dir and single-note export in two
  formats. Plain text writes the body verbatim with a proven
  byte-stable round trip (export, re-import, export again:
  byte-identical). HTML runs expand-then-render into a minimal
  standalone document with the display title escaped into `<title>`.
  Export filenames derive under the row-4 rules against the export
  directory, so `:` and `/` fold to `-` and collisions take the
  case-insensitive `.N` suffix; a preexisting file is never
  overwritten.
- The cut list is recorded in spec §4 with rationale: RTF/DOC/DOCX
  export (Cocoa attributed-string machinery, no Linux analog),
  Textile, Stickies import, the `.blor` archive, MultiMarkdown as a
  render engine, and Finder tags plus upstream catalog dates on
  exported files.

## v0.3.0 (2026-11-17)

- The write path and the tag/link layer, Phases 4-5 of the roadmap:
  the engine now writes notes with nvALT's file rules and carries the
  tag and link graph, still dependency-free and headless.
- Notes become writable (Phase 4): create with upstream's filename
  derivation (colon and the Linux path separator fold to hyphen, a
  leading dot folds to underscore, byte-accurate 255 cap with the
  collision reserve, case-insensitive `.N` suffixes), atomic saves
  (dot-prefixed temp, fsync, `rename(2)` replace; a crash mid-write
  leaves nothing the scan can admit), rename with the
  keep-existing-extension rule and two atomic legs with best-effort
  content rollback, delete through an injected sink (the freedesktop
  `trash` crate wires in at the app phase), rescan-reconcile keyed on
  path plus `(mtime, size)`, and the two-handle write coalescer (a
  2.7 s edit-cancellable debounce beside a 15 s cadence the edit
  never cancels, upstream's asymmetry preserved).
- Tags and metadata (Phase 5): MultiMarkdown metadata blocks parse
  per the pinned v6 grammar (placement, blank-line termination, the
  scanner's key charset, continuation lines, YAML-compat fences, the
  `...` closer); a `Title:` key overrides the display title while the
  raw first line stays the filename source; tags come from the
  metadata `Tags:` value plus inline `#tags`, folded for matching
  (so `#cafe` finds `#café`), indexed derived-and-rebuildable.
- Links (Phase 5): `[[wiki links]]` extract under nvALT's exact
  scanner rules (guard set, the Unicode newline trap, the
  ObjC-bracket heuristic); the derived backlink index maps folded
  link text to notes; renaming a note rewrites the inbound links
  that match its old title, the recorded [CP] improvement over
  upstream's empty stub. `{{transclusion}}` is recognized with cycle
  detection; expansion lands with the preview phases.
- Search integration: tag-word queries consult the tag store
  (`#word` prefix-matches folded tag titles beside the literal
  `#xyz` rule; `tag:` full-names them); the property suite pins the
  incremental filter to the oracle through the new matching. The
  strict full-name toggle for `#word` arrives with preferences
  (Phase 19).
- Correctness fixes from the review passes, user-visible on messy
  input: metadata offsets are byte-accurate (a rename could panic on
  unicode-titled metadata), CRLF notes retitle with their own line
  endings preserved (they previously never retitled at all), the
  URL-grammar rules follow the pinned scanner (a bare-URL first line
  starts no block; a line merely containing a URL is ordinary
  metadata; URL-valued keys like CSS start pairs instead of being
  swallowed as continuations), adjacent inline tags all register,
  and transclusion cycles are found even behind unresolvable
  siblings.

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
