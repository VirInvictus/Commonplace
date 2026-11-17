# Commonplace port map: Notational Velocity / nvALT 3.x

Research note. Produced 2026-11-17 from a read-only code survey of the
two reference clones.

**Trees** (both read-only reference clones, gitignored):
- Base (original): `.repos/nv`, scrod/nv, HEAD `5ea5eb8` "bump version
  to 2.5.2". This tree carries scrod's *post-fork* GPLv3 development
  (new segment-storage redesign: `NVStorage/` Swift package,
  `NVRootedDirectory.m`, `docs/`, `CONTEXT.md`), so its non-shared
  files are NOT part of the port base; its shared files are the common
  ancestor.
- Port base (primary): `.repos/nvalt`, ttscoff/nv, HEAD `6a4c000`
  "nvALT 3.0.40 (169)" (2026-11-13). `Info.plist:148` = 3.0.40,
  macOS 12+.

Paths below are relative to those two roots (`nvalt/...` vs `nv/...`).
Line counts are `wc -l` of the `.m` (implementation) file. "Shared" =
present in both trees with common lineage; "fork-only" / "nv-only" =
the diffs (verified by filename comm).

Scale: nvalt is roughly 53,300 lines of ObjC/Swift across root sources
plus subdirs; nv is roughly 34,100. nvALT 3.x feature vocabulary lives
in `nvalt/CHANGELOG.md` (3.0.1 → 3.0.40).

**Licensing consequence for the port:** Commonplace is GPL-3.0 (see
`spec.md`/`AGENTS.md`), which makes both trees usable as sources.
Provenance note regardless of license: shared-ancestor files exist in
divergent forms in the two trees; take them from `nvalt/` (the fork's
form, which is the lineage every user-visible feature actually ships
from). The nv tree's post-2011 additions (`NVRootedDirectory`,
`StartupRecovery`, `NVStorage`, `docs/`) may be read as behavior
reference.

---

## 1. CLASS INVENTORY by subsystem

### 1.1 Note model / storage

| Class/File | Origin | Lines | Responsibility + key methods |
|---|---|---|---|
| `NoteObject.h/.m` | shared | 251/1,935 | THE note model: title, filename, attributed body, UUID, dates, labels, search caches, file I/O. Key: `updateFromData:inFormat:` m:1536 (decode file bytes: plaintext encoding-guess, RTF, HTML); `writeUsingCurrentFileFormat` m:~1230-1330 (format, encode, atomic write, xattr encoding, Finder tags, dates); `setTitleString:` m:752; `_setTitleString:` m:794 (rebuilds `cTitle` search cache); `setFilename:withExternalTrigger:` m:810 (filename/title sync, link updating); `updateWithSyncBody:andTitle:` m:1601; `exportNoteAsFormat:toDirectory:` m:~1700-1772; `nextRangeForWords:` m:1816 (find bar). C-string caches `cTitle/cContents/cLabels` built m:433-434, 633-656, 801, 997. |
| `NotationController.h/.m` | shared | 222/1,738 | Owns `allNotes` + filtered list; the query engine (section 2); list sorting; note lifecycle (`addNewNote:`, `_addDeletedNote:`); FSEvents/FNNotify wiring m:73-74, 742; delegates to `NotationDirectoryManager`/`NotationFileManager` categories. |
| `NotationFileManager.h/.m` | fork-only (split out of NotationController) | 710 | Category on NotationController: all file-system ops. `uniqueFilenameForTitle:fromNote:` m:412-456 (sanitization/collisions); atomic write `storeDataAtomicallyInNotesDirectory:...` m:544-611 (temp file + `FSExchangeObjects`); `moveFileToTrash:` m:647-708; disk-UUID identity m:193-225; `relocateNotesDirectory` m:335. |
| `NotationDirectoryManager.h/.m` | shared | 669 | Directory scan/reconciliation: FSEvents stream callback m:86-109 (root-change → reconfigure), bulk `FSGetCatalogInfo` catalog scans, external-change detection (`synchronizeNotesFromDirectory`), notes-directory trash check. |
| `WALController.h/.m` | shared | 573 | Write-ahead log ("Interim Note-Changes" file) for the single-database format; coalesces DB writes. |
| `FrozenNotation.h/.m` | shared | 228 | Serializes the legacy "Notes & Settings" single-DB plist archive. |
| `TemporaryFileCache(.h/.m)` / `TemporaryFileCachePreparer.h/.m` | cache fork-only / preparer shared | /277 | Prepares + caches note files for external editing/preview. |
| `DeletedNoteObject.h/.m`, `DeletionManager.h/.m` | shared | 289/280 | Tombstones for deleted notes (UUID + sync keys), undo-of-delete support; `DeletionManager.m:202` sorts tombstones. |
| nv-only storage line | nv-only | — | `nv/NVRootedDirectory.m` (2,184), `nv/StartupRecovery.m` (588), `nv/ImportingUtils.m`, `nv/NVStorage/` (Swift pkg): scrod's 2.x immutable-segment redesign. Not in the fork; read as design reference only. |

### 1.2 Note-list + search UI

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `NotesTableView.h/.m` | shared | 1,759 | The list itself: keyboard handling (`keyDown:` m:902-983, `performKeyEquivalent:` m:995-1041 with ctrl-[ = escape), row selection movement m:1043-1056, in-row title/tag editing (tab swaps title/tags columns in horizontal layout m:1093-1112), sort-function table m:104-105, preview-drawing rows. |
| `FastListDataSource.h/.m` | shared | 144 | C-array "list" the filter/sort machinery mutates in place: `filterArrayUsingFunction:context:` m:97-117, `sortStableUsingFunction:` (mergesort) m:119-122, `fillArrayFromArray:` m:75. |
| `NoteAttributeColumn.h/.m` | shared | ~100 | Column identity + `sortFunction`/`reverseSortFunction` per column (title/labels/dateModified/dateCreated). |
| `UnifiedCell.h/.m` | shared | 280 | Single-cell row renderer (title + preview + label chips). |
| `LabelColumnCell`, `NotesTableHeaderCell`, `NotesTableCornerView`, `BTTableHeaderCell` | header cells fork-only (LabelColumnCell shared) | ~120-217 | Row/column chrome; flat 3.0.1 headers. |
| `NoteLayout.h/.m`, `SearchFieldLayout.h`, `TagEditingManager.h/.m` | all fork-only | 336 + headers + 721 | 3.x layout engine (vertical/horizontal), search-field-in-titlebar layout, tag-token chips in rows + multi-note tag panel (3.0.14). |
| `WordCountToken.h/.m` | fork-only | 173 | Toolbar word-count token field. |
| `EmptyView.h/.m`, `HeaderViewWIthMenu.h/.m`, `TitlebarButton.h/.m`, `StatusItemView.h/.m`, `FullscreenWindow.h/.m` | EmptyView/Header/Titlebar shared; StatusItem/Fullscreen fork-only | 100-244 | Empty-state view, section headers, titlebar buttons (share/sync/view-source), menu-bar extra, fullscreen window. |
| `DualField.h/.m` | shared | 497 | The search/URL dual-mode field cell + snapback button; escape relay m:148. |
| `ETScrollView/ETClipView/ETContentView/ETNoteScrollView`, `ETTransparentButton(.Cell)`, `CustomTextFieldCell`, `DFView` | all fork-only | ~40-131 each | ElasticThreads scroll-view family + button cells (historical nvALT chrome). |
| `LinearDividerShader`, `NSBezierPath_NV`, `NSString_CustomTruncation`, `NSCollection_utils`, `Spaces.c/h` | shared (Spaces fork-only) | 116-248 | Drawing/geometry/collection/truncation utilities. |

### 1.3 Search/indexing internals

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `NotationController.m` filter methods | shared | — | section 2: `filterNotesFromString:` m:1339, `filterNotesFromUTF8String:forceUncached:` m:1358-1499, `refilterNotes` m:1351, `filterNotesFromLabelAt(Index)(Set):` m:1321-1337. |
| `NoteObject.m` matching | shared | — | `noteContainsUTF8String` m:1850-1871 (strstr over lowercase caches), `noteMatchesTagWord` m:1842, `noteTitleHasPrefixOfUTF8String` m:1873, `noteTitleIsAPrefixOfOtherNoteTitle` m:1876, `resetFoundPtrsForNote` m:1836, `prefixParentsOfNote` m:264. |
| `TagSearch.h/.m` | fork-only | 139 | `#tag` query word parsing/matching (section 2): `NVSearchWordIsTagPrefix` m:27, `NVQueryContainsTagWord` m:42, `NVQueryNewlyHasTagWord` m:62, `NVNoteMatchesTagWord` m:120, `NVTagTitlesMatchPrefix` m:126, `NVStringByRemovingTagWords` m:66. |
| `SearchCommand.h/.m` | shared | 19 | AppleScript `search` command: NSScriptCommand → `searchForString:` m:12-17. |
| `MultiTextFinder.h/.m` + `NSTextFinder.h`/`NSTextFinder_LastFind.m` | fork-only | 211/170 | In-editor find bar (pre-10.7 fallback + NSTextFinder integration). |
| `QuickSearchTable.h/.m` | fork-only | 55 | NSTableView subclass for the quick-search popup. |
| `SavedSearchesController.m` | fork-only | 582 | Saved searches menu/persistence (dead code per the delta survey). |
| `SearchHighlight.h` | fork-only | header | Query-for-highlighting rules (body-focused vs typing state). |

### 1.4 Text editor

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `LinkingEditor.h/.m` | shared, massively extended in fork (1,610 → 5,916) | 5,916 | NSTextView subclass = the editor: markdown syntax highlighting (per-edit line ranges m:1726-1743, limit m:1280), ==highlight== spans (engine `MarkdownHighlight.h:54-130`), CriticMarkup coloring m:1519, task checkboxes (`nv_toggleTaskCheckboxForEvent:` m:2263, `toggleTaskMarker:` m:1868), numbered-list renumbering m:3594-3721, list/quote/fence continuation, invisible-character highlighting m:4907, auto-pairing, wiki `[[link]]` completion, automatic selection/scroll bookkeeping (`setAutomaticallySelectedRange:`, `firstVisibleCharIndexWasAutomatic:` h:62-66). |
| `LinkingEditor_Indentation.h/.m` | shared | 208 | Indentation/tab-width behavior of the editor. |
| `MarkdownHighlight.h` | fork-only | 332 (header-only inline fns) | Shared scanning primitives: `NVMarkdownSyntaxHighlightLimit = 300*1024` m:6, ==span scanner m:54, list-marker/indented-code detectors m:20-52. |
| `CriticMarkup.h` | fork-only | header | `{++ins++} {--del--} {~~sub~>} {==hl==} {>>comment<<}` span scanner m:56. |
| `TaskList.h` | fork-only | 240 | `- [ ]` task line parser/mutator. |
| `ListEnter/ListIndent/ListOutdent/ListSelection/ListSplit.h`, `SelectionRedraw.h`, `ClickBelowText.h`, `NoteLayout.h` | all fork-only | headers | 3.0.x list-editing + selection helper categories on LinkingEditor. |
| `NVAutoPair.h/.m` | fork-only | ~90 | Bracket/quote auto-pairing (3.0.22 backtick rules). |
| `LinkReferenceCompletion.h/.m` | fork-only | 307 | `[text][id]` reference-id completion (3.0.21/22); consumed from AppController.m:2104-2118. |
| `MarkdownButtonLayout.h` | fork-only | header | Bold/italic/list marker preference wiring. |
| `AttributedPlainText.h/.m` | shared | 544 | NSAttributedString ↔ plain text; `syntheticTitleAndSeparatorWithContext:` caller m:103; foreign-style sanitizer. |

### 1.5 Preview / export

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `PreviewController.h/.m` | fork-only | 1,437 | Preview window (legacy `WebView`): render pipeline m:615-741 (section 5), processor selector m:743-755, in-place content swap m:650-666, scroll-state save/restore m:41-121 JS, sticky preview m:799, save-HTML m:1040-1058, Marked handoff m:1073, share sheet. |
| `PreviewRender.h/.m` | fork-only | 238 | `NVPreviewHTMLPreparingContent` etc.: needs-math/mermaid/wiki detection, renderer `<script>` assembly. |
| `PreviewSlice.h` | fork-only | 155 | Large-note slicing: `NVPreviewAroundCaretCharacters = 64*1024`, fence lookback 16KB, update delays 0.05s/0.6s. |
| `PreviewSync.h`, `PreviewTruncation.h/.m` | fork-only | headers/143 | Scroll-to-edit math (`NVSourceLineForIndex`), preview truncation markers. |
| `NVApexMarkdown.swift` | fork-only | 26 | Apex bridge: `html(from:)` = `apexHTML(mode: .unified)`; source-positions variant. |
| `ThirdParty/apex/` | fork-only (vendored lib) | — | Full Apex Markdown renderer source (C + Swift, CMake/SPM). |
| `preview-js/` | fork-only assets | — | `highlight.min.js` + light/dark GitHub palettes, `mathjax/`, `mermaid.min.js`, `wikilinks.js`. |
| `NSString_MultiMarkdown.h/.m` | fork-only | ~130 | `stringWithProcessedMultiMarkdown:` m:113 (→ Apex); legacy `tp2md.rb` Ruby NSTask for TaskPaper m:20-60. |
| `NSString-Markdown.h/.m` + `Markdown_1.0.1/Markdown.pl` | fork-only | ~60 | Legacy Perl `Markdown.pl` via `/usr/bin/perl` NSTask m:10-30; only still used by saveHTML in "Markdown" mode (PreviewController.m:1047-1048). |
| `NSString_Textile.h/.m` + `Textile_2.12/` | fork-only | ~60 | Textile preview via Ruby script NSTask (NSString_Textile.m:17). |
| `NVTheme.h/.m` | fork-only | 466 | MultiMarkdown Composer `.theme` (JSON) parsing → editor style roles (`NVThemeStyleForeground` etc., NVTheme.h:15+). |
| `NVThemeManager.h/.m` | fork-only | 423 | Themes in `App Support/Themes`; editor+preview theme selection; Marked CSS import; `nvalt://add-theme` (header doc lines 3-8); `Paper.theme` sample. |
| `ExporterManager.h/.m` | shared | 175 | Export sheet: `exportNotes:forWindow:` m:25, per-format via `NoteObject exportNoteAsFormat:`. |
| `NVRTFConverter.h/.m` | fork-only | 251 | RTF→Markdown "Convert to Markdown" library migration (3.0.1). |
| `template.html`, `templateclean.html`, `custom.css`, `customclean.css` | fork-only assets | — | Preview templates with `{%support%}/{%title%}/{%content%}/{%style%}` slots. |

### 1.6 Tags / labels

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `LabelsListController.h/.m` + `LabelObject.h/.m` | shared | 232/120 | Tag sidebar ("labels"): consumed at NotationController.m:1322, tag-list filtering. |
| `TagEditingManager.h/.m` | shared, rewritten in fork | 721 | Tag editing: `NVTagToken` chips, multi-note tag panel, comma-chaining on Return (AppController.m:1559-1569). |
| `LabelEditor.h` | fork-only | header | Tag field editor in list rows. |
| Tag persistence | — | — | nvALT 3.0.1+: Finder tags via `[fileMan setTags:atFSPath:]` (NSFileManager_NV.m:123; NoteObject.m:1304); legacy OpenMeta xattr read path NSFileManager_NV.m:86-157. |

### 1.7 Bookmarks

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `BookmarksController.h/.m` | shared | 573 | Note bookmarks (history) + snapback: `NoteBookmark` (h:24-41) note+searchString pairs; driven by `[field pushFollowedLink:]` (AppController_Importing.m:178) and `setSnapbackString:` (AppController.m:1708,1838). |
| `BookmarksTable.h/.m` | shared | ~40 | Bookmarks list table. |

### 1.8 Simplenote sync (details in section 6)

`SimplenoteSession.h/.m` (shared, fork-modernized; 137/1,307),
`SimplenoteEntryCollector.h/.m` (494), `SyncSessionController.h/.m`
(441), `SyncResponseFetcher.h/.m` (248),
`NotationSyncServiceManager.h/.m` (481), and the protocol headers
(`SyncServiceSessionProtocol.h`, `SynchronizedNoteProtocol.h`,
`SynchronizedNoteMixIns.h`).

### 1.9 Preferences

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `GlobalPrefs.h/.m` | shared | 236/1,271 | App-level prefs (layout, fonts, list font 3.0.6, behaviors) via accessor methods. |
| `NotationPrefs.h/.m` | shared | 162/1,123 | Per-library prefs (inside "Notes & Settings"): storage format, extension sets m:252-302, 918-1120; sync accounts + keychain m:327-364. |
| `PrefsWindowController.h/.m` | shared, fork-redesigned | 1,213 | Settings window (3.0.3 sidebar panes; Debug pane 3.0.37). |
| `NotationPrefsViewController.h/.m` | shared | 896 | Notes-settings pane (storage format, extensions, Simplenote tab, Convert to Markdown). |

### 1.10 Import / export / URL schemes

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `AlienNoteImporter.h/.m` | shared, fork-extended | 763 | Import files/URLs: dialog walk m:53, background URL import m:54 (readability override), .blor/Help import m:51, Stickies via `StickiesDocument.h/.m` (shared). |
| `AppController_Importing.h/.m` | shared, fork-extended | 431 | Pasteboard/URL import: `addNotesFromPasteboard:`, `interpretNVURL:` m:161-296 (all URL verbs), markdownify m:396-429, file-drop → `[[wikilink]]`/`![]()` m:298-348. |
| `HTMLToMarkdownConverter.h/.m` | fork-only | 1,115 | ObjC HTML2Text port (used pre-3.0.16 alongside Swift). |
| `GGReadabilityParser.h/.m` | fork-only | 569 | Readability article extraction (options enum h:9+). |
| `NVMarkdownifier.swift` | fork-only | 138 | HTML→Markdown via HTML2Text+Readability Swift libs (3.0.16 "built-in Markdownifier"). |
| `URLGetter.h/.m` | shared | 203 | URL download helper. |
| `tp2md.rb` | fork-only | script | TaskPaper→Markdown Ruby helper. |
| `SafariExtension/` | fork-only | Swift | Safari share extension feeding `nvalt://make` (3.0.16). |

### 1.11 External editor handoff

| Class/File | Origin | Lines | Responsibility |
|---|---|---|---|
| `ODBEditor/ODBEditor.h/.m` (+ suite headers) | shared | ~1,000 | ODBEditor protocol AppleEvent suite: `editNote:inEditor:context:` → NoteObject.m:1774; callbacks `odbEditor:didModifyFile:` m:1793 re-read + dirty, `didClosefile:` m:1810. |
| `ExternalEditorListController.h/.m` | shared | 451 | Editor picker UI/prefs. |
| `TemporaryFileCachePreparer.h/.m` | shared | 277 | Stages note files for external editors. |
| nv-only `ExternalEditingSession/` | nv-only | — | scrod's newer external-edit session API (not in the fork). |

### 1.12 Security / encryption family (shared)

`NSData_transformations.h/.m` (456; crypto transforms),
`KeyDerivationManager` (129) + `KeyDerivationDelaySlider`,
`PassphraseRetriever/Picker/Changer`, `SecureTextEntryManager` (181),
`NVPasswordGenerator`, `BlorPasswordRetriever` (285), C helpers
`broken_md5.c`, `idea_ossl.c`, `pbkdf2.c` (fork-only), `hmacsha1.c`
(fork-only), `CRC32.c` (fork-only), `BufferUtils.c`, `dsa_pub.pem`
(Sparkle). nvALT 3.0.1 switched to system CommonCrypto; `library/openssl`
remains vendored for build compat.

### 1.13 Other notable

`main.m` (shared); `AppController.h/.m` shared but 2,313 → 4,148 lines
in the fork (delegate hub: section 3, menus, layout, URL events
m:180/569, wiki-link open, preview mode, share);
`AppController_Preview.h/.m` + `AppController_Markdown.m` fork-only
(preview/markdown menu wiring); `nvALT-Swift.h` bridging; `NVLog.h/.m`
+ Debug pane (3.0.37); `InvocationRecorder` (111, sync coalescing);
`MultiplePageView` (300, print pagination); `Sparkle.framework`;
`Notation.sdef` (AppleScript dictionary: search/find/make note);
`hashcash/`, `JSON/`, `readability/` vendored C/JS libs (fork-only).

---

## 2. DEEP DIVE: the search/query engine

**Grammar as implemented** (NotationController.m:1358-1499 +
NoteObject.m + TagSearch.m):

- **Tokenization** m:1396: `strsep` over the lowercased query with
  separators `" :\t\r\n"`: space, colon, tab, CR, LF. **No operators**:
  no `-negation`, no `AND/OR`, no wildcards, no fuzzy. A term is a raw
  substring needle.
- **Quoted phrases**: if the query contains a `"` (m:1396
  `strchr(searchString, '"')`), the separator set becomes `"` alone,
  so text between quotes is one token with spaces/colons preserved.
  Quoted phrase search is a side effect of re-tokenizing. (TagSearch.m:14-19
  mirrors this: `NVSearchSeparatorSet`.)
- **Implicit AND**: each token further compacts the same array
  (`filterArrayUsingFunction:` per token, m:1406-1438): every term
  must appear somewhere in the note.
- **Case**: insensitive by construction. Query lowercased at entry
  (`filterNotesFromString:` m:1342); every note keeps pre-lowercased
  UTF-8 C-string caches of title/labels/contents (NoteObject.m:433-434,
  633-656, 801, 997, with a `copyLowercaseASCIIString` fast path m:648).
- **Diacritics**: NOT folded. Match is byte-wise `strstr` over the
  UTF-8 (lowercased only). An accented character matches only itself.
- **Fields matched**: title OR body OR labels, per note (m:1861-1870).
  No field qualifiers, no per-field weighting at match time; ranking
  is positional/ordering only (below).
- **Tag words** (nvALT 3.0.14+): any token of `#xyz` with length ≥ 2
  after trim is a tag word (TagSearch.m:27-40). It matches if any tag
  title *prefixes* `xyz` case/diacritic-insensitively (m:126-139) OR
  the literal `#xyz` substring appears in title/body/labels (m:120-124,
  so a `#tag` written in the note text matches too, the 3.0.19 fix).
  A query *newly containing* a tag word forces a full refilter from
  `allNotes` (m:1374, `NVQueryNewlyHasTagWord` TagSearch.m:62).
- **Index**: there is **no inverted index**. The "index" is those
  per-note lowercase `strdup` caches, invalidated per edit
  (`contentCacheNeedsUpdate`, rebuilt in
  `updateContentCacheCStringIfNecessary`), plus resume-pointers
  `cTitleFoundPtr/cContentsFoundPtr/cLabelsFoundPtr` (m:1836-1840).
- **Incremental-as-you-type** (the famous trick): PHASE 1 m:1374-1388.
  If the previous filter string is a byte-prefix of the new one
  (`strncmp(currentFilterStr, searchString, oldLen)`) and not
  forceUncached and no new tag word, the *existing filtered array* is
  the candidate set; only the newly typed tokens are scanned
  (`lastWordInFilterStr` offsets, `strsep` resume m:1405). Within a
  re-scanned token, `useCachedPositions` (m:1427) resumes `strstr`
  from the note's `FoundPtr` (m:1861-1868) instead of rescanning from
  the start. Deleting characters (query no longer has the old prefix)
  re-inits from `allNotes`.
- **Result ordering / scoring**: there is no relevance score. The
  visible list is the filtered array sorted by the active sort column,
  which is always a **stable two-pass mergesort: primary = chosen
  column function, secondary = title-alphabetical**
  (`sortAndRedisplayNotes` m:1588-1616; `compareTitleString` applied
  first m:1598 then the column fn m:1600, so ties break by title).
  The reverse pref flips both. Column functions enumerated at
  NotesTableView.m:104-105 (title, labels, dateModified, dateCreated).
- **Title-vs-body weighting** exists only as *selection preference*,
  not ordering: PHASE 4 m:1460-1491. With `autoCompleteSearches` on
  and no tag words, the first note whose **title** has the whole query
  as prefix becomes `selectedNoteIndex` (m:1469), and a "prefix
  parent" (a shorter title that is also a prefix of both,
  alpha-sorted first, m:1471-1487) wins selection instead. Body
  matches never outrank title matches for selection.
- Entry points: typing (`controlTextDidChange:` AppController.m:1704),
  label clicks (m:1321-1337), refilter after edits/sync
  (`refilterNotes` m:1351), AppleScript (SearchCommand.m:15), URL
  `find` (AppController_Importing.m:182).

---

## 3. DEEP DIVE: the search-field state machine

Controller methods in AppController.m unless noted:

- **Typing filters incrementally**: `controlTextDidChange:`
  m:1695-1760. Filters (m:1704); with autocomplete on and exactly one
  title-prefix match, the note is selected+displayed (m:1716-1721) and
  the **remainder of its title is inserted into the field with the
  completion selected** (m:1725-1741). Empty query → deselect
  everything and clear the body (m:1747-1755).
- **Nothing matches → typed text becomes a new note's title**: two
  creation triggers. (a) Return: `fieldAction:` m:2121-2126 →
  `createNoteIfNecessary` m:2149-2171: if `currentNote == nil`
  (nothing selected), creates a NoteObject titled with the field
  string (or localized "Untitled Note" when empty, m:2159) with the
  editor's current body, `addNewNote:` m:2164, then focus moves to the
  body editor (m:2124). (b) Typing directly in the empty body:
  `textDidBeginEditing:` m:2054-2057 calls the same creator.
- **Selecting a note**: `processChangedSelectionForTable:` m:1823-1869.
  Displays contents (`displayContentsForNoteAtIndex:` m:~1940-2028:
  restores text, per-note scroll position, re-highlights search terms,
  scroll-to-nearest-match logic m:1993-2018); caches the *typed*
  string; 3.0.39 behavior: the field keeps what you typed
  (m:1849-1861), title shown in the window title bar otherwise
  (m:1863); snapback stack gets the typed string (m:1837-1838).
- **Escape**: `cancelOperation:` m:1428-1453. Clears field + editor +
  `typedString`, deselects, refilters to all notes, focuses the field,
  hides the clear button. In-editor escape first hides the find bar
  (m:1431-1434). `Control-[` is an alias (NotesTableView.m:1034).
- **Up/Down arrows in the field**: forwarded verbatim to the notes
  table (`doCommandBySelector:` m:1465-1481), keeping the field's text
  selected (m:1475-1478). Cmd-up/down → first/last row (m:1501-1508).
  Home/End map to beginning/end of the field-as-document
  (m:1510-1531).
- **Tab**: m:1483-1500. Empty field: eat it; nothing selected +
  autocomplete candidate available: re-run the search to trigger
  completion (m:1488-1491); otherwise focus the body editor without
  eating the tab (m:1496-1499). In the list's in-row editors,
  Tab/Backtab move between title and tags columns in horizontal layout
  (NotesTableView.m:1093-1112).
- **Return in the list/table editors**: `insertNewline:` on the
  table's field editor jumps to the body editor (m:1552-1557); 3.0.28
  adds an "Enter in list renames note" pref (default on; off = jump to
  editor).
- **Selection mechanics**: `tableViewSelectionIsChanging:` m:1762-1802
  (multi-select only with Shift/Cmd held: drag-select is gated),
  `tableViewSelectionDidChange:` m:1810-1821.
- **Launch restore**: `restoreListStateUsingPreferences` m:2174-2190.
  Last search string, last-selected note by UUID bytes, list scroll
  offset, body top char index.
- **Reveal**: `revealNote:options:` m:2192-2229. Clears the search if
  the note isn't visible, then selects/scrolls/orders-front/edits per
  options.
- **Pinned notes: do not exist** in either tree (verified by grep; the
  only "pinned" hits are titlebar-button layout code,
  AppController.m:2839-2850). Ordering is purely the column sort;
  there is no pinning concept to port.

---

## 4. DEEP DIVE: note storage rules

- **First-line-as-title**: applied at *creation/import/sync only*,
  never as a live body→title binding.
  `syntheticTitleAndSeparatorWithContext:...maxTitleLen:`
  NSString_NV.m:265-328: skip leading whitespace (m:282), take up to
  the first `\n \r \t` or Unicode line/paragraph separator
  (m:275-276, 298-299); if longer than the cap (60 chars; 36 in some
  import paths, e.g. AppController_Importing.m:143) break at the last
  space within the final 10 chars (m:301-313); empty → "Untitled Note"
  (m:320). Runtime renames happen only via the list's title column
  (FastListDataSource.m:124-131) or `renameNote:` (AppController.m:872).
  An externally renamed file re-derives the title from the filename
  minus extension (NoteObject.m:826).
- **Filename derivation** (`uniqueFilenameForTitle:fromNote:`
  NotationFileManager.m:412-456): title → replace `:` with `-` (m:418)
  and leading `.` with `_` (m:419); truncate so title+numbering+
  extension fits 255 UTF-16 chars
  (`filenameExpectingAdditionalCharCount:` NSString_NV.m:223-229,
  called with `3 + extLen + 2` m:431); extension = the library's
  chosen extension for the note's format, but a note's *existing*
  extension is kept if still allowed (m:423-428); collisions are
  case-insensitive (`caseInsensitiveCompare` m:444) against every
  note's basename and resolved with numeric suffixes `title.N.ext`
  (m:447-449, loop m:434-453). Title→filename flow: `setTitleString:`
  m:752-792 → `setFilenameFromTitle` m:806 → rename on disk m:817 with
  revert-on-failure (m:820-823) + link rewriting
  (`updateLinksToNote:fromOldName:` m:834).
- **Extensions per format** (NotationPrefs.m:252-271): PlainText =
  `md, txt, text, utf8, taskpaper` (default `md`, force-inserted at
  index 0 by `ensureMarkdownPathExtension` m:273-302, the 3.0.1
  change); RTF = `rtf`; HTML = `html, htm`; WordDoc = `doc`; WordXML =
  `docx`. Users can add arbitrary safe extensions
  (`stringAsSafePathExtension` strips `./*: \t\n\r`, NSString_NV.m:219-221;
  adder m:~1053). Directory scan admission: extension OR legacy Mac
  OSType match; dot-files, the `Notes & Settings` DB file, and the
  `Interim Note-Changes` WAL are skipped (m:~1093-1120
  catalogEntryAllowed).
- **Where notes live**: user-chosen notes directory (default
  `~/Library/Application Support/Notational Data`,
  NotationFileManager.m:394-409); three library formats: single legacy
  DB ("Notes & Settings"), or one file per note in
  plaintext/RTF/HTML/Word formats. nvALT 3.0.1 made *plain text files
  with `md`* the default for new libraries. The directory is watched
  by FSEvents (NotationDirectoryManager.m:86-109) + FNNotify fallback
  (NotationController.m:73-74, 742); synced on app deactivation
  (AppController.m:1406-1411).
- **Encodings**: per-file `fileEncoding`; read via
  `newShortLivedStringFromData:ofGuessedEncoding:withPath:orWithFSRef:`
  (NoteObject.m:1554). Encoding resolved from the file's
  `com.apple.TextEncoding` xattr, BOM, then heuristics; manual
  re-interpret panel = EncodingsManager (h:26-40). Write: encode in
  `fileEncoding` with lossless intent, **auto-promote to UTF-8** on
  failure (m:1706-1710), then record the encoding back via the xattr
  (`setTextEncodingAttribute` m:1369-1381). Library-wide
  `upgradeEncodingToUTF8` m:1389-1417.
- **Line endings**: no enforced normalization on read/write. The body
  is an NSAttributedString; newlines are whatever the file/sync
  carried. (No CRLF-conversion code exists; the only newline-specific
  logic is in scanners/parsers.)
- **Writes are atomic**: temp file with a random name in the notes dir
  → `FSExchangeObjects` swap (or emulation on volumes lacking it) →
  delete temp (NotationFileManager.m:544-611); file creation+modification
  dates are forced to the note's logical dates (NoteObject.m:1332-1367);
  extension hidden in Finder (m:1307); Finder tags written on every
  save (m:1304).
- **UUID bookkeeping**: each note has `uniqueNoteIDBytes` (CFUUIDBytes,
  NoteObject.h:72): used for URL deep links (`NV=` base64 param,
  AppController_Importing.m:191-194), last-selected restore
  (AppController.m:2183), deleted-note tombstones, and sync identity.
  Per-disk tracking uses a volume UUID (HFS finder-info / FSEvents /
  synthetic MD5 of creation date; NotationFileManager.m:193-225) to
  distinguish "file changed elsewhere" from our own writes.
- **Trash**: deletion moves the file to the volume's Trash (m:647-708,
  with duplicate-name renaming); `DeletedNoteObject` tombstones
  preserve UUID+sync keys for undo and remote deletion propagation;
  notes-dir-in-trash detection m:277-282.

---

## 5. DEEP DIVE: nvALT 3.x preview / export / editor pipeline

**MultiMarkdown**: no bundled `multimarkdown` binary anymore. 3.0.1
replaced it with **Apex**, a vendored Markdown renderer
(`ThirdParty/apex/`, full C/Swift source). Bridge:
`NVApexMarkdown.swift` (`html(from:)` = `apexHTML(mode: .unified)`;
source-position variant for scroll-sync). Called through
`NSString_MultiMarkdown.m:79-80,113-118`. Preview modes map
(PreviewController.m:743-755): `Markdown` and `MultiMarkdown` both →
`stringWithProcessedMultiMarkdown:`; `Textile` → the Ruby
`Textile_2.12` script via NSTask (NSString_Textile.m:17). The Perl
`Markdown_1.0.1/Markdown.pl` NSTask (NSString_Markdown.m:10-30)
survives only in the legacy save-as-HTML path (m:1047-1048).
`tp2md.rb` handles TaskPaper→Markdown (NSString_MultiMarkdown.m:20-60).

**Render pipeline** (PreviewController.m:615-741): debounce by note
size (0.05s small / 0.6s large, PreviewSlice.h); notes > 300KB are
sliced to ±64KB around the caret with 16KB code-fence lookback
(m:620-622); HTML from Apex → `NVPreviewHTMLPreparingContent` flags
needsMath/needsMermaid/needsWiki (m:639) → if the same note and no new
renderer is needed, swap only the content between the
`<!--nv-content-start/end-->` marker comments via JS, preserving
scroll (m:650-666); else full `loadHTMLString:` with lazily injected
`preview-js` assets: highlight.js (GitHub light/dark palette chosen
from the background, 3.0.38), MathJax (3.0.20), Mermaid (3.0.20),
`wikilinks.js` (`[[Note title]]` → opens the note; stays plain in
code/math/diagrams, 3.0.22) (m:693-714). Scroll-to-edit sync uses Apex
`data-sourcepos` attributes (source-line math in PreviewSync.h
`NVSourceLineForIndex`; stripped for display
`NVHTMLWithoutSourcePositions`).

**Theming**: editor themes = MultiMarkdown Composer `.theme` JSON/plist
(NVTheme.h/.m, style-role bitmask NVTheme.h:15+); preview themes =
plain CSS (Marked-compatible; content wrapped in `#wrapper`,
m:723-726); both live in `App Support/Themes`, importable from
files/links (`nvalt://add-theme?url=` / `?path=`, NVThemeManager.h doc
header + AppController.m:578); `Paper.theme` sample; first-run
`custom.css` + `template.html` copied into App Support
(`createCustomFiles` m:757-791) with
`{%support%}/{%title%}/{%content%}/{%style%}` slots (m:728-731).

**Editor**: syntax highlighting is custom, incremental (per-edit line
ranges LinkingEditor.m:1726-1743), off past 300KB (MarkdownHighlight.h:6).
3.0.38 also skips list renumbering, code-fence closing, and word count
past that size; ==highlight== spans (scanner MarkdownHighlight.h:54-130)
and CriticMarkup (CriticMarkup.h, colored + struck/underlined
substitutions m:1519); task lists: `- [ ]` parsing (TaskList.h),
Option-Cmd-T toggle (m:1868,3133), click-in-bracket toggling (m:2263),
done-item strikethrough, Return continuation; numbered-list
auto-renumber debounced 0.05s (m:3594-3721); bullets/quotes/fence
Return continuation, Tab/Shift-Tab indent rules (3.0.40: Tab at end of
note indents the blank line); link-reference completion
(LinkReferenceCompletion.m, F6/F7 jumps); invisible-character
highlighting (m:4907); auto-pairs incl. the 3.0.22 backtick rules
(NVAutoPair.m).

**Export**: per-note via ExporterManager (format popup) →
`NoteObject exportNoteAsFormat:toDirectory:` m:~1700-1772
(plaintext/RTF/HTML/doc/docx writers, exported Finder tags + dates,
`:`→`/` filename fix m:1740); whole-note HTML export via the preview
save panel (m:1040-1058); "Open in Marked" / `x-marked://stream/`
handoff (m:1073+; NoteObject.m:1778 `previewUsingMarked`); Share
button sends selection-or-note; "Share Preview" sends rendered rich
text (3.0.34). **Import-side pipeline**: URLs → Readability +
HTML2Text → Markdown (NVMarkdownifier.swift; the older ObjC twins
HTMLToMarkdownConverter.m 1,115 lines and GGReadabilityParser.m 569
remain); HTML pasteboard → attributed string; images/attachments are
NOT stored: attachments are stripped on import (`removeAttachments`,
AppController_Importing.m:264,374) and dropped image files become
markdown image links `![](file://...)` (m:323-345).

---

## 6. Simplenote sync

Present in both trees; **survives in nvALT 3.x** (removed in 3.0.1,
restored 3.0.4/3.0.5 with new auth; changelog lines 464, 477, 491).

- `SimplenoteSession.h/.m` (1,307): the Simperium REST v1 client for
  app id `chalk-bump-f49` (Simplenote). Index/changes endpoints
  `https://api.simperium.com/1/chalk-bump-f49/Note/index` and
  `/Note/changes` with the `X-Simperium-Token` header (m:76-82,
  350-367), mark-based pagination (`indexMark`, `indexEntryBuffer`),
  change-version tracking (`lastCV`), per-note push queueing/coalescing
  (`schedulePushForNote:` h:91, `pushSyncServiceChanges` h:92,
  suppression set), content-hash merge matching
  (`_invertedContentHashesOfNotes:` h:110, title re-derivation on
  merge m:721,878). **Auth**: the legacy `auth.simperium.com` password
  login is dead (m:336-338 error string "Simplenote no longer accepts
  a password..."); login is via emailed codes:
  `https://app.simplenote.com/account/request-login` and
  `/complete-login` (m:85-93), the resulting sync token stored
  keychain-side with a `sn-token:` prefix (m:48-50, 106-109).
- `SimplenoteEntryCollector.h/.m` (494): streams one fetched remote
  entry into a local note (add/update/merge paths).
- `SyncSessionController.h/.m` (441): service registry + timers +
  sleep/idle power assertions, status menu, schedules pushes for all
  initialized sessions.
- `SyncResponseFetcher.h/.m` (248): thin async HTTP fetcher used by
  the session.
- `NotationSyncServiceManager` (481): settings UI/tab management for
  services.
- Protocols: `SyncServiceSessionProtocol.h` (service interface),
  `SynchronizedNoteProtocol.h` + `SynchronizedNoteMixIns.h` (per-note
  sync key/modified-date storage mixed into NoteObject).
- Note format on the wire: title + separator + body re-synthesized
  with `syntheticTitleAndSeparatorWithContext:...maxTitleLen:60`
  (SimplenoteSession.m:721,878); system tags/pinned handled as remote
  metadata (`remoteEntryWasMarkedDeleted:` etc., SimplenoteSession.h:85-89).

Port note: Simperium v1 REST is a simple HTTPS JSON API
(index/changes/push); a Rust port could reimplement it directly,
including the newer `app.simplenote.com` code-login flow. (Out of
Commonplace scope per the 2026-11-17 files-first decision; recorded
here because it documents the sync architecture if that decision ever
reopens.)

---

## 7. PORTABILITY NOTES (Cocoa-isms → GTK4/Rust)

1. **NSTextView editing stack** (`LinkingEditor`): the editor's whole
   feature set hangs off NSTextView delegate hooks:
   `shouldChangeTextInRange:replacementString:` (undo coalescing, list
   continuation), did-change-text-style incremental highlighting
   (m:1726-1743 assumes edits are contiguous ranges), typing
   attributes, field editors. GTK equivalent is GtkSourceView; the
   port must rebuild "edit happened in range R" semantics on
   `GtkTextBuffer::insert-text`/`delete-range`, and the automatic
   selection/scroll bookkeeping (`setAutomaticallySelectedRange:`,
   `firstVisibleCharIndexWasAutomatic:` LinkingEditor.h:62-66) has no
   GTK analog: Commonplace must decide its own rules for "was this
   scroll user-intended".
2. **NSAttributedString everywhere** (NoteObject body, previews,
   RTF/HTML/doc import/export): the model stores rich text even for
   plain notes. Decide early: keep notes as plain `String` with the
   editor layer owning styling (matches GTK4/SourceView better), and
   port `AttributedPlainText`'s sanitizer role as a boundary filter.
   RTF/DOC/DOCX/HTML read+write (NoteObject.m:1702-1727) maps to
   external crates or gets cut from phase 1.
3. **Legacy WebView preview** (PreviewController): `loadHTMLString:` +
   injected JS. GTK: WebKitGTK (already in the house stack via
   Viaduct/Framework). The marker-comment in-place swap (m:693) and
   scroll-state JS translate directly.
4. **Carbon FSRef/FSCatalogInfo filesystem API**:
   NotationFileManager/NotationDirectoryManager are written against
   FSRef, `FSExchangeObjects`, `FSCreateFileUnicode`, HFS catalog
   info, volume UUIDs. Linux equivalents: `renameat2(RENAME_EXCHANGE)`
   for atomic swap (or link+rename fallback), `inotify`/`fanotify`
   replacing FSEvents, statx for dates, no xattr-encoding analog:
   pick an explicit encoding strategy instead (UTF-8 always + BOM
   sniff on read is the sane default).
5. **Finder tags / OpenMeta**: tags ride macOS Finder tags
   (`setTags:atFSPath:` NSFileManager_NV.m:123). The Linux port needs
   its own sidecar (a `tags:` xattr or a small index), since no
   desktop-standard tag store exists; the OpenMeta read path is
   macOS-history only.
6. **NSTableView cell editing + custom cells** (NotesTableView,
   UnifiedCell, NoteAttributeColumn): in-row title/tag editing with
   Tab moving columns (NotesTableView.m:1093-1112) and preview-drawing
   rows. GTK4 has no in-cell editing like NSTableView's field editors;
   GtkColumnView + a popover/inline GtkEntry, or a custom list widget,
   must recreate "click title to rename in place".
7. **Key-value observing / bindings / notifications**: the codebase is
   largely explicit delegate/notifications, not Cocoa bindings (good
   for porting), but leans on `NSNotificationCenter` strings
   ("ModTimersShouldReset", "TextFindContextShould..."): map to a
   typed event bus.
8. **NSUndoManager per note** (NoteObject.m:1911-1918): undo is
   per-note with window-level fallback (AppController.m:2128-2147).
   Plan per-note undo stacks in the Rust editor layer.
9. **AppleEvent / AppleScript / sdef** (`Notation.sdef`, SearchCommand,
   ODBEditor suite): not portable; replace with a D-Bus/CLI surface or
   drop. ODBEditor external editing maps naturally to an "open in
   $EDITOR + watch file" loop (the temp-file + didModifyFile callback
   flow, NoteObject.m:1793-1813, is straightforward file watching).
10. **Keychain** (sync passwords, NotationPrefs.m:327-364; encryption
    passphrases): use libsecret/GNOME Keyring on Linux.
11. **NSTokenField** (WordCountToken, TagEditingManager chips): GTK
    has no token field; the chips UI must be custom.
12. **Spotlight/NSMetadataQuery**: not used (search is entirely the
    in-memory strstr engine). Nothing to port there; a Rust port can
    keep the same design (lowercase String caches + memmem), which is
    provably fast at this scale.
13. **Localization**: `.lproj` bundles (en/de/es/fr/it/pt/zh...) via
    NSLocalizedString — use gettext or a simple table.
14. **Sparkle updates, Safari extension, menu-bar StatusItem,
    FullscreenWindow**: platform features with no 1:1; cut or
    approximate (a tray icon on GNOME is likely out of scope).

---

## 8. LICENSE CHECK (confirmed from the files)

- **nv (scrod/nv): GPLv3.** `/repos/nv/COPYING.txt` opens: "GNU
  GENERAL PUBLIC LICENSE / Version 3, 29 June 2007". Git history pins
  the switch: commit `7af2491` 2011-03-02 "Changed license to GPLv3.
  See http://scrodlog.notational.net/44603382 for details." Current
  HEAD is version 2.5.2 (2026-10-16), still GPLv3.
- **nvalt (ttscoff/nv): BSD-3.** `/repos/nvalt/License.txt`:
  "Copyright (c) 2009, Zachary Schneirov. All rights reserved."
  followed by the standard 3-clause BSD redistribution terms and AS-IS
  disclaimer; the fork predates the 2011 relicensing. Caveat: the
  nvALT tree also carries a stale full-text GPLv3 `COPYING.txt` at its
  root (leftover from upstream merges); `License.txt` is the operative
  license, consistent with the per-file BSD headers (e.g.
  FastListDataSource.m:7-16).

**Port-licensing rule recorded for the roadmap:** Commonplace is
GPL-3.0, which licenses both trees as sources. Provenance discipline
either way: port shared-ancestor files from the `nvalt/` form (the
lineage users actually run), and treat the nv tree's post-2011
additions as behavior reference only. Attribution to Zachary Schneirov
is retained either way (README acknowledgment + LICENSE notices).

---

## 9. Suggested phase anchors (derived from this map)

- Phase A: note model + storage rules (1.1, section 4:
  NoteObject/NotationFileManager/NotationDirectoryManager semantics).
- Phase B: list + query engine (1.2-1.3, section 2).
- Phase C: search-field state machine (section 3:
  AppController/NotationController/DualField).
- Phase D: editor core (LinkingEditor plain-text behaviors, no
  highlighting).
- Phase E: Markdown editing layer (MarkdownHighlight/TaskList/
  CriticMarkup/renumbering).
- Phase F: preview pipeline (section 5: renderer + WebKitGTK + themes).
- Phase G: tags/bookmarks/saved searches.
- Phase H: import/export/URL schemes.
- Phase I: Simplenote sync (section 6; out of scope per the scoping
  decision, recorded for a possible reopen).
- Phase J: encryption family (optional).
