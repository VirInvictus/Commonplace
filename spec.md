# Commonplace: the contract

This is the source of truth for behavior. Before changing semantics,
change this file first. The parity tables in section 5 are filled from
the research corpus (`research/`) and cite it; when the code and this
file disagree, this file wins and the code gets fixed.

## 1. Purpose

Commonplace is a Linux port of the Notational Velocity line: the
original NV by Zachary Schneirov, Brett Terpstra's nvALT fork, and the
public surface of the closed-source nvUltra. It exists to put that
line's defining interaction (one field that finds notes and becomes
them) on Linux, over plain files, without a database, without an
account, and without giving up keyboard-first speed. It is built as an
ode to the bloggers and their commonplace books.

## 2. Definitions

- **note**: one plain-text file in the notes folder, titled by its
  first line.
- **notes folder**: the directory a collection watches. No metadata
  store is required inside it.
- **collection**: one notes folder plus its settings (nvUltra's term;
  NV called it "the notes folder").
- **index**: the in-memory derived structure used for search. Always
  rebuildable from the folder alone.
- **search-as-create**: the field behavior where a query that matches
  nothing becomes a new note's title.

## 3. Core invariants

1. **Notes are files.** Creating, editing, renaming, and deleting a
   note is creating, editing, renaming, and deleting a file. No
   database is required for operation; any cache the engine keeps is
   derived and safe to delete.
2. **Search-first.** One field drives find-and-create. Typing filters
   the list incrementally; a no-match query is a new note title, not
   an error.
3. **Atomic writes.** The engine never leaves a half-written note on
   disk. Writes are atomic-rename based.
4. **External changes are first-class.** Files changed outside the app
   are picked up (file watching plus scan), never clobbered, and never
   locked. The folder must stay syncthing-safe: no lock files, no
   sidecars that other tools must respect.
5. **Local-first.** Fully functional offline. No cloud sync in scope
   (files stay syncthing-friendly instead; a Simplenote importer may
   exist as a one-way converter, never as a sync engine).
6. **Engine/GUI split.** `commonplace-core` is GUI-free and
   dependency-free and tests headless. The app crate owns the whole
   GTK stack. The CLI talks to the same engine.
7. **Keyboard-first.** Every core flow (search, open, create, tag,
   preview, export) must be reachable without the mouse, in the NV
   tradition.

## 4. Non-goals

- Cloud sync engines (Simplenote two-way sync is out of scope per the
  2026-11-17 scoping decision; importers are allowed).
- RTF as an authoring format. NV supported RTF notes; Commonplace is a
  plain-text/Markdown app. Reading notes from an existing RTF folder is
  a migration concern, not a runtime format (conversion is one-way via
  a textutil-equivalent pass if it happens at all; decision logged at
  the storage phase).
- A plugin system. nvALT's customization surface (templates, CSS) is
  ported as configuration, not as a plugin API.
- Library encryption, at 1.0. NV shipped transparent library
  encryption, but invariant 1 (notes are plain files) rules out an
  encrypted container as the default; any encryption design is a
  gated post-1.0 decision (roadmap Phase 22).

## 5. Parity program

Parity is tracked per feature against three tiers, each tagged:

- **[NV]**: original Notational Velocity semantics (the port base).
- **[ALT]**: nvALT additions and changes (primary port base tree).
- **[ULTRA]**: nvUltra behaviors known from public docs only (facts;
  nvUltra source is closed and is never decompiled or redistributed).
- **[CP]**: Commonplace's own additions or deliberate divergences.

The table below is filled from `research/nv-source-map.md`,
`research/nvalt-delta.md`, `research/nvultra-feature-survey.md`, and
`research/linux-ecosystem-brief.md`, and every row carries a phase
number from `roadmap.md`.

| # | Feature | Tier | Phase | Notes/citation |
|---|---------|------|-------|----------------|
| 1 | Notes as individual plain-text files | NV, ALT | 1, 4 | NV shipped file-per-note as a format option but defaulted to a compressed, optionally-encrypted single DB (as did nvALT 1.x/2.x); nvALT 3.0.1 flipped the default to plain-text `.md` files (commit c050f15; brief, map §4/§8) |
| 2 | Plain-text extensions `md txt text utf8 taskpaper`, `md` default | ALT | 1, 4 | map §4 (`NotationPrefs.m:252-271`) |
| 3 | First-line title, 60-char cap, break-at-space, "Untitled Note" | NV | 1 | map §4 (`NSString_NV.m:265-328`) |
| 4 | Filename rules: `:`→`-`, leading `.`→`_`, 255 cap, `title.N.ext` collisions | ALT | 4 | map §4 (`NotationFileManager.m:412-456`) |
| 5 | Atomic writes (temp+swap) | ALT | 4 | `FSExchangeObjects` → `renameat2(RENAME_EXCHANGE)`; map §7 |
| 6 | External-change reconciliation | ALT | 4, 10, 20 | map §4; inotify at 10 |
| 7 | Delete to trash | NV | 4 | map §4; freedesktop Trash [CP shape] |
| 8 | Query: implicit AND, no operators | NV | 2 | map §2 tokenization |
| 9 | Query: quoted exact phrases | NV, ULTRA | 2 | map §2; ultra search-syntax |
| 10 | Query: boolean AND/OR/NOT, `&&`/`||`/`!`, parens | ULTRA | 2 | ultra search-syntax (manual snapshot verified) |
| 11 | Query: `#tag` prefix matching | ALT | 2 | delta §1 (`TagSearch`); ULTRA strict full-name mode is the flag |
| 12 | Case-insensitive matching | NV, ULTRA | 2, 3 | map §2 lowercase caches |
| 13 | Diacritic folding | ULTRA | 2 | ultra search-syntax; deliberate NV divergence |
| 14 | No inverted index; per-note string caches | NV | 3 | map §2; deliberate design preserved |
| 15 | Incremental prefix-resume filtering | NV | 3, 11 | map §2 PHASE 1 + `useCachedPositions` |
| 16 | Stable column sort, title tiebreak, no relevance score | NV | 3 | map §2 (`sortAndRedisplayNotes`) |
| 17 | Title-prefix autocomplete drives selection; prefix parents | NV | 3, 11 | map §2 PHASE 4 |
| 18 | Search/create single field; no-match becomes new note | NV | 11 | map §3 (`createNoteIfNecessary`) |
| 19 | Field state machine (Return/Escape/Tab/arrows/launch restore) | NV, ALT | 11 | map §3, every behavior cited |
| 20 | Field keeps typed text on selection | ALT | 11 | delta §1 (3.0.39) |
| 21 | Snapback stack | NV | 11, 17 | map §1.7 |
| 22 | Tag sidebar + tag tokens + multi-note apply | ALT | 17 | delta §1 (3.0.13/14); map §1.6 |
| 23 | MMD metadata blocks as primary tag store | ULTRA | 5 | ultra tagging-and-metadata |
| 24 | Inline `#tags` in body count as tags | ALT | 5 | delta §1 |
| 25 | `[[wiki links]]` + rename rewriting | ALT, ULTRA, CP | 5, 14 | ultra advanced-features; upstream's `updateLinksToNote:` hook is an empty stub (contract §3B: rename never rewrote links in NV or nvALT), so rewriting on rename is Commonplace's improvement [CP, decided] |
| 26 | `{{transclusion}}` | ULTRA | 5, 15 | ultra advanced-features; expanded pre-render [CP] |
| 27 | Derived backlink index | CP | 5 | brief §2 gap |
| 28 | Markdown editor highlighting with 300KB cap | ALT | 13 | delta §6 (`MarkdownHighlight.h:6`) |
| 29 | Task lists (`- [ ]` full behavior set) | ALT | 13 | delta §6 (`TaskList.h`) |
| 30 | List continuation + 0.05s renumbering + marker cycling | ALT | 14 | delta §6; map §1.4 |
| 31 | Fence continuation + bounded-close (return-hang fix) | ALT | 14 | map §1.4 (m:4985) |
| 32 | Auto-pairing with backtick fence rules | ALT | 14 | delta §6 (`NVAutoPair`) |
| 33 | Reference-link completion, F6/F7 | ALT | 14 | delta §6 |
| 34 | Paragraph move + expand/shrink selection | ALT | 14 | delta §6 (3.0.7 batch) |
| 35 | Per-note undo stacks | NV | 12 | map §7 note 8 |
| 36 | Per-note scroll/caret restore | ALT | 12 | delta §1 (3.0.40) |
| 37 | Word count, blank past 300KB | ALT | 12 | delta §1 (3.0.38) |
| 38 | Live preview, debounce 0.05s/0.6s | ALT | 15 | map §5 (`PreviewSlice.h`) |
| 39 | Preview slicing >300KB (±64KB caret window) | ALT | 15 | map §5 |
| 40 | In-place content swap + scroll-to-edit | ALT | 15 | map §5 (m:650-666, `PreviewSync`) |
| 41 | Preview themes (CSS), editor themes (`.theme`) | ALT | 15, 19 | map §5 theming; format-compat goal |
| 42 | MathJax/Mermaid/highlight.js injection + wikilinks in preview | ALT | 15 | map §5 preview-js; KaTeX is the Phase 15 gate |
| 43 | Preview templates `{%title%}/{%style%}` | ALT | 15 | map §5 |
| 44 | Export: multi-note, HTML + text (RTF/DOC cut, rationale) | NV, ALT | 7 | map §1.5; spec §4 |
| 45 | Import: URL + Readability + markdownify | ALT | 21 | delta §1 (3.0.15/16) |
| 46 | Simplenote: one-way importer only, no sync | CP | 21 | scoping decision 2026-11-17; upstream kept live sync (map §6) |
| 47 | URL scheme find/make/markdownify/add-theme | ALT | 18 | delta §3 route table; `x-nvultra://find` courtesy shape [CP] |
| 48 | Saved searches on hotkey slots | ULTRA | 9, 17 | ultra §1; shared CLI/app store [CP] |
| 49 | Collections: unlimited folders, subfolders flat, slash titles | ULTRA | 16 | ultra §1 |
| 50 | Preferences panes parity (General/Notes/Editing/Fonts/Debug) | ALT | 19 | delta §4 inventory |
| 51 | Library encryption | NV | 22 | gated; recommendation defer past 1.0 (spec §4) |
| 52 | Flatpak + desktop + metainfo + scheme handler | CP | 23 | brief §3 baseline; house packaging |
| 53 | `Title:` metadata overrides the displayed title; first line stays the filename source | CP | 5 | formats §1: nvALT has zero `Title:` awareness, nvUltra undocumented; the override rule is Commonplace's pinned decision |
| 54 | Encoding read path: UTF-8 always, leading BOM stripped, invalid UTF-8 never lossily replaced | CP | 1 | recorded divergence (roadmap translation layer 4): upstream resolves per-file encodings via the `com.apple.TextEncoding` xattr, BOM, then heuristics, and writes the xattr back (`NoteObject.m:1554,1369-1381`); Linux has no analog, so Commonplace pins UTF-8. The decoder is `NoteBody::from_utf8`; undecodable files surface as per-file warnings at the store level (row 56) |

## 6. Machine surfaces

- `commonplace-cli` verbs document their exit codes (house exit-taxonomy
  style: 0 success/empty, 1 findings-or-trouble, 2 usage error). Pinned
  when the CLI phase lands.
- `--json` output shapes are additive from their first release.

## 7. Versioning

Single-source `VERSION` at the repo root; the workspace `Cargo.toml`
mirrors it. Patchnotes entries are `## vX.Y.Z (YYYY-MM-DD)`, newest
first. The flag/verb surface is additive from 1.0 (the bindery-cli
stability rule).
