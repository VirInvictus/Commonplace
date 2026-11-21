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
- `research/gtk-prior-art.md` (house GTK evidence: Framework/Docket/Viaduct/Atrium/Quire; "house §N")
- `research/interaction-contract.md` (complete nvALT keymap + nvUltra shortcuts + wiki-link semantics; "contract §N")
- `research/rust-crate-survey.md` (Rust ecosystem for the DECISION gates; "crates §N")
- `research/format-briefs-mmd-simplenote.md` (MMD metadata grammar + Simplenote export schema; "formats §N")
- `research/objc-rust-crosswalk.md` (the ObjC idiom dictionary: delegates, notifications, GCD, undo, runtime tricks; "crosswalk §N")
- `research/rust-model-notes.md` (core type sketches + the index-units contract; "models")

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

**Translation layers.** This is a macOS app being rebuilt for a
Thinkpad, not translated line-for-line. Every layer below crosses a
boundary; each names where the crossing is handled.

1. **Language and framework**: Objective-C/Cocoa → Rust + GTK4. A
   manual port against behavior contracts (the research corpus); no
   upstream code is compiled or linked.
2. **UI toolkit**: NSTextView / NSTableView / legacy WebView →
   GtkSourceView / GtkListView / WebKitGTK. Mapped per phase with
   house evidence (house §Q1-Q5); GTK4's gaps (no in-cell editing,
   no token fields, no NSUndoManager) are exactly where the Phase
   10/12/13/17 decisions live.
3. **Text model**: NSAttributedString everywhere upstream → plain
   String + editor-layer styling here (map §7 note 2). The file on
   disk is plain text in both; the in-memory rich model is an
   upstream artifact we deliberately drop. Also inside this layer:
   **index units**: upstream counts everything in UTF-16 code units
   (the 300KB cap, preview slices, editor offsets); Commonplace
   counts `char`s in-engine and converts to bytes only at the disk
   and comrak boundaries (sketch and rule in
   `research/rust-model-notes.md`, Phase 1 confirms).
4. **Filesystem services**: FSEvents → inotify via the notify crate
   (Phase 10; crates §2); FSExchangeObjects →
   renameat2(RENAME_EXCHANGE) (Phase 4); the com.apple.TextEncoding
   xattr → UTF-8 always + BOM sniff (Phase 1, recorded divergence);
   Finder tags → metadata blocks + inline tags (Phase 5: no
   desktop-standard tag store exists on Linux); the Apple single-DB
   library format → not ported at all (files-only, invariant 1).
5. **Platform services**: Keychain → libsecret (only if Phase 22
   greenlights); Sparkle updates → release artifacts + Flatpak
   (Phase 23); AppleScript/sdef → the CLI + `commonplace://` scheme
   (Phases 8/9/18); ODB external editing → `$EDITOR` + file watch
   (Phase 12); Spotlight was never used upstream (map §7 note 12)
   and is not introduced here.
6. **Keyboard notation**: the research tables are in Mac notation
   (⌘⌥⌃⇧). The Linux keymap derives mechanically: **mac ⌘ → Ctrl**
   (the GTK convention: copy is Ctrl+C), **mac ⌥ → Alt**, mac ⌃
   merges into the Ctrl space, and a binding that needs two
   modifiers maps its second to Alt (mac ⌃⌘X → Ctrl+Alt+X). Known
   collisions for the Phase 14 pass: mac ⌃⌘P (preview) translates
   onto ⌘P's print slot, so preview moves to Ctrl+Alt+P; nvALT's
   ⌃⌘↑/↓ (paragraph move) and nvUltra's ⌃⌥↑/↓ CONVERGE to the same
   Ctrl+Alt+Up/Down on Linux, which resolves the bind-both
   recommendation for free; nvUltra's ⌃⌘↑/↓ line-shift has no free
   translation and stays unbound. The Mac-notation tables in
   `research/interaction-contract.md` stay canonical; Phase 14
   derives and checks the Linux table against them.
7. **Hardware**: none: no Retina, TouchBar, or notch assumptions
   exist in any code we port; HiDPI rides GTK fractional scaling
   (Phase 20 smoke pass).

Layers 1-5 and 7 are already distributed through the phases; layer 6
is new as an explicit pass and joins Phase 14's checklist and the
decision ledger.

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
BOMs), all headless. Architecture notes (crosswalk §1-3): upstream's
informal delegate protocols become typed callbacks with
`Option<callback>` fields (the `respondsToSelector:` idiom), and the
store's change events feed the Phase 11 event bus; foreign-class
categories become free functions in per-class modules, except the two
that are intrinsic model behavior (title extraction, body
sanitization), which become inherent methods on `NoteTitle`/`NoteBody`.

- [x] `store::load_folder` + `Note` + sorted stable view
- [x] golden fixture-folder tests pass headless
- [x] docs carried: spec divergence rows (§5 rows 54-56); the
      patchnotes entry lands with the first tagged release, batched at
      the end of the engine arc after Phase 3 (Brandon's call,
      2026-11-17); nothing ships untagged

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

- [x] lexer + parser + AST
- [x] the two DECISION rows recorded in spec §5 before code lands
      (rows 57-58, plus the grammar resolutions pinned as row 59)
- [x] golden tests over both upstream vocabularies + manual examples
- [x] docs carried: spec §5 rows (the patchnotes entry lands with the
      engine-arc release, per the Phase 1 batching call)

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

- [x] `search` module: incremental `FilterState` + stable sort + selection rule
- [x] property tests (incremental == naive at every step; a 600-step
      seeded random edit walk plus scripted sequences)
- [x] 10k-note benchmark with a recorded latency gate (50 ms per
      keystroke on `--release`, measured 12.9 ms cold / 65-90 µs warm)
- [x] docs carried: spec rows (the patchnotes entry lands with the
      engine-arc release, per the Phase 1 batching call)

## Phase 4: Engine: write path

Goal: creating, renaming, deleting notes with upstream's exact file
rules and house atomicity.

Ports [ALT]: filename derivation `uniqueFilenameForTitle:` (`:` →
`-`, leading `.` → `_`, 255-byte cap, case-insensitive collision
numbering `title.N.ext`, keep-existing-extension rule, map §4
`NotationFileManager.m:412-456`); title→rename flow with
revert-on-failure. **Correction from contract §3B:** upstream's
wiki-link rewrite hook `updateLinksToNote:` is an EMPTY STUB in both
trees (NC:833-835), so upstream never rewrites links on rename; old
links survive only as searches for the old title. Commonplace
rewriting links on rename is a recorded [CP] improvement (ledger row:
yes, rewrite). Trash [CP]: the `trash` crate 5.4 (verified
freedesktop-compliant, light default deps, crates §3) in the app
crate beats hand-rolling; the engine exposes a delete-with-callback
boundary so core stays dependency-free. Atomic writes [ALT]: temp +
rename; upstream's `FSExchangeObjects` maps to std's `rename(2)`
atomic replace (spec row 5's recorded divergence: the dependency-free
core has no `renameat2`, and the observable contract - never a
half-written note - holds).
External-change detection [ALT]: mtime+size reconciliation pass
(inotify joins at Phase 10; the reconciliation algorithm lands here,
`synchronizeNotesFromDirectory` semantics; crates §2: atomic saves
arrive as Create+Rename pairs, so reconcile keys on path + re-stat,
not on event kind). Write coalescing has a documented asymmetry to
replicate exactly (crosswalk §10, NotationController.m:1086): a user
edit invalidates the performSelector-queued save but NOT the
timer-queued one; the Rust design keeps two independent debounce
handles (edit-cancellable write coalescer + fixed fsync cadence) so
saves coalesce exactly like nvALT's.

DECISION: Syncthing conflict files (`*.sync-conflict-*`, brief §3)
are admitted as ordinary notes at scan time [CP], never hidden; a
note whose title collides after admission gets the collision suffix.

Deliverables: `store::write`: create, rename, delete, atomic save,
rescan-reconcile. Tests: collision tables, rename revert, atomicity
under simulated crash (kill mid-write), conflict-file admission.

- [x] create / rename (revert-on-failure) / delete-to-trash boundary
- [x] link rewriting on rename (the [CP] improvement; upstream is a stub)
- [x] atomic save + rescan-reconcile (path + re-stat keying)
- [x] write coalescing: the two-handle cancellation matrix (edit cancels
      the write coalescer, not the fsync cadence; crosswalk §10)
- [x] the trash-crate decision confirmed in the ledger before code lands
- [x] tests: collision tables, rename revert, crash-mid-write, conflict admission
- [x] docs carried: spec rows (the patchnotes entry lands with the
      v0.3.0 release, per the release-batching section)

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

DECISION (the `Title:` interaction; formats §1): upstream evidence is
one-sided: nvALT has zero `Title:` awareness (`Title: Foo` lists as
"Title: Foo", verified in the clone) and nvUltra documents metadata
tags but never states how `title:` interacts with the displayed name.
Commonplace pins its own rule in spec before code: recommendation,
**`Title:` metadata overrides the displayed title when present**,
falling back to first-line, with the raw first line always available
as the filename source [CP; supersedes neither tier]. The exact
grammar this phase parses is pinned in formats §1 (first-block
placement, blank-line termination, case-insensitive keys with spaces
stripped, URL-precedence rule, `Keywords:`/`Base Href:` are folklore
not standard keys, and the MMD-vs-YAML-frontmatter distinguishing
rules).

Deliverables: metadata-block parser (MMD block rules: first block,
blank-line terminated), link extraction + rename rewriting hookup for
Phase 4, derived backlink index [CP, brief §2 gap]. Tests: golden
blocks, transclusion cycles rejected, backlink correctness under
rename.

- [x] metadata-block parser (grammar pinned from formats §1)
- [x] the `Title:`-interaction DECISION recorded in spec before code lands
- [x] link extraction + rename rewriting hookup
- [x] derived backlink index
- [x] tests: golden blocks, transclusion cycles, backlinks under rename
- [x] docs carried: spec rows (incl. the Finder-tag divergence); the
      patchnotes entry lands with the v0.3.0 release

## Phase 6: Engine: render boundary + renderer decision

Goal: Markdown → HTML for preview/export behind an in-core trait, so
the engine stays dependency-free while both frontends share one
pipeline.

Context from research: nvALT 3.x renders via Apex (vendored cmark-gfm
"unified", map §5); nvUltra embeds MultiMarkdown 6/libMultiMarkdown
(ultra §1, §5); upstream MMD is deprecated toward MMD 7 with no
nvUltra migration statement (ultra §6). Commonplace does not need MMD
metadata-through-renderer (Phase 5 parses metadata natively).

DECISION (gate, options in order; crates §1):
1. **comrak (recommended)**: hit 1.0.0 on 2026-11-17 (BSD-2, MSRV
   1.89, published semver policy): full GFM plus footnotes, wikilinks
   (both pipe orders), math dollars, and metadata blocks;
   `Render.sourcepos` emits position attributes straight into the
   HTML (blocks solid; lists/description-lists weak, the known limit
   for scroll-sync); safe by default (raw HTML scrubbed; `unsafe` +
   ammonia when the preview needs injection). Costs one sanctioned
   dependency in the frontend wiring (not in core: core defines
   `trait Renderer`, the app/CLI wire it).
2. pulldown-cmark 0.13.6: active but leaner; only source mapping is
   `(Event, Range)` via `into_offset_iter`.
3. libMultiMarkdown FFI: no upstream-alive bindings crate exists
   (crates §1); closest to nvUltra, MMD-specific syntax native; C
   dependency, upstream deprecated.

Transclusion (`{{...}}`) is expanded before render [CP] regardless of
choice (upstream renderer does not own it either).

Deliverables: `render::Renderer` trait + a test stub renderer in
core; the comrak wiring lands in the frontend crates with its
sanctioned dependency. Tests: trait contract, transclusion expansion,
math/mermaid/wikilink detection flags (map §5
`NVPreviewHTMLPreparingContent`).

- [x] renderer DECISION answered in the ledger
- [x] `render::Renderer` trait + test stub in core
- [x] renderer wiring in the frontend crates (sanctioned dependency;
      CLI crate now, app crate joins at Phase 15 per the ledger)
- [x] transclusion expansion + detection-flag tests
- [x] docs carried: spec rows 66-68 (the patchnotes entry lands with
      the v0.4.0 release, per the release-batching section)

Phase-open re-diff (2026-11-20): upstream moved `6a4c000..69754e5`
(five commits, nvALT 3.0.41/42: list-formatting restore, note-flash
fixes, bold-italic fix, MarkdownHighlight/NVAutoPair/SearchHighlight
changes, and a save/scan no-op guard in NoteObject.m that preserves
undo on self-inflicted rescans). None of it touches render or export;
the window parks for the Phases 12-14 phase opens. Phase tip assumed:
69754e5.

## Phase 7: Engine: export + import core

Goal: file-in, file-out parity for the formats Commonplace keeps.

Ports [NV]/[ALT]: multi-note export flow (map §1.5 ExporterManager),
note → HTML via Phase 6, note → plain text. Cut with rationale [CP]:
RTF/DOC/DOCX read+write (map §7 note 2: Cocoa attributed-string
machinery with no Linux analog), Textile, Stickies import, .blor.

Deliverables: `export` verbs in core (dir of notes → dir of exports;
single note → HTML/text). Tests: round-trip idempotence
(export → re-import == byte-stable for text), HTML well-formedness.

- [x] export verbs (dir → dir; single note → HTML/text)
- [x] round-trip idempotence + well-formedness tests
- [x] the cut list recorded in spec §4
- [x] docs carried: spec §4 + rows 69-70 (the patchnotes entry lands
      with the v0.4.0 release, per the release-batching section)

## Phase 8: CLI: verb surface + machine shapes

Goal: the scriptable half, the house exit taxonomy, `--json` shapes.

Surface (additive from 1.0, spec §6): `commonplace-cli search`,
`list`, `open`, `new`, `append`, `export`, `tags`, `cat`, `render`,
`config`; `--json` envelopes; exit codes 0 success/empty, 1 findings,
2 usage (spec §6). Arg parsing: DECISION hand-rolled (current) vs
clap; recommendation stays hand-rolled until verb count hurts.
Config: `~/.config/commonplace/config.toml` (XDG, brief §3), default
notes dir `~/Documents/Notes` [CP; upstream default was an
app-support dir]. Parsing is a hand-written TOML subset in core, no
serde: the engine stays dependency-free (crosswalk §11; the ~300-line
subset writer-reader is the sanctioned replacement for upstream's
NSKeyedArchiver/NSUserDefaults plumbing).

Deliverables: verbs over the Phase 1-7 engine, `--json` golden
shapes, README usage section. Tests: exit-code table, json schema
snapshots.

- [x] verb surface over the Phase 1-7 engine (twelve verbs: the ten
      pinned plus the [CP] `completions` and `scheme` additions)
- [x] `--json` envelopes + exit-code table tests (schema snapshots in
      `tests/cli_golden.rs`, driven through the real parser)
- [x] config file + default notes dir (hand-written TOML subset, no serde)
- [x] README usage section (user-facing docs, not deferred)
- [x] docs carried: spec §6 exits (pinned at this phase), patchnotes
      entry lands with the v0.5.0 release

## Phase 9: CLI: saved searches, scheme half, completions

Ports [ULTRA]: saved searches with hotkey slots ⌘1-⌘9/⌘0 (ultra §1)
→ stored in config, invocable as `commonplace-cli search --saved N`
and reused by the app's Phase 17 [CP shared store]. Shell completions
[CP]. The URL-scheme verbs get their CLI entry points here
(`find`/`make`/`markdownify` argument handling), sharing Phase 18's
routes.

- [x] saved-search store + `--saved N` (the config's
      `saved_searches` slots; unset slots are usage errors, ledger)
- [x] shell completions (`completions bash|zsh|fish`, hand-written
      static scripts over the pinned surface)
- [x] scheme-verb CLI entry points sharing Phase 18 routes (`scheme
      find/make/markdownify`; find and make are functional, the
      markdownify/html/tags arms defer to Phase 21's converter with
      clean trouble lines)
- [x] docs carried: patchnotes entry (lands with v0.5.0), README
      machine surface (the CLI usage section)
- [x] M1 fired (2026-11-20: the five surfaces re-read against the
      shipped verbs; the exit-code table, config keys, and saved-slot
      semantics verified against the tests; README usage section
      smoke-run through the real binary)

## Phase 10: App: shell, list, watching

Goal: the two-pane NV layout, styled, watching the folder.

Ports [NV]/[ALT]: vertical layout (search field above the results
list, editor right), row = title + body preview + label chips
(`UnifiedCell` map §1.2); column sort picker (title/labels/modified/
created, map §1.2 `NoteAttributeColumn`); empty state. GTK shape
[CP]: **`GtkListView` + `SingleSelection` + `SignalListItemFactory`**
(house §Q3: the only list shape any house repo ships at scale; no
repo uses GtkColumnView; Atrium's title Stack, task_list.rs:338-367,
is the proven in-row editing shape Phase 17 needs; Viaduct's
400-char preview cap before Pango, timeline.rs:495-566, is the
load-bearing row rule for large bodies). vir-gtk styling + Kanagawa
from day one.

File watching: DECISION hand-rolled inotify vs the `notify` crate
(recommendation: `notify` 8.x in the app crate only, with
`debouncer-full` merging rename pairs; core keeps the Phase 4
reconcile pass as the fallback/truth. crates §2: the inotify backend
pairs MOVED_FROM/MOVED_TO by cookie into RenameMode::Both, v8 keeps a
single pending From so interleaved renames can mispair, and atomic
saves arrive as Create+Rename, which is why Phase 4 reconciles on
path + re-stat). Accessibility rides standard widgets (brief §3).

Deliverables: window that opens a collection, lists notes live
against external edits, sorts, keyboard-navigates the list.
Tests: headless where possible; the watch behavior gets an
integration test with a temp dir.

- [ ] window + two-pane vertical layout + sort picker + empty state
- [ ] list widget: GtkListView + GObject domain items (house §Q3)
- [ ] vir-gtk styling + Kanagawa from day one
- [ ] file watching (notify crate decision confirmed; reconcile stays the truth)
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
table as unit tests on the model. The complete verified key/behavior
table lives in contract §1J (Cmd-L, Return=create-or-edit, arrow
forwarding with text-kept-selected, Cmd-Up/Down, Tab semantics,
Ctrl-U, Esc-clear, Cmd-D, snapback) plus house latency patterns
(house §Q4: Atrium's `GtkSearchEntry` search-delay or Viaduct's
cancel-and-re-arm 150 ms timer for the expensive pass, Quire's
tick-callback coalescing for same-frame feedback, Docket's
completion-re-entry and deletion-diff field behaviors,
timeline.rs INVALID_LIST_POSITION deselect).

- [ ] headless state-machine model (Quire evaluator pattern)
- [ ] the full AppController behavior table as unit tests
- [ ] thin GTK shell wiring over the model
- [ ] launch restore (last search + last note + scroll)
- [ ] typing-in-a-focused-list forwards to the field (NTV:976-984;
      crosswalk §2: keyDown forwarding, synthesized via the capture
      controller, since GTK has no event-forwarding primitive)
- [ ] docs carried: spec rows, patchnotes entry

## Phase 12: App: editor pane

Goal: the plain-text editing core under the Markdown layer.

Ports [NV]/[ALT]: per-note undo stacks (map §7 note 8); per-note
scroll + caret restore (delta §1, 3.0.40 `LastNoteTopCharIndexKey`);
word count with the 300KB blank-past-cap contract (delta §1 3.0.38);
click-below-text lands at end (`ClickBelowText.h` map §1.4); external
editor handoff [ALT→CP]: stage + watch replaces ODBEditor
(map §7 note 9), `$EDITOR` plus a picker.

DECISION: autosave on by default, debounced (250-500 ms), with
Docket's `loading` guard discipline and explicit autosave() on
programmatic edits [CP; upstream saves on quit/close, delta §1
"auto-save not found": Docket itself ships synchronous
per-keystroke writes, house §Q2, which does not scale to note-sized
bodies; the debounce timer pattern is Viaduct's, with its
handle-clearing gotcha]. This aligns with Syncthing safety
(invariant 4). Ledger notes from crosswalk §9-12: per-note undo
stacks have no house precedent (single-buffer apps), but upstream's
undo behaviors are contracts to replicate: **undoing dirties the
file** (NoteObject.m:1928-1931) and undo-group nesting during undo
exists (NotationController.m:887-892). The modifier-hold popovers
(option/control held 1.2 s pop word count/preview,
AppController.m:3752-3789) are RECOMMENDED FOR DROP [CP]: GTK has no
flagsChanged observer, the feature is undiscoverable, and the popups
duplicate visible controls; recorded in the ledger at phase open.
The encrypted-notes temp-file guarantee maps upstream's RAM-disk
machinery to `$XDG_RUNTIME_DIR` (tmpfs) and only matters if Phase 22
ever greenlights (crosswalk §11).

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
(delta §6 3.0.36). Styling-layer sizing from crosswalk §8: the
upstream attribute surface is 13 attributes (foreground 56 sites,
font 40, background 22, strikethrough 20, paragraph style 20, link
15, underline 8); the AppKit-only pair is **faux bold/italic**
(NSStrokeWidth -3.50 / NSObliqueness 0.20, no Pango analog): DECISION
at phase open between a synthesized `pango::FontDescription` tag and
dropping faux styles to plain+CSS class (the markdown-conversion path
already converts them away, AppController_Markdown.m:205-210).

Deliverables + tests: a highlight-model crate module (pure
computation: ranges from buffer text) tested headless against the
upstream contract table, thin sourceview wiring. Gate shape from
house §Q2: no house repo does true per-edit-range incremental
application; the honest baseline is the GtkSourceView language
engine (stock `markdown.lang` first; a custom `.lang` is a proven,
documented path if NV-specific contexts are needed) plus Quire-style
signature-guarded tag passes for the spans GtkSourceView cannot
express, with the 300KB cap enforced BEFORE the pass. The stock
engine's ceiling for nvALT-style task/fence spans is the flagged
prototype risk (crates §7).

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
(delta §6). Keymap resolutions (contract §4C): paragraph-move binds
BOTH Ctrl-Cmd-Up/Down [ALT primary] and Ctrl-Opt-Up/Down [ULTRA
alias]; heading promote/demote adopts nvUltra's always-on
Opt-Cmd-]/[ over nvALT's pref-gated Cmd-Shift-=/-; quote keys
Cmd-Shift->/< ship ungated. Default keymap = the complete verified
table in contract §1.

Deliverables + tests: same pattern: pure buffer-transform functions
with golden tables, then keybinding wiring. The full nvALT shortcut
table lands as the default keymap (delta §1 3.0.7 batch).

- [ ] buffer-transform functions (lists, fences, renumbering, markers)
- [ ] auto-pairing + reference-link completion + F6/F7
- [ ] paragraph move + expand/shrink selection
- [ ] golden transform tables headless
- [ ] default keymap = the nvALT shortcut table
- [ ] Linux keymap derived from the Mac-notation tables (translation
      rule in the header: ⌘→Ctrl, ⌥→Alt, ⌃⌘X→Ctrl+Alt+X) with the
      collision pass (preview slot, paragraph-move convergence,
      line-shift unbound) recorded in the ledger
- [ ] docs carried: spec rows, patchnotes entry

## Phase 15: App: preview pane

Platform facts first (crates §6, house §Q1): `webkit6 = "0.6"` (repo
alive) against webkitgtk-6.0. The 6.0 API has **no `run_javascript`**
(use `evaluate_javascript`/`call_async_javascript_function`) and **no
scroll API at all**: scroll save/restore is JS-only, sequenced after
load-finished, and `load_html` resets scroll (fresh document).
Framework's proven architecture (house §Q1) is therefore the
baseline: `load_html` with a custom-scheme base_uri for same-origin
resources (fw-webview.c:677-686); post-load JS queued until
LOAD_FINISHED via pending fields; scroll tracked by an injected
200 ms-debounced user script posting `{anchor, scroll_y, frac}` to a
script-message handler with a host-side synchronous cache (a JS
round-trip cannot finish during dispose); reload = debounced full
load_html + restore_position (anchor-then-scrollTo fallback).
**In-place DOM replacement between markers (nvALT's marker-comment
trick) is unproven in house and is the slicing hot-path experiment**,
not the baseline: record both in the ledger. Security posture for
untrusted notes: raw-HTML policy (Framework's `MD_FLAG_NOHTML`
equivalent, or comrak's scrub-by-default with `unsafe` only where
MathJax needs it) + a nav policy allowing only fragment jumps with
external links handed to GtkUriLauncher (fw-webview.c:401-462);
Viaduct's img-scheme + CSP rewrite is the alternative for remote
images. Carry Framework's env-var gotchas into packaged-run notes
(WebKitGTK sandbox vs Landlock; `WEBKIT_DISABLE_DMABUF_RENDERER` on
Wayland, main.c:133-164).

Ports [ALT]: live side-by-side preview with the debounce contract
(0.05s small / 0.6s large, map §5 PreviewSlice.h); **>300KB slicing**:
render ±64KB around the caret with 16KB fence lookback so WebKit
never sees the whole file (map §5); in-place content swap between
marker comments preserving scroll (map §5 m:650-666; see the
baseline note above); scroll-to-edit via renderer sourcepos (Phase 6
comrak sourcepos ↔ `PreviewSync` line math, map §5; blocks only:
lists/description-list sourcepos is comrak's known weak spot, crates
§1); themes: user CSS in XDG themes dir, import from file/URL,
`#wrapper` wrap contract (map §5 theming); math (MathJax/KaTeX:
DECISION which; no house evidence either way), mermaid,
highlight.js with light/dark palette picked from background,
wikilinks clickable in preview staying plain in code/math/diagrams
(map §5 preview-js); preview templates with
`{%title%}`/`{%style%}` slots (map §5); save-HTML export (map §5).

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
here: Atrium's title Stack, Label page / Entry page, is the proven
shape, house §Q3); in-list rename-vs-jump Enter pref (delta §1
3.0.28). Chip rendering note (crosswalk §12): upstream uses a runtime
subclass of the attachment cell drawing at 0.45 alpha for partial
tokens; the translation is CSS `opacity` on the chip (Atrium pill
pattern), behavior preserved, mechanism gone. Ports [NV]:
bookmarks/snapback as note+searchString pairs (`BookmarksController`
map §1.7).

DECISION (the Cmd-[0-9] namespace; contract §1C/§2/§4B): upstream
clashes hard: nvALT assigns Cmd-0/1-9 to note bookmarks (with
Shift/Ctrl-Shift escalation, BC:238-270) while nvUltra assigns the
same keys to saved searches; nvALT's own saved-search controller is
dead code, so it has no slots at all. Recommendation: **bookmarks
take Cmd-0/1-9 [ALT]; saved searches take Shift-Cmd-[0-9] over the
Phase 9 store [CP]**. Keymap sources: the complete verified tables
in contract §1 and §2; keep Cmd-R = rename (no refresh-preview
binding: live preview needs none) [CP].

- [ ] tag sidebar + tag-token chips + multi-note apply
- [ ] in-row title/tag editing (Phase 10's DECISION pays here)
- [ ] bookmarks/snapback
- [ ] saved searches on hotkey slots
- [ ] docs carried: spec rows, patchnotes entry

## Phase 18: App: URL scheme + deep linking + single instance

Ports [ALT] (delta §3 route table; contract §3B for link semantics):
`commonplace://find/<terms>` (with UUID reveal semantics),
`/make` (`title`/`txt`/`html`/`tags`/`url`), `/markdownify`,
`/add-theme`, bare-host-as-search; the nvUltra compatibility note
(ultra §1: its `find` is deliberately nvALT-compatible) means
Commonplace also answers `x-nvultra://find` shape-for-shape [CP
courtesy; a second MimeType entry]. Wiki-link grammar to honor
(contract §3B): bracket-pair scan, no newline inside, control-char
guards, a modern equivalent of upstream's ObjC-bracket exclusion
(exclude fenced/code-span lines), case-insensitive prefix matching
with shortest-title preference, completion from titles with the
shortest preselected, extension-tolerant matching [ULTRA]. Link
behaviors: plain click = search, never create (upstream-verified);
Cmd-click = create via make with `txt=[[X]]`; DECISION: upstream
performs no dedup on that create, Commonplace should bind the
missing note or refuse with a message [CP, record at phase open].
Linux mechanics [CP]: `.desktop` x-scheme-handler registration
(brief §3), GTK single-instance activation with the URL routed to
the running window: the entire route is proven end-to-end by
Viaduct (house §Q5: MimeType → HANDLES_OPEN → connect_open with a
window-present retry loop → activation presents, never duplicates;
main.rs:132-181, 304-308).

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
color; the spell-check/smart-punct live-sync ports upstream's single
Cocoa-bindings site via settings-read + changed-signal, crosswalk
§5), Fonts & Colors (body/list fonts, user scheme, editor theme +
preview theme pickers with import, max text width), per-collection
Notes pane (formats/extensions, delete confirmation), Debug pane
[CP: logging toggles + open-log]. Hidden keys from delta §4 carried
as config keys where they still make sense. RISK FLAG (crosswalk
§2): the Bring-to-Front global activation hotkey rides Carbon
PTHotKeys upstream and has no house precedent on Linux; options are
the XDG GlobalShortcuts portal (unproven in house) or dropping the
binding: DECISION at phase open. Editor cursor note (crosswalk §12):
upstream monkey-patches the process-wide I-beam cursor; the
translation is a per-widget `gdk::Cursor` swap on enter/leave +
theme change. The settings fan-out reproduces GlobalPrefs'
selector-keyed registry one-for-one, including `excludingSender:`
echo suppression (crosswalk §7).

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
(`AlienNoteImporter` flow, map §1.10); HTML paste → text, with the
import sanitizer (`sanitizeForeignStylesForImporting`, crosswalk §3)
as the boundary filter and every NSTask shell-out (perl Markdown,
Ruby Textile, RTF conversion; crosswalk §10 surprise #10) replaced
by the pure-Rust pipeline. The readability parse runs off-main with
an upstream defensive invariant carried as a debug_assert
(crosswalk §6: GGReadabilityParser re-queues itself at HIGH priority
when it finds itself on main). One-way Simplenote escape hatch [CP,
per the scoping decision]: import a Simplenote export archive into a
folder per the formats §2 honor-list (no live sync engine ever;
spec §4). Cut [CP]: Safari extension (browser clippers are ecosystem
work; the URL scheme covers automation).

DECISION: HTML → Markdown converter (crates §5: the `readability`
crate is a 3-year dead end; the live picks are **dom_smoothie 0.19**
for article extraction and **htmd 0.5.6** for HTML→Markdown, both
active, both app-crate-side; htmd rides html5ever's rcdom which
self-describes as not production-grade: the flagged risk against the
alternative of a minimal in-core converter porting the upstream
HTMLToMarkdown semantics; recommendation: sanctioned crates,
benchmarked against upstream behavior examples). The Simplenote
archive grammar and the official importer honor-list (content,
creationDate, deleted, markdown, modificationDate, pinned, tags:
everything else dropped) are pinned in formats §2, including the
`systemTags` vocabulary and the `markdown` flag as the exact
md-vs-plaintext marker.

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
tree; crates §4: age 0.12.1 + keyring 4.2 with the dbus/zbus
secret-service stores; gpgme rejected, last release 2022), (c) defer
with an explicit spec note (recommendation: c for 1.0; revisit
post-1.0 with real usage). The gate must be answered even if the
answer is "defer".

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

## Release batching (2026-11-17, decided with Brandon)

Phases land in version bumps by arc, the engine-arc precedent
(v0.2.0 = Phases 1-3) continued to 1.0. The batches are targets, not
promises: a phase may cross a batch boundary when dependency forces
it, and a release cuts when its arc is green, never by calendar.
Patch level (0.x.y) is reserved for within-arc fixes; the flag and
verb surface turns additive at 1.0 (spec §7). The milestone passes
ride the release that completes their arc: M1 ships with v0.5.0, M2
with v0.9.0, M3 is v1.0.0 itself.

| Version | Phases | The story |
|---|---|---|
| v0.1.0 | 0 | the skeleton (catch-up tag, cut with v0.2.0) |
| v0.2.0 | 1-3 | the engine arc: read side, query language, the search machinery (shipped 2026-11-17) |
| v0.3.0 | 4-5 | notes become writable: upstream filename rules, atomic saves, external-change reconcile, then tags/metadata blocks and the link graph (shipped 2026-11-17) |
| v0.4.0 | 6-7 | render boundary + export core: comrak behind the core trait (the first sanctioned dependency, in a frontend crate), export verbs, round-trip proofs; the engine is feature-complete after this (shipped 2026-11-20) |
| v0.5.0 | 8-9 | the CLI arc: verbs, `--json` shapes, exit codes, config, saved searches, completions; the first runnable artifact; **M1 fires here** |
| v0.6.0 | 10-11 | the app rises: GTK4 window, live list over a watched folder, and the search-field state machine, the phase the project exists for |
| v0.7.0 | 12-14 | the editor arc: pane with autosave/undo/scroll memory, markdown highlighting + task lists, lists/fences/pairs + the Linux keymap |
| v0.8.0 | 15-16 | live preview (WebKitGTK, slicing, themes) + collections |
| v0.9.0 | 17-19 | tags UI/bookmarks/row editing, the URL scheme, preferences + theming; **M2 fires here** |
| v0.10.0 | 20-21 | hardening (sync-safety, perf gates into CI) + importers |
| v0.11.0 | 22-23 | the encryption gate answered (recommendation: defer past 1.0) + packaging (desktop, metainfo, Flatpak, `release.yml`) |
| v1.0.0 | 24 | the parity audit and the contract-is-the-promise release; **M3** |

Two flow facts already settled by the engine arc: tags and GitHub
Releases exist since v0.2.0 (the v0.1.0 catch-up tag anchored at the
pre-engine commit), so Phase 23's checklist item is about the install
tree, not the release mechanics; and every release follows the house
flow (docs carried per phase, doc-drift + release-auditor before the
cut, verbatim tag messages).

---

## Decision ledger (running)

| Decision | Phase | Status |
|---|---|---|
| Sync scope: files-first, no cloud; Simplenote importer only | 21 | decided 2026-11-17 with Brandon (spec §4) |
| Tag matching: nvALT prefix default, strict flag | 2 | decided (roadmap) |
| Diacritic folding: on | 2 | decided (roadmap) |
| Trash: `trash` crate in the app crate, engine keeps a delete boundary | 4 | decided (2026-11-17, confirmed per crates §3: crate 5.4 verified freedesktop-compliant; lands with the app crate at Phase 10+, the engine's delete takes the injected sink now) |
| Link rewriting on rename: YES (upstream's hook is an empty stub) | 4 | decided (contract §3B) [CP improvement] |
| `Title:` metadata overrides displayed title, first line is filename source | 5 | decided (2026-11-17, confirmed in spec row 53; the fallback is the row-3 rule, which for a metadata-titled note yields nvALT's literal `Title: Foo`, pinned as the compatibility fixture per formats §1.7) |
| Renderer: comrak behind core trait (1.0.0 on 2026-11-17) | 6 | decided (2026-11-20, crates §1: pinned `=1.0.0`, MSRV 1.89 inside stable CI; alternatives recorded: pulldown-cmark 0.13.6, libMultiMarkdown FFI). Wikilinks un-parsed by the renderer so needs-wiki detection fires (Apex parity); math dollars on. Wiring lands in the CLI crate now (export consumes it); the app crate joins at Phase 15 where its own options (unsafe + sanitize) are decided |
| List widget: GtkListView + SingleSelection (house §Q3; no ColumnView anywhere in house) | 10 | decided (roadmap, house evidence) |
| File watching: notify 8.x + debouncer-full in app crate, reconcile keys on path + re-stat | 10 | recommendation (crates §2), confirm at phase |
| Autosave: on, debounced 250-500 ms, Docket guard discipline | 12 | recommendation (house §Q2), confirm at phase |
| Highlighting: sourceview engine + Quire-style guarded passes, cap enforced before pass | 13 | decided (roadmap, house evidence) |
| Paragraph-move: bind Ctrl-Cmd (ALT) AND Ctrl-Opt (ULTRA alias); headings: nvUltra's ungated Opt-Cmd-]/[ | 14 | decided (contract §4C) |
| Keyboard translation: mac ⌘→Ctrl, ⌥→Alt, ⌃⌘X→Ctrl+Alt+X; collision pass (preview → Ctrl+Alt+P; paragraph-move convergence; line-shift unbound) | 14 | recommendation (roadmap translation layers), confirm at phase |
| Text-index units: `char`s in-engine (caps and slices in chars, not bytes or UTF-16 units); bytes only at disk/comrak boundaries | 1 | decided (Phase 1 confirms: the title cap and its space window count chars, `note::NoteTitle`) |
| Event bus: the 13-name nvALT notification inventory becomes the typed Event enum (2 dead names dropped) | 11 | decided (crosswalk §4) |
| Write coalescing: two independent handles; user edits cancel the write coalescer but never the fsync cadence | 4 | decided (crosswalk §10, upstream comment cited) |
| Undo contracts: undo dirties the file; group nesting during undo; per-note stacks with stored closures | 12 | decided (crosswalk §9, upstream behavior) |
| Modifier-hold popovers: DROP (GTK has no flagsChanged observer; undiscoverable; duplicates visible controls) | 12 | recommendation (crosswalk §12), confirm at phase |
| GlobalPrefs fan-out: typed setting-key subscriptions with excludingSender echo suppression | 19 | decided (crosswalk §7, upstream behavior) |
| Preview baseline: Framework reload-with-anchor-restore; DOM replacement = unproven experiment | 15 | decided (house §Q1 + crates §6 scroll-API constraint) |
| Preview math: MathJax vs KaTeX | 15 | open (no house evidence) |
| Cmd-[0-9] namespace: bookmarks [ALT]; saved searches take Shift-Cmd-[0-9] [CP] | 17 | recommendation (contract §4B), confirm at phase |
| Cmd-click create dedup: bind-or-refuse (upstream duplicates) | 18 | recommendation (contract §3B), confirm at phase |
| HTML→Markdown: dom_smoothie + htmd (app crate side) | 21 | recommendation (crates §5), confirm at phase |
| Encryption | 22 | recommendation: defer past 1.0 (age + keyring named if reopened) |
| Config parsing: hand-written TOML subset in core, no serde | 8 | decided (crosswalk §11, dependency-free contract) |
| Global activation hotkey: XDG GlobalShortcuts portal vs drop | 19 | open risk (crosswalk §2: Carbon PTHotKeys, no house precedent) |
| Faux bold/italic (NSStrokeWidth/NSObliqueness): synthesized Pango tag vs drop to CSS class | 13 | open (crosswalk §8) |
| Editor cursor: per-widget gdk::Cursor swap (upstream monkey-patches the process-wide I-beam) | 19 | decided (crosswalk §12) |
| CLI exit taxonomy reading: 0 success incl. empty reports; 1 trouble (runtime failures + failed resolutions where a target was required); 2 usage. Data rows are not findings (the house "findings" leg is for verdict verbs; the pinned surface has none, so `list` on a full folder exits 0) | 8 | decided (2026-11-20, spec §6) |
| `--sort` sets ascending intent (the upstream reversed default applies only to the unnamed column); `--reverse` flips either | 8 | decided (2026-11-20) |
| Unset saved-search slot: usage error, not the empty query (empty would print every note; nvUltra's empty hotkey slot does nothing) | 9 | decided (2026-11-20) |
| Flags come after the verb (`commonplace-cli list --dir X`); pre-verb flags stay reserved so the parser never guesses verb-vs-flag | 8 | decided (2026-11-20, README documents it) |
