# Interaction-contract research: nvALT keymap + nvUltra shortcuts + wiki-link/navigation semantics

Research note. Produced 2026-11-17 from a direct parse of the nvALT
fork's `MainMenu.xib` (167 menu items, base64 key-equivalents decoded)
plus every `performKeyEquivalent:`/`keyDown:` site in the fork, and
pandoc extraction of the four nvUltra help snapshots. Feeds the keymap
decisions in `roadmap.md` Phases 11, 14, 17, and 18.

Citation shorthand (all under the repo root):
- `XIB` = `.repos/nvalt/en.lproj/MainMenu.xib`; `BMXIB` = `.repos/nvalt/en.lproj/MarkupPreview.xib`
- `LE` = `.repos/nvalt/LinkingEditor.m`; `AC` = `.repos/nvalt/AppController.m`; `NTV` = `.repos/nvalt/NotesTableView.m`; `AI` = `.repos/nvalt/AppController_Importing.m`; `NC` = `.repos/nvalt/NotationController.m`; `BC` = `.repos/nvalt/BookmarksController.m`; `DF` = `.repos/nvalt/DualField.m`; `QST` = `.repos/nvalt/QuickSearchTable.m`; `BT` = `.repos/nvalt/BookmarksTable.m`
- `HELP` = `.repos/nvalt/en.lproj/Excruciatingly Useful Shortcuts.nvhelp` (built-in RTF help doc)
- `LOG` = `.repos/nvalt/CHANGELOG.md`; `U-*` = `research/raw/nvultra-help-*.html` snapshots

Nib modifier convention: the nib stores the letter; an UPPERCASE letter
encodes recorded Shift (`"T"` + command-only = Shift-Cmd-T). The
"Delete" items store base64 0x08 (backspace). Rows marked barMenu exist
identically in the second nib tree (ids 1130-1547) feeding the menu-bar
status menu.

---

## 1. THE COMPLETE nvALT DEFAULT KEYMAP

### 1A. App level

| Key | Action | Source |
|---|---|---|
| Cmd-, | Settings | XIB:47 (id 129); barMenu XIB:1046 |
| Cmd-Q | Quit | XIB:84 (id 136) |
| Cmd-H / Opt-Cmd-H | Hide / Hide Others | XIB:65/70 |
| Shift-Cmd-N | Show Main Window | XIB:668 |
| (user hotkey) | Bring to front, global | PTHotKey `AppActivationKeyCombo`, user-settable, no default (GlobalPrefs.m:426) |

No Cmd-N "new note" exists: creation is the search-field contract
(Return on unmatched text).

### 1B. Note menu

| Key | Action | Source |
|---|---|---|
| Cmd-R | Rename (in-place title edit) | XIB:95 (id 392); HELP |
| Shift-Cmd-T | Tag (token chip editor) | XIB:100 (id 994) |
| Cmd-Backspace | Delete note(s) | XIB:105-111 (id 393); NTV:919-921 |
| Cmd-L | Search or Create (focus field) | XIB:116 (id 554) |
| Shift-Cmd-U | Import URL (Readability via Option) | XIB:129 (id nvU1); LOG:271,301 |
| Opt-Cmd-E | Export | XIB:134 (id 396); LOG:6 (3.0.40 moved it here) |
| Shift-Cmd-R | Show in Finder | XIB:140 (id 1207) |
| Shift-Cmd-P / Cmd-P | Page Setup / Print | XIB:151/156 |

### 1C. Bookmarks (runtime-built menu items; absent from the nib)

| Key | Action | Source |
|---|---|---|
| Shift-Cmd-D | Add to Bookmarks | BC:248-249 |
| Cmd-0 | Show Bookmarks | BC:238-239 |
| Cmd-1...9 | Restore bookmark slot | BC:267-268 |
| Shift-Cmd-1...9 | Slots 10-18 | BC:269 |
| Ctrl-Shift-Cmd-1...9 | Slots 19+ | BC:270 |
| Backspace (in list) | Remove bookmark | BT:29-36 |

In nvALT the Cmd-[0-9] slots are **note bookmarks**, not saved searches.

### 1D. Edit menu + Find

| Key | Action | Source |
|---|---|---|
| Cmd-Z / Shift-Cmd-Z | Undo / Redo (per-note undo manager) | XIB:167/172; NC:887-893 |
| Opt-Cmd-C | Copy URL (UUID-bearing deep link) | XIB:190 (id 1118); NoteObject.m:1117-1133 |
| Shift-Cmd-V | Paste as New Note | XIB:201 (id 1000) |
| Opt-Cmd-V | Paste Markdown Link (URL in clipboard → `[sel](url)`) | XIB:206 (id 1394); LE:2401-2410, 4962-4981; gated by Markdown completion pref |
| Shift-Cmd-L | Insert wiki link (wrap `[[...]]`) | XIB:968 (barMenu id 1382); LE:3162-3164, 3377-3391 |
| Opt-Cmd-K | Insert Markdown Link `[sel]()` | LE:3165-3168, 4922-4925, 4962-4981 |
| Cmd-F | Find bar | XIB:232 (id 1332) |
| Opt-Cmd-F | Find and Replace | XIB:237 (id 1350); gate LE:2387-2399 |
| Cmd-G / Shift-Cmd-G | Find Next / Previous | XIB:243/248 |
| Cmd-E | Use Selection for Find | XIB:253 (id 1335); LOG:6 |
| Opt-Cmd-J | Jump to Selection | XIB:259 (id 1336) |

### 1E. View menu / list navigation

| Key | Action | Source |
|---|---|---|
| Opt-Cmd-L | Switch to Horizontal Layout (toggle) | XIB:388 (id 1115) |
| Shift-Cmd-C | Collapse Notes List | XIB:394 (id 1127) |
| Opt-Cmd-1/2/3 | Color scheme B/W / Low Contrast / User | XIB:403/409/415 |
| Shift-Cmd-K | Show/Hide Word Count | XIB:442 (id 1255; nib anomaly: alternate=YES with no primary) |
| Opt-Cmd-P | Show/Hide Note-Previews in List | XIB:452 (id 1062) |
| Cmd-J / Cmd-K | Next Note / Previous Note | XIB:461/466 (ids 521/522); intercepted pre-menu at NTV:991-1041 |
| Cmd-D | Deselect Note(s) (restores search text) | XIB:474 (id 507); DF:237 |

List key handling (NTV): Return = rename (pref `enterInListRenamesNote`,
default on, LOG:161) else jump to editor (NTV:908-918); Backspace =
delete (919-921); Tab = next key view (922-924); Esc passes through
(925-928); Cmd-Up/Down = first/last row (933-942); Shift-Up/Down =
extend selection (944-968); **any other printable key is forwarded to
the search field: typing while the list has focus starts filtering**
(NTV:976-984). In performKeyEquivalent (995-1041): Cmd-J/K with Opt =
jump to last/first row (1016-1020); Ctrl-J/K work when pref
`UseCtrlForSwitchingNotes` (1000-1022); Ctrl-N/P = next/prev note when
the editor is not first responder (1025-1031); **Ctrl-[ = Escape alias**
(1034-1037).

### 1F. Format menu / editor text style

| Key | Action | Source |
|---|---|---|
| Cmd-T | Plain Text Style (strip styles) | XIB id 977; LE:2422-2439 |
| Cmd-B / Cmd-I | Bold / Italic | XIB:500/513; LE:1871-1878 |
| Cmd-Y | Strikethrough (`~~`) | XIB:527 (id 576) |
| Cmd-[ / Cmd-] | Shift Left / Right (indent/outdent) | XIB:544/549 |
| Ctrl-Cmd-Left/Right | Outdent / Indent (alias) | LE:4926-4932 |

### 1G. Editor bindings that exist only in code (the 3.0.7 batch + neighbors)

All require the editor first responder; LE performKeyEquivalent 1846-1927
+ handleMarkdownCommandShortcut 4914-4948 + handleMarkdownFunctionShortcut
4950-4960; batch intro LOG:413-433.

| Key | Action | Notes |
|---|---|---|
| Opt-Cmd-L | Cycle list marker - → * → + → 1. → none | LE:1863-1866, 3899; gated by auto-list-formatting pref |
| Opt-Cmd-T | Toggle task `- [ ]`/`- [x]` | LE:1867-1870, 3133-3136 |
| Ctrl-Cmd-Up/Down | Move paragraph/list item up/down | LE:4933-4939, 5430/5434 |
| Opt-Cmd-Up/Down | Expand/shrink selection by Markdown structure | LE:4940-4946, 5671/5693 |
| F6 | Insert reference link from selection, or jump to its `[id]:` definition | LE:4955-4956 (keyCode 97) |
| F7 | Insert footnote / jump marker↔definition | LE:4957-4958 (keyCode 98) |
| Cmd-Return | Open link under caret; else newline at end of paragraph | LE:1879-1896 |
| Cmd-Shift-Return | Insert newline above | LE:1888-1891 |
| Cmd-Backspace (editor) | Delete to beginning of line | LE:1898-1902 |
| Return | Smart newline: continue bullets/numbers/quotes/fences/tasks; empty item exits | LE:4983+ |
| Tab / Shift-Tab | Indent/outdent list items + quotes; inside `]`/past `]` completes `[id](http:// )`; else soft-tab or key-view hop | LE:1982-2044 / 2046-2078; Opt-Tab always indents |
| Cmd-Shift-> / Cmd-Shift-< | Quote line / remove quote | LE:1909-1911 / 1905-1907; gated by `UsesMarkdownCompletions` |
| Cmd-Shift-= / Cmd-Shift-- | Raise / lower ATX heading level | LE:1913-1921; gated by `UsesMarkdownCompletions` |
| Cmd-Left at col 0 | Jump from body to title rename (Cmd-Right reverse) | LE:1947-1969 |
| Esc | Clear current search (relay) | LE:1381-1382 → AC:1428-1453; LOG:370 |

### 1H. Preview menu + preview window

| Key | Action | Source |
|---|---|---|
| Ctrl-Cmd-P | Toggle Preview Window | XIB:588 (id 1244) |
| Ctrl-Cmd-M | Preview using Marked | XIB:594 (id 1540) |
| Cmd-U | Toggle Source View | XIB:606 (id 1240) |
| (none since 3.0.7) | Lock Note to Preview | XIB id 1293; the preview toolbar lock button still carries Opt-Cmd-L (BMXIB:149-150), colliding with cycle-list-marker when preview is key: an upstream bug pattern to avoid |
| Shift-Opt-Cmd-P | Print Preview / PDF | XIB:617 (id 1292) |
| Shift-Cmd-S | Save Preview HTML | XIB:623 (id 1239) |
| Shift-Cmd-M | Toggle Markdown Completion | XIB:634 (id 1385) |

### 1I. Window menu

Cmd-M minimize (XIB:650); Cmd-W close (XIB:655; the quit-when-closing
pref can turn this into quit); Shift-Cmd-N show main window (XIB:668).

### 1J. The search-field state machine (DualField: the defining keys)

| Key | Behavior | Source |
|---|---|---|
| Cmd-L | Focus field | XIB:116; DF:41 |
| Return | Note selected → edit it; nothing selected → create note titled with field text, focus body | AC:2121-2126 → 2149-2171; DF:246,253 |
| Up/Down/Shift-Up/Down | Forwarded verbatim to the list; field text kept selected | AC:1465-1481 |
| Cmd-Up/Down | First/last row | AC:1501-1508 |
| Home/End | Beginning/end of field-as-document | AC:1510-1531 |
| Tab | Empty: eaten; unmatched + candidate: re-run search to trigger completion; else focus body | AC:1483-1500 |
| Ctrl-U | Delete to beginning of line (bash-style) | AC:1538-1548 |
| Cmd-Backspace | deleteToMark remapped to deleteWordBackward | AC:1534-1537 |
| Esc | Clear search + selection, refilter, refocus field | AC:1428-1453; DF:148 |
| Cmd-D | Deselect (restore typed search) | XIB id 507; DF:237 |
| Snapback button | Return to previous search+note | DF:303-338, 376-382; pushed on every find route AI:177-179 |

### 1K. Auxiliary windows

Tag field editor: Return/Tab append `,` and move on (comma-chaining,
AC:1558-1569). Saved-searches quick table: Return renames, Backspace
removes (QST:29-44): but the controller is dead code (delta §3), so
nvALT 3.0.40 effectively has **no saved-search slots**; the Cmd-[0-9]
namespace belongs to note bookmarks. Sync wait panel: Return = Quit Now
(XIB:830-831).

**Pref-gated summary:** `UsesMarkdownCompletions` gates Cmd-Shift->/<,
Cmd-Shift-=/-, and the Tab link tricks; `enterInListRenamesNote` gates
Return-in-list; `UseCtrlForSwitchingNotes` gates Ctrl-J/K;
editor-first-responder gates table 1G; list-first-responder gates the
J/K interception. No shortcut differs between layouts except key-view
order and the in-row title/tags Tab swap (NTV:1093-1112).

---

## 2. THE nvULTRA SHORTCUT TABLE (from the local snapshot)

Source: `research/raw/nvultra-help-keyboard-shortcuts.html`.

| Key | Action | Notes |
|---|---|---|
| Cmd-, / Cmd-Q / Cmd-H / Opt-Cmd-H | app shell | Opt-Cmd-Q = Quit and Close All |
| Cmd-N / Cmd-O | New Folder / Open Folder | notebook model |
| Cmd-L | Search/Create | same contract as nvALT |
| Cmd-W / Opt-Cmd-W | Close / Close All | |
| Shift-Cmd-P / Cmd-P | Page Setup / Print | |
| Cmd-Z / Shift-Cmd-Z / X/C/V | standard | Cmd-Z after drag-drop strips link syntax to bare path first |
| Shift-Cmd-C / Opt-Cmd-C | Copy As HTML / Copy As LaTeX | export |
| Opt-Shift-Cmd-V | Paste and Match Style | |
| Opt-Cmd-[ / Opt-Cmd-] | Decrease / Increase ATX Header Level | always-on (vs nvALT's gated Cmd-Shift-=/-) |
| Opt-Cmd-Up/Down | Increase / Decrease Selection | word→sentence→paragraph, honors pairs |
| Ctrl-Opt-Up/Down | Move to Previous/Next Paragraph | paragraph-move keys |
| Ctrl-Opt-Cmd-Up/Down | Move to Previous/Next Header | ATX + Setext |
| Cmd-[ / Cmd-] | Shift Left / Right | |
| Ctrl-Cmd-Up/Down | Shift Up / Down (move lines) | nvALT uses these for paragraph-move instead |
| Cmd-F / Opt-Cmd-F / Cmd-G / Shift-Cmd-G / Cmd-E | find family | identical to nvALT |
| Cmd-J | Jump to Selection | nvALT hijacks Cmd-J for list nav |
| Cmd-B / Cmd-I | Bold / Italic | |
| Ctrl-Cmd-L | Toggle List Type | nvALT analog: Opt-Cmd-L cycle |
| Ctrl-Opt-Cmd-A / R / P / N | CriticMarkup accept / reject / previous / next change | |
| Ctrl-Opt-Cmd-X | Track Changes toggle | |
| Ctrl-Cmd-T / Shift-Cmd-A / Cmd-= / Cmd- | Typewriter / Auto Zoom / Zoom In / Out | view |
| Shift-Cmd-L / Opt-Shift-Cmd-L / Opt-Cmd-I | Line Numbers / Paragraph Numbers / Invisible Characters | |
| Ctrl-Cmd-F / Cmd-M / Opt-Cmd-M | Full Screen / Minimize / Minimize All | |
| Shift-Cmd-F | Toggle File List | nvALT analog: Shift-Cmd-C collapse |
| Shift-Cmd-I | Toggle Info | |
| Shift-Cmd-O | Edit File(s) (external editor) | |
| Ctrl-Cmd-P / Cmd-R | Toggle Preview / Refresh Preview | nvALT: Ctrl-Cmd-P same; Cmd-R free (nvALT uses it for rename) |
| Cmd-/ | MultiMarkdown Syntax Guide | |
| Cmd-[0-9] | Saved Searches | order-based slots (vs nvALT's note bookmarks) |

Additional snapshot keys: Ctrl-Cmd-N = Folder Navigator; Opt-Esc / F5 =
autocomplete trigger (Esc deliberately does NOT autocomplete); Cmd-V
with selection = paste clipboard URL as Markdown link.

---

## 3. WIKI-LINK + DOCUMENT-NAVIGATION SEMANTICS

### 3A. nvUltra rules (snapshot-cited)

| Rule | Statement | Source |
|---|---|---|
| Syntax | `[[Note Title]]`; clickable in editor AND preview | U-FEAT |
| Matching | Title "can include a file extension"; "will find the closest match if enough of the title is correct": fuzzy/partial matching | U-FEAT |
| Toggle | Preferences▸Editing "Use WikiLinks" | U-EDIT |
| Completion | while typing; hidden pref `NVSuppressWikiLinkCompletionExtensions` | U-ADV |
| Subfolders | display as `Subfolder name/Note name.md`; Tab-completes folder names in the search field; `x-nvultra://make` accepts slashes to create into subfolders | U-ADV |
| Drag-drop | creates a wiki link OR `{{transclusion}}` per a preference; Cmd-Z once strips extra syntax | U-FEAT |
| Nonexistent-note click | not explicitly documented; `x-nvultra://make` is the creation path; "if no title is provided, created using the first line of the text" | U-ADV |
| Folder Navigator | Ctrl-Cmd-N, type-to-filter history of notebook folders | U-FEAT |
| Saved searches | magnifier dropdown in the bar; first 10 get Cmd-1-9/0 by order | U-SS |
| Backlinks | **none**; the nearest surface is Connections mode (content-similarity ranking across all notes; images by filename+tags, PDFs by contents) | U-FEAT |
| In-document | emacs Ctrl-A/E; Ctrl-Opt-Up/Down paragraph moves respecting lists/indented code; Ctrl-Opt-Cmd-Up/Down header jumps | U-DOCNAV |

### 3B. nvALT implementation (code-cited)

**Link detection**: `AttributedPlainText.m:252-289`: scan pairs `[[`
with `]]`; the interior gets an NSLinkAttributeName of
`nvalt://find/<percent-escaped-title>`: a wiki link is stored as a
SEARCH URL, not a note pointer. Rules: interior may not begin/end with
control chars (:266, 278); a newline inside kills the link (:272-276);
ObjC-looking or unbalanced-bracket blocks are excluded (:291-304); the
preview makes `[[...]]` clickable via wikilinks.js (3.0.22), staying
plain in code/math/diagrams.

**Clicking (editor)**: `LE.clickedOnLink:atIndex:` 2455-2484:
- Plain click on `[[Title]]` → the find route: pushes the current note
  to snapback, sets the search string to the title (AI:173-212);
  resolution falls to the search engine: first note whose title has the
  whole query as prefix, shortest "prefix-parent" title winning
  (NC:1505-1523, 837-868). **If no note matches, NOTHING is created**:
  the field holds the terms and the standard search-or-create
  affordance applies.
- Cmd-click rewrites the URL on the fly to
  `nvalt://make/?title=X&txt=%5B%5BX%5D%5D` (LE:2469-2480, literal
  `[[%@]]` at 2473): the only click path that creates a missing note.
- **Duplicate-title ambiguity: `make` performs no deduplication**
  (`addNewNote:` always adds, NC:872-899): Cmd-clicking a link whose
  title already exists duplicates the note. Plain click is the
  disambiguated path. True disambiguation only via UUID-bearing links
  from Opt-Cmd-C Copy URL (NoteObject.m:1117-1133).
- **Rename does NOT rewrite wiki links**: `updateLinksToNote:fromOldName:`
  is an EMPTY STUB in both trees (NC:833-835, body is just a comment;
  called from NoteObject.m:834). This corrects the one-line claim in
  nv-source-map §4 ("+ link rewriting"): the hook exists, the
  implementation does not. Old `[[Old Title]]` links keep working only
  as searches for the old title.

**Case sensitivity**: matching case-insensitive by construction;
diacritics not folded; no subfolder paths (flat title matching).

**Completion**: typing inside `[[` completes note titles
(AC:2104-2118) with the SHORTEST matching title preselected
(NC:1505-1523); range bounded to the `[[`...no-`]]` span on the line
(LE:2515-2549); suppressed while undoing/deleting (2494-2513).

**Saved-search slots**: dead code (above). **Backlinks/connections**:
none in nvALT. **Folder Navigator**: none (single library).

---

## 4. KEYMAP COMPARISON AND RECOMMENDATION SHAPE

### 4A. Where nvALT and nvUltra agree (adopt as-is)

Cmd-L Search/Create + Return-creates + Esc-clears; Cmd-,/Q/H/Opt-H/W/M;
Ctrl-Cmd-F; Cmd-Z/Shift-Z/X/C/V/A; Cmd-B/I; the find family
(Cmd-F/Opt-F/G/Shift-G/E, nvALT's Jump-to-Selection is Opt-Cmd-J);
Opt-Cmd-Up/Down selection expand/shrink (semantics differ: Markdown
structure vs word/sentence/paragraph); Ctrl-Cmd-P toggle preview;
Cmd-[/] shift; print family; Cmd-E.

### 4B. Where they diverge (decision surface)

| Function | nvALT | nvUltra | Conflict? |
|---|---|---|---|
| Next/prev note | Cmd-J/K (+Ctrl aliases pref-gated) | none | no |
| Jump to Selection | Opt-Cmd-J | Cmd-J | yes |
| Rename | Cmd-R | none (Cmd-R = Refresh Preview) | yes |
| List type | Opt-Cmd-L cycle | Ctrl-Cmd-L toggle | near-miss |
| Heading level | Cmd-Shift-=/- (pref-gated) | Opt-Cmd-]/[ (always) | different keys, same intent |
| Paragraph move | Ctrl-Cmd-Up/Down | Ctrl-Opt-Up/Down (their Ctrl-Cmd-Up/Down = shift lines) | yes |
| Slots Cmd-[0-9] | note bookmarks | saved searches | semantic clash |
| New note | none (search-field contract) | Cmd-N = New Folder | open if collections land |
| Task toggle | Opt-Cmd-T | none bound | no |
| CriticMarkup | auto-pair only | full accept/reject/navigate family | nvUltra-only |
| Wiki-link click-to-create | Cmd-click synthesizes make | undocumented; make handler | nvALT is the port base |
| Editor Esc | clears search | ends editing | nvALT is the identity |

### 4C. Recommendation shape for the roadmap

**Phase 11**: adopt the full 1J table verbatim (Cmd-L, Return=create-
or-edit, arrows forwarding with text-kept-selected, Cmd-Up/Down, Tab
semantics, Ctrl-U, Esc-clear, Cmd-D, snapback); keep nvALT's
typing-in-list-filters (NTV:976-984), an NV signature nvUltra lacks.

**Phase 14**: keep the 3.0.7 batch exactly; paragraph-move: bind BOTH
Ctrl-Cmd-Up/Down (primary, [ALT]) and Ctrl-Opt-Up/Down (alias,
[ULTRA]); heading keys: prefer nvUltra's always-on Opt-Cmd-]/[ over
nvALT's pref-gated Cmd-Shift-=/-; quote keys Cmd-Shift->/< ungated
from markdown-completion; F6/F7 unmapped.

**Phase 17**: Cmd-R rename, Shift-Cmd-T tag, Cmd-Backspace delete,
Shift-Cmd-D bookmark, Return-in-list rename pref, Tab column swap,
Cmd-J/K with Opt jump-ends and Ctrl-pref aliases. Cmd-[0-9] namespace:
recommend bookmarks follow nvALT (Cmd-0 + 1-9 with Shift/Ctrl-Shift
escalation); saved searches, when they land, take Shift-Cmd-[0-9]
[CP decision to record]. Keep Cmd-R=rename; do NOT add
refresh-preview on Cmd-R (live preview needs no refresh) [CP].

**Phase 18**: port find/make + the Cmd-click make synthesis + the
no-UUID search-link semantics, honoring: non-creation on plain click,
no dedup on Cmd-click (decide: Commonplace should dedup or refuse:
[CP decision]), and the empty updateLinksToNote: stub (decide:
Commonplace SHOULD rewrite links on rename: [CP decision] improving on
both ancestors). Wiki-link grammar: bracket-pair scan, no newline
inside, control-char guards, modern equivalent of the ObjC-bracket
exclusion = exclude fenced/code-span lines, case-insensitive prefix
matching with shortest-title preference, completion from titles with
shortest preselected, extension-tolerant matching [ULTRA].
Subfolder display + Tab-completion into subfolders ride Phase 16
[ULTRA]; Connections mode and Folder Navigator are non-goals for 1.0
[CP decision to record].

**Cross-phase guardrails**: single Shift-encoding convention for
keymaps; reserve (don't assign) Cmd-N, Cmd-O, Opt-Cmd-W until Phase 16
decides the collections model; one shortcut, one owner (the BMXIB
stray Opt-Cmd-L lesson).
