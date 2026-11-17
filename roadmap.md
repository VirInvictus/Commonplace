# Commonplace roadmap

The port plan: which portions of the Notational Velocity / nvALT /
nvUltra surface get ported, in what order, and the decisions each phase
must answer first. Phases are sequential by dependency, not by calendar;
within a phase, work is shippable to `main` incrementally.

**Checklists.** Every phase carries checkboxes. Tick a box in the
commit that ships the work, never in a batch after the fact, and never
leave an "Unreleased" holding pen: landed work lands released
(patchnotes entry, VERSION bump, tag, GitHub Release; see AGENTS.md,
"Documentation surfaces" and "Release flow"). Documentation updates
ride the same commits as the code they describe; the three
documentation catch-up milestones below exist to catch what slipped,
never to carry planned work.

**How to read a phase.** Every phase names the upstream code it ports
with `file:line` citations into the research corpus:

- `research/nv-source-map.md` (the NV/nvALT code map; "map §N")
- `research/nvalt-delta.md` (the nvALT feature delta; "delta §N")
- `research/nvultra-feature-survey.md` (nvUltra's public surface; "ultra §N")
- `research/linux-ecosystem-brief.md` (Linux prior art + 2026 baseline; "brief §N")

Tiers per `spec.md` §5: **[NV]** original semantics, **[ALT]** nvALT,
**[ULTRA]** nvUltra public docs, **[CP]** Commonplace's own additions
or deliberate divergences. **DECISION** marks a gate: the phase opens
by recording the chosen option here or in `spec.md`, with the
alternatives written down, before code lands.

**Licensing lineage rule** (from map §8): shared upstream files are
ported from the `nvalt/` tree's form (that is the lineage users run);
`scrod/nv` master's post-2011 additions are behavior reference only.
Commonplace is GPL-3.0, which licenses both trees as sources either
way; attribution to Zachary Schneirov rides in README + LICENSE.

---

## Phase 0: Skeleton (done)

- [x] Three-crate workspace (`commonplace-core` engine,
      `commonplace` GTK4 shell, `commonplace-cli`), green tests,
      clippy clean, fmt clean, Docket-shaped CI with fleet pins.
- [x] Contract (`spec.md`), this roadmap, README with the upstream
      acknowledgment and the ode, patchnotes seed, placeholder logo.
- [x] Reference corpus: `.repos/nv` + `.repos/nvalt` clones, 31
      snapshot pages in `research/raw/`, four research notes under
      `research/`.

## Phase 1: Engine: note store, read side

Goal: load a notes folder into memory exactly the way nvALT reads one.
The engine stays dependency-free and headless.

Ports [NV]/[ALT]: the note record (`NoteObject`, map §1.1) and the
directory scan (`NotationDirectoryManager`, map §1.1):

- First-line-as-title derivation at load: skip leading whitespace,
  break at `\n`/`\r`/`\t`/Unicode separators, 60-char cap with
  break-at-last-space-within-final-10, empty title becomes
  "Untitled Note" (map §4, `NSString_NV.m:265-328`).
- Extension admission table: `md, txt, text, utf8, taskpaper` plain
  text set with `md` default; dot-files and the upstream metadata
  filenames (`Notes & Settings`, `Interim Note-Changes`) skipped
  (map §4, `NotationPrefs.m:252-271`).
- Encoding read path simplified for Linux [CP]: UTF-8 always, BOM
  sniff on read, lossy-replace never silently (the xattr encoding
  bookkeeping of `NoteObject.m:1554,1369-1381` has no Linux analog;
  the divergence is recorded in spec §4/§5).
- No line-ending normalization on read (map §4: upstream does none).

Deliverables: `commonplace-core::store` with `load_folder`,
`Note` (title, body, path, mtime), sorted stable view. Tests: golden
fixture folders (titles, caps, weird filenames, dotfiles, empty notes,
BOMs), all headless.

- [ ] `store::load_folder` + `Note` + sorted stable view
- [ ] golden fixture-folder tests pass headless
- [ ] docs carried: spec divergence rows, patchnotes entry

## Phase 2: Engine: query language

Goal: one parser covering the three documented grammars of the line,
superset-ordered from weakest to strongest.

Ports: NV tokenization (implicit AND, colon/space separators, quoted
phrases via re-tokenization, map §2 `NotationController.m:1358-1499`);
nvALT tag words (`#tag` with prefix matching, delta §1; map §2
`TagSearch.m`); nvUltra booleans: `AND`/`OR`/`NOT` with `&&`/`&`,
`||`/`|`, `!` aliases and parentheses, `tag:X` full-name operator,
quoted exact ordered phrases, case + diacritic folding (ultra §1
search-syntax, verified against the manual snapshot
`research/raw/nvultra-help-search-syntax.html`).

DECISION (record in spec §5 rows): tag matching defaults to **nvALT
prefix matching** with a full-name strictness flag [ULTRA-toggle];
diacritics **fold** (nvUltra behavior, a documented divergence from
NV's byte matching).

Deliverables: `commonplace_core::query`: lexer, parser (precedence:
parens > NOT > AND > OR), AST, golden tests over both upstream
vocabularies and the nvUltra manual's examples (`#startrek AND (worf
OR geordi) NOT picard`).

- [ ] lexer + parser + AST
- [ ] the two DECISION rows recorded in spec §5 before code lands
- [ ] golden tests over both upstream vocabularies + manual examples
- [ ] docs carried: spec §5 rows, patchnotes entry

## Phase 3: Engine: matcher, incremental filter, ordering

Goal: reproduce the famous as-you-type feel headlessly, and prove it
at 10k notes (brief §3 performance baseline).

Ports [NV]: no inverted index by design: per-note lowercased caches
plus prefix-resume incremental filtering (`NoteObject.m` string
caches, `filterNotesFromUTF8String:` PHASE 1 prefix check and
`useCachedPositions` resume, map §2); implicit AND per token; quoted
phrase substring match; tag-word refilter rule (`NVQueryNewlyHasTagWord`).
Ordering [NV]: stable sort, primary = column function
(title/labels/dateModified/dateCreated), secondary = title
alphabetical, reverse flag flips both (`sortAndRedisplayNotes`
map §2, `NotationController.m:1588-1616`); no relevance score.
Selection rule [NV]: first title-prefix match selected, "prefix
parent" wins (map §2 PHASE 4, `NotationController.m:1460-1491`).

Deliverables: `search` module with the incremental state machine
typed in Rust (`FilterState` carries the previous query + candidate
set), property tests (result set == naive re-filter at every step),
a 10k-note benchmark with a latency gate recorded in the test.

- [ ] `search` module: incremental `FilterState` + stable sort + selection rule
- [ ] property tests (incremental == naive at every step)
- [ ] 10k-note benchmark with a recorded latency gate
- [ ] docs carried: spec rows, patchnotes entry

## Phase 4: Engine: write path

Goal: creating, renaming, deleting notes with upstream's exact file
rules and house atomicity.

Ports [ALT]: filename derivation `uniqueFilenameForTitle:` (`:` →
`-`, leading `.` → `_`, 255-byte cap, case-insensitive collision
numbering `title.N.ext`, keep-existing-extension rule, map §4
`NotationFileManager.m:412-456`); title→rename flow with
revert-on-failure and wiki-link rewriting on rename
(`updateLinksToNote:` map §4); tombstoned trash semantics simplified
[CP]: delete moves the file to the freedesktop Trash (DECISION: hand
trash helper in-core vs app-level; recommendation: in-core, stdlib
only). Atomic writes [ALT]: temp + rename, `renameat2(RENAME_EXCHANGE)`
for the replace case where upstream used `FSExchangeObjects`
(map §7 portability note 4). External-change detection [ALT]:
mtime+size reconciliation pass (inotify joins at Phase 10; the
reconciliation algorithm lands here, `synchronizeNotesFromDirectory`
semantics).

DECISION: Syncthing conflict files (`*.sync-conflict-*`, brief §3)
are admitted as ordinary notes at scan time [CP], never hidden; a
note whose title collides after admission gets the collision suffix.

Deliverables: `store::write`: create, rename, delete, atomic save,
rescan-reconcile. Tests: collision tables, rename revert, atomicity
under simulated crash (kill mid-write), conflict-file admission.

- [ ] create / rename (revert-on-failure, link rewrite) / delete-to-trash
- [ ] atomic save + rescan-reconcile
- [ ] the trash DECISION answered in the ledger before code lands
- [ ] tests: collision tables, rename revert, crash-mid-write, conflict admission
- [ ] docs carried: spec rows, patchnotes entry

## Phase 5: Engine: tags, metadata blocks, links

Goal: the portable tag store and the link graph, with no xattrs.

Ports [ULTRA]: MultiMarkdown metadata blocks (`Title:`/`Tags:`/free
keys) parsed as the primary tag store; `[[wiki links]]` and
`{{transclusion}}` recognized (ultra §1 tagging-and-metadata,
advanced-features). Ports [ALT]: inline `#tag` recognition in body
text (delta §1); tags participate in search via Phase 2's tag words.

DECISION (the Linux tag store; brief §3 says never xattrs): metadata
blocks are primary; inline `#tags` are recognized as tags too [CP
superset]; a derived tag index (rebuildable) serves the sidebar. No
Finder-tag parity: documented divergence (map §7 note 5).

Deliverables: metadata-block parser (MMD block rules: first block,
blank-line terminated), link extraction + rename rewriting hookup for
Phase 4, derived backlink index [CP, brief §2 gap]. Tests: golden
blocks, transclusion cycles rejected, backlink correctness under
rename.

- [ ] metadata-block parser
- [ ] link extraction + rename rewriting hookup
- [ ] derived backlink index
- [ ] tests: golden blocks, transclusion cycles, backlinks under rename
- [ ] docs carried: spec rows (incl. the Finder-tag divergence), patchnotes

## Phase 6: Engine: render boundary + renderer decision

Goal: Markdown → HTML for preview/export behind an in-core trait, so
the engine stays dependency-free while both frontends share one
pipeline.

Context from research: nvALT 3.x renders via Apex (vendored cmark-gfm
"unified", map §5); nvUltra embeds MultiMarkdown 6/libMultiMarkdown
(ultra §1, §5); upstream MMD is deprecated toward MMD 7 with no
nvUltra migration statement (ultra §6). Commonplace does not need MMD
metadata-through-renderer (Phase 5 parses metadata natively).

DECISION (gate, options in order):
1. **comrak (recommended)**: pure Rust, GFM + extensions, emits
   `sourcepos` for scroll-sync (map §5 PreviewSync needs it), active
   maintenance. Costs one sanctioned dependency in the frontend
   wiring (not in core: core defines `trait Renderer`, the app/CLI
   wire it).
2. pulldown-cmark: lighter, fewer extensions, no transclusion story.
3. libMultiMarkdown FFI: closest to nvUltra, MMD-specific syntax
   native; C dependency, deprecated upstream.

Transclusion (`{{...}}`) is expanded before render [CP] regardless of
choice (upstream renderer does not own it either).

Deliverables: `render::Renderer` trait + a test stub renderer in
core; the comrak wiring lands in the frontend crates with its
sanctioned dependency. Tests: trait contract, transclusion expansion,
math/mermaid/wikilink detection flags (map §5
`NVPreviewHTMLPreparingContent`).

- [ ] renderer DECISION answered in the ledger
- [ ] `render::Renderer` trait + test stub in core
- [ ] renderer wiring in the frontend crates (sanctioned dependency)
- [ ] transclusion expansion + detection-flag tests
- [ ] docs carried: spec rows, patchnotes entry

## Phase 7: Engine: export + import core

Goal: file-in, file-out parity for the formats Commonplace keeps.

Ports [NV]/[ALT]: multi-note export flow (map §1.5 ExporterManager),
note → HTML via Phase 6, note → plain text. Cut with rationale [CP]:
RTF/DOC/DOCX read+write (map §7 note 2: Cocoa attributed-string
machinery with no Linux analog), Textile, Stickies import, .blor.

Deliverables: `export` verbs in core (dir of notes → dir of exports;
single note → HTML/text). Tests: round-trip idempotence
(export → re-import == byte-stable for text), HTML well-formedness.

- [ ] export verbs (dir → dir; single note → HTML/text)
- [ ] round-trip idempotence + well-formedness tests
- [ ] the cut list recorded in spec §4
- [ ] docs carried: spec §4, patchnotes entry

## Phase 8: CLI: verb surface + machine shapes

Goal: the scriptable half, the house exit taxonomy, `--json` shapes.

Surface (additive from 1.0, spec §6): `commonplace-cli search`,
`list`, `open`, `new`, `append`, `export`, `tags`, `cat`, `render`,
`config`; `--json` envelopes; exit codes 0 success/empty, 1 findings,
2 usage (spec §6). Arg parsing: DECISION hand-rolled (current) vs
clap; recommendation stays hand-rolled until verb count hurts.
Config: `~/.config/commonplace/config.toml` (XDG, brief §3), default
notes dir `~/Documents/Notes` [CP; upstream default was an
app-support dir].

Deliverables: verbs over the Phase 1-7 engine, `--json` golden
shapes, README usage section. Tests: exit-code table, json schema
snapshots.

- [ ] verb surface over the Phase 1-7 engine
- [ ] `--json` envelopes + exit-code table tests
- [ ] config file + default notes dir
- [ ] README usage section (user-facing docs, not deferred)
- [ ] docs carried: spec §6 exits, patchnotes entry

## Phase 9: CLI: saved searches, scheme half, completions

Ports [ULTRA]: saved searches with hotkey slots ⌘1-⌘9/⌘0 (ultra §1)
→ stored in config, invocable as `commonplace-cli search --saved N`
and reused by the app's Phase 17 [CP shared store]. Shell completions
[CP]. The URL-scheme verbs get their CLI entry points here
(`find`/`make`/`markdownify` argument handling), sharing Phase 18's
routes.

- [ ] saved-search store + `--saved N`
- [ ] shell completions
- [ ] scheme-verb CLI entry points sharing Phase 18 routes
- [ ] docs carried: patchnotes entry, README machine surface
- [ ] M1 fired (documentation catch-up milestone below)

## Phase 10: App: shell, list, watching

Goal: the two-pane NV layout, styled, watching the folder.

Ports [NV]/[ALT]: vertical layout (search field above the results
list, editor right), row = title + body preview + label chips
(`UnifiedCell` map §1.2); column sort picker (title/labels/modified/
created, map §1.2 `NoteAttributeColumn`); empty state. GTK shape
[CP]: `GtkColumnView` or custom list widget: DECISION at phase open
(map §7 note 6: no in-cell editing; row editing lands Phase 17).
vir-gtk styling + Kanagawa from day one.

File watching: DECISION hand-rolled inotify vs the `notify` crate
(recommendation: `notify` in the app crate only; core keeps the
Phase 4 reconcile pass as the fallback/ truth). Accessibility rides
standard widgets (brief §3).

Deliverables: window that opens a collection, lists notes live
against external edits, sorts, keyboard-navigates the list.
Tests: headless where possible; the watch behavior gets an
integration test with a temp dir.

- [ ] window + two-pane vertical layout + sort picker + empty state
- [ ] list widget DECISION answered at phase open
- [ ] vir-gtk styling + Kanagawa from day one
- [ ] file watching (DECISION answered; reconcile stays the truth)
- [ ] watch integration test (temp dir + external edits)
- [ ] docs carried: spec rows, patchnotes entry

## Phase 11: App: the search-field state machine

Goal: the defining UX, ported 1:1. This is the phase the project
exists for.

Ports [NV]/[ALT] (map §3, every bullet carries its citation):
incremental filter per keystroke; completion-selected remainder
inserted into the field (AppController.m:1725-1741); empty query
deselects and clears; **Return with no selection creates the note
titled with the field text** (`createNoteIfNecessary`
AppController.m:2149-2171); typing in an empty body creates too
(m:2054-2057); Escape clears field+selection, refilters, refocuses
(m:1428-1453, Ctrl-[ alias); Up/Down forwarded to the list with the
field's text kept selected (m:1465-1481); Cmd-Up/Down first/last;
Tab semantics (m:1483-1500); the field keeps typed text on selection
(3.0.39, delta §1); snapback stack push on navigation (m:1837-1838);
launch restore of last search + last note + scroll
(m:2174-2190); "Enter in list renames note" pref behavior (delta §1
3.0.28).

Deliverables: the field + list interaction contract, golden-tested
as a state machine (headless model + thin GTK shell over it, the
Quire evaluator pattern). Tests: the full AppController behavior
table as unit tests on the model.

- [ ] headless state-machine model (Quire evaluator pattern)
- [ ] the full AppController behavior table as unit tests
- [ ] thin GTK shell wiring over the model
- [ ] launch restore (last search + last note + scroll)
- [ ] docs carried: spec rows, patchnotes entry

## Phase 12: App: editor pane

Goal: the plain-text editing core under the Markdown layer.

Ports [NV]/[ALT]: per-note undo stacks (map §7 note 8); per-note
scroll + caret restore (delta §1, 3.0.40 `LastNoteTopCharIndexKey`);
word count with the 300KB blank-past-cap contract (delta §1 3.0.38);
click-below-text lands at end (`ClickBelowText.h` map §1.4); external
editor handoff [ALT→CP]: stage + watch replaces ODBEditor
(map §7 note 9), `$EDITOR` plus a picker.

DECISION: autosave on by default with a short debounce [CP;
upstream saves on quit/close, delta §1 "auto-save not found"]. This
aligns with the Docket autosave precedent and Syncthing safety
(invariant 4).

Deliverables: GtkSourceView pane with markdown lang, autosave,
undo, scroll memory, word count, external-editor round trip.
Tests: editor-model tests headless; autosave debounce timing test.

- [ ] GtkSourceView pane with markdown lang
- [ ] autosave (DECISION answered) + debounce timing test
- [ ] per-note undo + scroll/caret restore + word count (cap contract)
- [ ] external-editor round trip ($EDITOR + picker)
- [ ] docs carried: spec rows, patchnotes entry

## Phase 13: App: Markdown editing I: highlighting + tasks

Ports [ALT] (delta §6, the UX contract): markdown syntax
highlighting via GtkSourceView's engine with the **300KB cap
contract**: plain past the cap, only edited lines re-colored below it
(`MarkdownHighlight.h:6`, per-edit line ranges map §1.4); task lists:
`- [ ]` toggle (opt-Cmd-T analog), click-in-bracket toggling,
done-strikethrough honoring ` @done`, Return continuation with
`[ ] `, empty-checkbox Return clears, checkbox-aware Tab, all
suppressed inside fences (`TaskList.h` semantics, delta §6);
==highlight==/CriticMarkup spans colored from the theme
(delta §6 3.0.36).

Deliverables + tests: a highlight-model crate module (pure
computation: ranges from buffer text) tested headless against the
upstream contract table, thin sourceview wiring.

- [ ] highlight model (pure ranges-from-buffer module)
- [ ] 300KB cap contract implemented + tested
- [ ] task-list behavior set (toggle, click, continuation, fence suppression)
- [ ] ==highlight==/CriticMarkup spans from theme
- [ ] thin sourceview wiring
- [ ] docs carried: spec rows, patchnotes entry

## Phase 14: App: Markdown editing II: lists, fences, pairs, reflinks

Ports [ALT]: Return continuation for bullets/numbers/quotes/fences
with nesting (`insertMarkdownStructureNewline`, delta §6); numbered
renumbering debounced 0.05s, single undo group, skipped past 300KB
(map §1.4 m:3594-3721); fence continuation + bounded-close rule that
kills the long-note return-key hang (map §1.4 m:4985); Tab/Shift-Tab
indent rules incl. 3.0.40's blank-line rule; list-marker cycling
`-`→`*`→`+`→numbered→none (delta §6); auto-pairing with the backtick
fence rules, pref-gated (delta §6 `NVAutoPair`); reference-link
completion `[text][id]` + F6/F7 jumps (delta §6); paragraph move
Ctrl-Cmd-Up/Down; expand/shrink selection by Markdown structure
(delta §6).

Deliverables + tests: same pattern: pure buffer-transform functions
with golden tables, then keybinding wiring. The full nvALT shortcut
table lands as the default keymap (delta §1 3.0.7 batch).

- [ ] buffer-transform functions (lists, fences, renumbering, markers)
- [ ] auto-pairing + reference-link completion + F6/F7
- [ ] paragraph move + expand/shrink selection
- [ ] golden transform tables headless
- [ ] default keymap = the nvALT shortcut table
- [ ] docs carried: spec rows, patchnotes entry

## Phase 15: App: preview pane

Ports [ALT]: live side-by-side preview with the debounce contract
(0.05s small / 0.6s large, map §5 PreviewSlice.h); **>300KB slicing**:
render ±64KB around the caret with 16KB fence lookback so WebKit
never sees the whole file (map §5); in-place content swap between
marker comments preserving scroll (map §5 m:650-666); scroll-to-edit
via renderer sourcepos (Phase 6 comrak sourcepos ↔ `PreviewSync`
line math, map §5); themes: user CSS in XDG themes dir, import from
file/URL, `#wrapper` wrap contract (map §5 theming); math
(MathJax/KaTeX: DECISION which), mermaid, highlight.js with
light/dark palette picked from background, wikilinks clickable in
preview staying plain in code/math/diagrams (map §5 preview-js);
preview templates with `{%title%}`/`{%style%}` slots (map §5);
save-HTML export (map §5).

Cut with rationale [CP]: Marked handoff, `x-marked://stream/`,
share-to-Marked (macOS app interop; map §5).

- [ ] live preview + debounce contract (0.05s / 0.6s)
- [ ] >300KB slicing contract
- [ ] in-place content swap + scroll preservation
- [ ] scroll-to-edit via sourcepos
- [ ] themes (XDG dir, import, `#wrapper`) + templates
- [ ] math DECISION answered; mermaid + highlight.js + wikilink injection
- [ ] save-HTML export; the Marked cuts recorded in spec §4
- [ ] docs carried: spec rows, patchnotes entry

## Phase 16: App: collections (notebooks)

Ports [ULTRA]: unlimited simultaneous notebooks/collections, folder
switcher (⌃⌘N analog), per-collection settings (extensions, default
extension), subfolders displayed flat as `Sub/Note.md`, slash-titled
note creation creating the folder (ultra §1), search scoped to the
active collection [NV: single folder is the floor].

Deliverables: multi-collection config, switcher UI, per-collection
store instances. Tests: config round-trip, cross-collection
isolation.

- [ ] multi-collection config + switcher UI
- [ ] per-collection settings + store instances
- [ ] subfolder flat display + slash-title creation
- [ ] config round-trip + isolation tests
- [ ] docs carried: spec rows, patchnotes entry

## Phase 17: App: tags UI, bookmarks, row editing

Ports [ALT]: label/tag sidebar (`LabelsListController` map §1.6),
tag-token chips with multi-note apply and comma-chaining (delta §1
3.0.13/14), in-row title/tag editing with column swap
(map §1.2 `NotesTableView.m:1093-1112`, Phase 10's DECISION pays
here); in-list rename-vs-jump Enter pref (delta §1 3.0.28).
Ports [NV]: bookmarks/snapback as note+searchString pairs
(`BookmarksController` map §1.7). Ports [ULTRA]: saved searches on
⌘1-⌘9/⌘0 over the Phase 9 shared store.

- [ ] tag sidebar + tag-token chips + multi-note apply
- [ ] in-row title/tag editing (Phase 10's DECISION pays here)
- [ ] bookmarks/snapback
- [ ] saved searches on hotkey slots
- [ ] docs carried: spec rows, patchnotes entry

## Phase 18: App: URL scheme + deep linking + single instance

Ports [ALT] (delta §3 route table): `commonplace://find/<terms>`
(with UUID reveal semantics), `/make` (`title`/`txt`/`html`/`tags`/
`url`), `/markdownify`, `/add-theme`, bare-host-as-search; the
nvUltra compatibility note (ultra §1: its `find` is deliberately
nvALT-compatible) means Commonplace also answers `x-nvultra://find`
shape-for-shape [CP courtesy]. Linux mechanics [CP]: `.desktop`
x-scheme-handler registration (brief §3), GTK single-instance
activation with the URL routed to the running window.

- [ ] route table (find/make/markdownify/add-theme/bare-host)
- [ ] `x-nvultra://find` shape compatibility
- [ ] `.desktop` scheme-handler registration
- [ ] single-instance activation routing URLs to the running window
- [ ] docs carried: spec rows, patchnotes entry, README automation section

## Phase 19: App: preferences + theming

Ports [ALT] (delta §4 pane inventory): General (folder, external
editor, list font/size, activation hotkey), Editing (marker choices
bold/italic/list, completion on/off, Tab behavior, soft tabs, auto
formats, auto-pair, link suggest, styled paste, search highlight +
color), Fonts & Colors (body/list fonts, user scheme, editor theme +
preview theme pickers with import, max text width), per-collection
Notes pane (formats/extensions, delete confirmation), Debug pane
[CP: logging toggles + open-log]. Hidden keys from delta §4 carried
as config keys where they still make sense.

Deliverables: prefs model (typed config structs + GTK UI), theme
file format = nvALT-compatible `.theme` JSON where feasible [ALT
interoperability, map §1.5 NVTheme] and plain CSS preview themes.

- [ ] prefs model (typed config structs)
- [ ] the five panes (General/Editing/Fonts & Colors/Notes/Debug)
- [ ] `.theme` compatibility + CSS preview themes
- [ ] hidden keys carried as config keys where they make sense
- [ ] docs carried: spec rows, patchnotes entry
- [ ] M2 fired (documentation catch-up milestone below)

## Phase 20: Hardening: sync-safety, perf, large notes

Goal: make invariant 4 (Syncthing-safe) and invariant 7 (10k notes,
instant) provable rather than intended.

Deliverables: inotify + full-reconcile convergence tests (editor and
CLI and external writer interleaved); conflict-file lifecycle test;
no-lock/no-sidecar audit of the notes tree (brief §3, Zettlr's
`.ztr-directory` as the named counterexample); the 300KB contracts
end-to-end (highlighting off, renumber off, fence-close skip, word
count blank, preview sliced, delay 0.6s) asserted in one integration
suite; 10k-note benchmark gates (filter keystroke budget, cold load
budget) in CI; fractional-scale + Wayland smoke pass (brief §3).

- [ ] convergence tests (editor + CLI + external writer interleaved)
- [ ] conflict-file lifecycle test
- [ ] no-lock/no-sidecar audit of the notes tree
- [ ] 300KB contracts end-to-end in one integration suite
- [ ] 10k-note perf gates wired into CI
- [ ] fractional-scale + Wayland smoke pass
- [ ] docs carried: spec invariant evidence, patchnotes entry

## Phase 21: Importers

Ports [ALT]: Import URL with Readability processing and the
markdownify verb (delta §1; map §1.10); file-drop/file batch import
(`AlienNoteImporter` flow, map §1.10); HTML paste → text. One-way
Simplenote escape hatch [CP, per the scoping decision]: import a
Simplenote export archive into a folder (no live sync engine ever;
spec §4). Cut [CP]: Safari extension (browser clippers are ecosystem
work; the URL scheme covers automation).

DECISION: HTML → Markdown converter (options: a small sanctioned
Rust crate in the CLI/app wiring, or a minimal in-core converter
porting the upstream HTMLToMarkdown semantics; recommendation:
sanctioned crate, benchmarked against upstream behavior examples).

- [ ] URL import + markdownify verb
- [ ] file-drop / batch import
- [ ] Simplenote export-archive importer (one-way, per the ledger)
- [ ] converter DECISION answered
- [ ] Safari-extension cut recorded in spec §4
- [ ] docs carried: spec §4, patchnotes entry, README import section

## Phase 22: Encryption (gated, optional)

Upstream context [NV]: transparent library encryption was an NV
pillar (brief sources: notational.net's "Transparent Database
Encryption"); nvALT kept per-library encryption; files-first changes
the shape entirely [CP]. Options: (a) per-collection encrypted
container (closest to NV, breaks plain-file invariant 1: rejected
unless files-first is amended), (b) per-note age encryption with
libsecret-held keys (interop-friendly, breaks grep-ability of the
tree), (c) defer with an explicit spec note (recommendation: c for
1.0; revisit post-1.0 with real usage). The gate must be answered
even if the answer is "defer".

- [ ] the gate answered in the ledger (even if the answer is "defer")
- [ ] if greenlit: implementation + tests
- [ ] docs carried: spec §4 outcome, patchnotes entry

## Phase 23: Packaging + release infrastructure

Deliverables: real logo (placeholder dies), desktop file +
AppStream metainfo + icons (Docket's `data/` shape), the hardened
`release.yml` copied from Docket with Commonplace's install tree
(built artifact attached to the release, house rule), Flatpak
manifest `io.github.virinvictus.Commonplace` (app-id split note:
already in the right namespace), scheme-handler registration riding
the desktop file, first real release with the verbatim-tag
procedure.

- [ ] real logo (placeholder dies)
- [ ] desktop file + metainfo + icons
- [ ] hardened `release.yml` (artifact attached to the Release)
- [ ] Flatpak manifest in the right namespace
- [ ] first tagged release: verbatim tag message + GitHub Release in
      the same session (AGENTS.md release rule)
- [ ] docs carried: README install/usage truth, patchnotes entry

## Phase 24: Parity audit + 1.0

Goal: the contract-is-the-promise release (the bindery 1.0
precedent).

Deliverables: the spec §5 parity table verified row-by-row against
the research corpus (a claim-verifier pass over every confirmed/
refuted row); doc-drift + release-audit fleet passes; the perf
gates re-run on real note collections; patchnotes + roadmap
reconciliation; 1.0 tagged only after all of it is green.
Post-1.0 the flag/verb surface is additive (spec §7).

- [ ] parity table verified row-by-row (claim-verifier dispatch)
- [ ] doc-drift + release-audit passes green
- [ ] perf gates re-run on real note collections
- [ ] patchnotes + roadmap reconciliation (M3)
- [ ] 1.0 tagged + GitHub Release cut

---

## Documentation catch-up milestones

Documentation updates ride the commits that ship the code (AGENTS.md,
"Documentation surfaces"); there is no Unreleased holding pen and no
deferred docs sweep. These milestones exist only to catch what
slipped. Each is a full five-surface pass (README, AGENTS.md, spec,
patchnotes, roadmap) plus a `doc-drift-auditor` dispatch, and any
drift it finds is fixed in the same session.

- [ ] **M1, fires at the end of Phase 9** (engine + CLI arc
      complete): all five surfaces re-read against the shipped
      engine and CLI verbs; spec §5 phase references re-checked;
      README's CLI usage section verified against the real verbs and
      exit codes.
- [ ] **M2, fires at the end of Phase 19** (app arc complete): same
      pass over the GUI surface; README gains real screenshots and
      keyboard-flow docs; AGENTS.md's decisions and the ledger
      reconciled; every parity row touched by Phases 10-19
      re-verified.
- [ ] **M3 = Phase 24**: the 1.0 audit is the final catch-up;
      nothing ships 1.0 with a stale claim anywhere.

---

## Decision ledger (running)

| Decision | Phase | Status |
|---|---|---|
| Sync scope: files-first, no cloud; Simplenote importer only | 21 | decided 2026-11-17 with Brandon (spec §4) |
| Tag matching: nvALT prefix default, strict flag | 2 | decided (roadmap) |
| Diacritic folding: on | 2 | decided (roadmap) |
| Trash: in-core freedesktop trash | 4 | recommendation, confirm at phase |
| Renderer: comrak behind core trait | 6 | recommendation, confirm at phase |
| File watching: notify crate in app crate | 10 | recommendation, confirm at phase |
| Autosave: on, debounced | 12 | recommendation, confirm at phase |
| Preview math: MathJax vs KaTeX | 15 | open |
| HTML→Markdown converter | 21 | recommendation, confirm at phase |
| Encryption | 22 | recommendation: defer past 1.0 |
