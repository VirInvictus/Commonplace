# nvALT feature delta vs. original Notational Velocity

Research note. Produced 2026-11-17 from a read-only code survey of the
two reference clones: `.repos/nvalt` (ttscoff/nv fork @ 6a4c000,
"nvALT 3.0.40 (169)", 2026-11-13) and `.repos/nv` (scrod/nv @ v2.5.2).
Abbreviations: **NV** = `.repos/nv/...`, **nvALT** = `.repos/nvalt/...`.

**Version chronology (git evidence):** nvALT 1.0.2 tagged 2010-12-10 →
1.0.7 2011-01-14; v1.2b104 2013-03-24; the 2.2.x line ends 2017-09-19
(tag 2.2.8 = commit a863e96 "Revert 'Release 2.2.8'"; the release
commit itself was reverted on master); the 3.0 rewrite starts at
7277289 "Update nvALT for modern macOS" 2026-10-24, v3.0.1-130 the
same day, master now 3.0.40 build 169 (2026-11-13). **Important
caveat:** `nvALT/CHANGELOG.md` (563 lines, read in full) covers only
3.0.1-3.0.40. For the 1.x/2.x era the changelog has no entries;
earliest-evidence dates below come from git file-add dates, tag
messages, and branch commits.

---

## 1. CHANGELOG MINING: consolidated feature inventory

### Search

| Feature | Earliest evidence | Most recent change |
|---|---|---|
| Search-or-create single field, fuzzy title autocomplete, prefix matching | Original NV (shared) | unchanged (`SearchCommand.m` is 19 lines in both) |
| `#tag` search in the query field (`#unix` = tag-prefix match, mixes with plain words, `#a #b` ANDs) | 3.0.13 / 3.0.14 (CHANGELOG:321-327, 343-349); code `TagSearch.h/.m` (added 2026-10-28); `NotationController.m:1369`, `NoteObject.m:1847` | 3.0.19: "#tag search matches a tag prefix or the same #tag written in the title, body, or labels" (CHANGELOG:263) |
| Tag tokens: solid chips for tags on all selected notes, dimmed for partial, multi-note apply | 3.0.13/3.0.14 (CHANGELOG:322) | 3.0.27 (theme-styled selected tags in vertical layout, CHANGELOG:176) |
| Search highlighting of matched words in the note body | pre-3.0 NV base, reworked by nvALT | 3.0.35 (persist while switching notes, clear on editor focus; dark/light legibility 3.0.32; skip #tag words 3.0.13) — `SearchHighlight.h` |
| Search field keeps typed text when a note is selected | 3.0.39 (CHANGELOG:25) | same (commit e250cb4 2026-11-12) |
| Escape in editor clears search | 3.0.11 (CHANGELOG:370) | same |
| Find panel (Find Next/Previous/Replace) | nvALT 1.x (`MultiTextFinder.m` added 2011-01-14) | 3.0.40: "Command-E uses the selection for Find" (CHANGELOG:6) |

### Notes

| Feature | Earliest | Recent |
|---|---|---|
| File-based storage (vs NV's DB), extension/type tables per format | nvALT 1.x (`NotationFileManager.m` added 2011-01-14; the file-per-note *option* also exists in base nv, which has had the file since 2009) | 3.0.1: new DBs stored as plain text files, default extension `.md` (CHANGELOG:536-537). Precisely: nvALT 1.x/2.x still *defaulted* to the single database; 3.0.1 (commit c050f15, "Default to Markdown note files") flipped the default to PlainTextFormat |
| Encryption + keychain + password generator | original NV (shared); nvALT moved crypto to system libs | 3.0.1: "Crypto now uses the system libraries instead of the bundled OpenSSL" (CHANGELOG:552); fork-only `pbkdf2.c/h`, `CRC32.c/h`, `FSExchangeObjectsCompat.c/h` |
| Finder tags replace OpenMeta for note tags (OpenMeta copied over at launch) | 3.0.1 (CHANGELOG:538); `TagEditingManager.m` 95→721 lines; `GlobalPrefs.h:164-166 useFinderTags` | 3.0.14 (token editing, dark-theme legibility) |
| Convert to Markdown (migrate a Rich-Text library into a new .md folder) + one-time launch prompt | 3.0.1 (CHANGELOG:545-546); `AppController_Markdown.m` (whole file); `offerMarkdownConversionIfNeeded` AppController.m:553; action `convertNotesToMarkdown:` NotationPrefsViewController.h:94 | 3.0.20/3.0.21: completes, switches folder, reports success (CHANGELOG:254-256, 237-239) |
| Import URL (Note menu; Shift-Cmd-U; Readability via Option) | 3.0.15 (CHANGELOG:301), shortcut 3.0.17/3.0.18 (CHANGELOG:271); `importURL:` AppController.m:1129 | 3.0.15 (selected-right-away fix, CHANGELOG:306) |
| markdownify (URL→Markdown note; built-in HTML2Text + Readability, no Python) | 3.0.15/3.0.16 (CHANGELOG:302, 285-286); `NVMarkdownifier.swift` (added 2026-10-31), `HTMLToMarkdownConverter.m`, `GGReadabilityParser.m` (2012-06-27), `readability/` (2011-01-17) | 3.0.16 (Safari extension + built-in Markdownifier, CHANGELOG:284-286); `SafariExtension/` dir |
| HTML→Markdown preferences ("Convert imported URLs to Markdown", "Process with Readability") | 2.x era (`GlobalPrefs.h:209-212 useMarkdownImport/useReadability`; Option-drag hint in Preferences.xib) | 3.0.15 in-app conversion honors them |
| Word count toolbar token | nvALT 1.x (`WordCountToken.h` added 2011-05-19; pref `showWordCount` GlobalPrefs.h:207) | 3.0.22 (readable on dark themes) / 3.0.38 (blank past 300KB) |
| Remember per-note scroll position | 3.0.40 era (commit 5cf50a3 "Remember where you left each note", 2026-11-13; `GlobalPrefs.h:184-186`, key `LastNoteTopCharIndexKey` GlobalPrefs.m:1150) | latest master |
| Enter in notes list: rename vs. jump to editor | pref existed 2.x era | 3.0.28: "stays on by default" (CHANGELOG:161); `ListEnter.h` |
| RTL text direction | 2.x era (`GlobalPrefs.h:205-206`) | unchanged |
| "Auto-save" / "TRIM" | **not found**: neither term appears in CHANGELOG.md, fork code, or headers as a feature; saving on quit/close is original NV behavior. Reported as absent rather than guessed. | — |
| Aliases (notes-folder alias record) | original NV (shared, `aliasDataForDefaultDirectory` GlobalPrefs.h:194) | unchanged |

### Tags

Covered above: Finder-tag storage (3.0.1), `#tag` search + tokens
(3.0.13/14), LabelEditor for the tag edit field (nvALT 1.x,
2011-01-14; separate `LabelEditor.h/.m` not in NV), Simplenote tag
sync (merged branch `tagsync`).

### Preview (entirely nvALT-only; NV 2.5.2 has no preview at all)

| Feature | Earliest | Recent |
|---|---|---|
| HTML preview window, Toggle Preview Window, Lock Note to Preview, source-view toggle | nvALT 1.x (`PreviewController.m`, `MarkupPreview.xib` added 2011-01-14; menus in `MainMenu.xib`: "Preview", "Lock Note to Preview", "Toggle Source View", none of which exist in NV's nib) | 3.0.34 (View Source becomes `</>` icon) |
| Markdown / MultiMarkdown / Textile engines + SmartyPants | nvALT 1.x (`Markdown_1.0.1`, `Textile_2.12`, `NSString_MultiMarkdown.m` 2011-01-14; commit 74634f2 "Added SmartyPants" 2010-12-05; mode pref `markupPreviewMode` default MultiMarkdownPreview, GlobalPrefs.m:171, AppController.m:284) | 3.0.1: preview now rendered by **Apex** (cmark-gfm "Unified") via `NVApexMarkdown.swift`, replacing the bundled multimarkdown binary (CHANGELOG:525, 533) |
| Preview themes (CSS) + editor themes, Import, Open Themes Folder, MMD Composer `.theme` + Marked CSS import, sample Paper theme | 3.0.8 (CHANGELOG:390-399); `NVTheme.h/.m`, `NVThemeManager.h/.m` (2026-10-27); `Paper.theme`/`Paper.css` | 3.0.9/3.0.10 (theme-link import fixes, layout fix) |
| `nvalt://add-theme` install from link | 3.0.8 (CHANGELOG:394); `NVThemeManager.h:8-10` | 3.0.9 |
| MathJax (`$...$`, `$$...$$`, `\(...\)`, `\(...\)`) + Mermaid fenced diagrams, bundled, lazy-loaded | 3.0.20 (CHANGELOG:245-251); `preview-js/{mathjax,mermaid.min.js,highlight.min.js,wikilinks.js,highlight-light/dark.css}` | 3.0.38: code blocks syntax-colored with a light/dark GitHub palette chosen from the background (CHANGELOG:37) |
| Wiki links `[[...]]` clickable in preview | 3.0.22 (CHANGELOG:216); `preview-js/wikilinks.js` | same |
| Preview templates with `{%title%}`/`{%style%}` + custom CSS folder | 2.x era (`template.html`, `templateclean.html`, `custom.css`, `customclean.css`; menu "Open Custom CSS Folder") | 3.0.8: preview theme wraps the note in `#wrapper` (CHANGELOG:396) |
| Preview sync: preview scrolls to the block being edited, only when the block leaves the screen | 3.0.34 (CHANGELOG:103); `PreviewSync.h`, branch `preview-scrolls-to-edit` | 3.0.25 (editing no longer jumps preview to top; same-note anchors scroll) |
| Preview slicing for >300KB notes: renders lines around the caret, expands to the nearby fence, 0.6s pause | 3.0.40 (CHANGELOG:10); `PreviewSlice.h` (`NVPreviewAroundCaretCharacters = 64*1024`, `NVPreviewFenceLookbackCharacters = 16*1024`, delays 0.05s/0.6s at lines 11-14) | latest |
| Open in Marked / Preview using Marked / Open Marked Streaming Preview (`x-marked://stream/`) | 3.0.1 (CHANGELOG:532, 543); AppController.m:750-758 detects Marked by bundle id; `openInMarked:` PreviewController.h:63 | 3.0.34: "Open in Marked" removed from preview toolbar, share instead (CHANGELOG:97); Save HTML button replaced 3.0.1 (CHANGELOG:532) |

### Export / Share

- Export (multi-note) = original NV (`ExporterManager.m`, line delta
  < 60; menu AppController.m:964-970). nvALT 3.0.40 moved it to
  Option-Cmd-E (CHANGELOG:6).
- Save Preview HTML: nvALT fork (`saveHTML:` PreviewController.h:62,
  `SaveHTMLPreview.nib`); replaced by Open in Marked in 3.0.1
  (CHANGELOG:532), still present in the menu.
- Share button in the title bar (selection or whole note), Share
  Preview as rich text, Marked Share publish with `nvalt://share-auth`
  callback: 3.0.34 (CHANGELOG:100-103); `NoteShare.h` (2026-11-08),
  branch `share-menus`.

### Editor behaviors

Version map: the shortcut batch (Markdown link Cmd-Opt-K,
paste-URL-as-link, cycle marker Cmd-Opt-L, Tab indent/outdent,
blockquote/fence continuation, renumbering, paragraph move, selection
expand, F6/F7, invisible chars) **3.0.7** (CHANGELOG:413-433); marker
settings, link-id completion, backticks, wiki-link coloring **3.0.22**;
task lists **3.0.30/31**; `==highlight==`/CriticMarkup **3.0.36**;
300KB caps + GitHub code palette **3.0.38**; system scrollbars replace
custom **3.0.35**; the auto-pair toggle is 2.x-era (`UseAutoPairing`,
Preferences.xib "Match opening characters...").

### External editor integration

nvALT replaced NV's `ExternalEditingSession` with the **ODB Editor
protocol** (`ODBEditor/` dir added 2011-05-20; "Edit With" submenu via
`ExternalEditorListController`, default editor pref key `TextEditor` —
NotationPrefsViewController.m:587-590). The "Open File in Editor" menu
path is commented out in AppController.m:698.

### URL schemes & bookmarks

`nvalt://`/`nv://` registered (Info.plist:156-166; route table in
section 3). `nvalt://find/` links are also *generated* for wiki links
(NoteObject.m:1131, AttributedPlainText.m:281). Bookmarks/snapback =
original NV (BookmarksController delta only 565→573 lines).
`SavedSearchesController.m` in the fork is dead code (no header, not
referenced in Notation.xcodeproj/project.pbxproj).

### Preferences & app shell

- Menu-bar (status) icon + status menu: nvALT-only since 1.x
  (`StatusItemView.h` 2011-05-19; pref `StatusBarItem`, rocket icon
  3.0.1 CHANGELOG:560; "Open Marked Streaming Preview" in the status
  menu 3.0.1).
- Hide Dock Icon ("This will immediately restart nvALT";
  `ShowDockIcon`, Preferences.xib): nvALT-only.
- Color Schemes menu (System / User Scheme / Low Contrast;
  `ColorScheme` int default 2, GlobalPrefs.m:167): nvALT-only.
- Settings as a sidebar window with 5 panes, "Preferences"→"Settings":
  3.0.2/3.0.3 (CHANGELOG:497-499); Keyboard Shortcuts in its own window
  3.0.7 (CHANGELOG:411); Debug pane 3.0.37 (CHANGELOG:54; `NVLog.h/.m`
  2026-11-10).
- Simplenote sync removed 3.0.1, restored with **email login-code
  auth** 3.0.4/3.0.5 (CHANGELOG:464, 487); Sparkle 2 updates 3.0.1;
  macOS 12 min 3.0.1; Apple silicon 3.0.1; fonts → SF Pro 16 body /
  14 list 3.0.5/3.0.6. **Simperium** sync was a 2.x-era addition
  (`SimperiumConfig-example.h` 2013-03-18, branch `simperium`, commit
  edd6a7f "change Simplenote sync from using app engine api2 to
  Simperium api"); current `SyncSessionController.m:52-60` registers
  only Simplenote, so Simperium is gone in 3.x.

---

## 2. CODE DELTA

### Fork-only top-level source files → feature map

- Preview/Markdown/Import: `PreviewController.{h,m}`,
  `PreviewRender.{h,m}`, `PreviewSlice.h`, `PreviewSync.h`,
  `PreviewTruncation.{h,m}`, `MarkupPreview.xib`,
  `SaveHTMLPreview.nib`, `NVApexMarkdown.swift`, `NVMarkdownifier.swift`,
  `HTMLToMarkdownConverter.{h,m}`, `NVRTFConverter.{h,m}`,
  `GGReadabilityParser.{h,m}`, `readability/`, `SafariExtension/`,
  `NSString_MultiMarkdown.{h,m}`, `NSString-Markdown.{h,m}`,
  `NSString_Textile.{h,m}`, `Markdown_1.0.1/`, `Textile_2.12/`,
  `preview-js/`, `ThirdParty/` (apex/cmark-gfm), `template.html`,
  `templateclean.html`, `custom.css`, `customclean.css`, `Paper.css`,
  `Markdownify.nvhelp`, `tp2md.rb`
- Themes: `NVTheme.{h,m}`, `NVThemeManager.{h,m}`, `Paper.theme`
- Editor 3.x: `ListEnter.h`, `ListIndent.h`, `ListOutdent.h`,
  `ListSelection.h`, `ListSplit.h`, `TaskList.h`, `MarkdownHighlight.h`,
  `CriticMarkup.h`, `SearchHighlight.h`,
  `LinkReferenceCompletion.{h,m}`, `NVAutoPair.{h,m}`,
  `SelectionRedraw.h`, `ClickBelowText.h`, `MultiTextFinder.{h,m}` +
  `NSTextFinder.h` + `NSTextFinder_LastFind.m` (find panel)
- Scroll/window chrome (1.x→3.0.35): `ETScrollView`, `ETClipView`,
  `ETContentView`, `ETNoteScrollView`, `ETTransparentButton{,Cell}`
  (added 2011-05-19; system scrollbars restored 3.0.35, CHANGELOG:77-78),
  `FullscreenWindow.{h,m}` (2011-01-18), `Spaces.{c,h}`
- Search/tags UI: `TagSearch.{h,m}`, `QuickSearchTable.{h,m}`,
  `SearchFieldLayout.h`, `LabelEditor.{h,m}`,
  `CustomTextFieldCell.{h,m}`, `BTTableHeaderCell.{h,m}`,
  `NotesTableHeaderCell.{h,m}`, `NotesTableCornerView.{h,m}`
- Share: `NoteShare.h`
- Status bar/word count: `StatusItemView.{h,m}`, `WordCountToken.{h,m}`
- Crypto/storage/sync plumbing: `pbkdf2.{c,h}`, `CRC32.{c,h}`,
  `hmacsha1.{c,h}`, `FSExchangeObjectsCompat.{c,h}`,
  `DiskUUIDEntry.{h,m}`, `NotationFileManager.{h,m}`, `ODBEditor/`,
  `JSON/`, `hashcash/`, `SimperiumConfig-example.h`
- Infra: `NVLog.{h,m}` (3.0.37 Debug pane), `AppController_Preview.{h,m}`,
  `AppController_Markdown.m`, `NoteLayout.{h,m}`,
  `MarkdownButtonLayout.h`, `nvaDevConfig.h`, `MAAttachedWindow.{h,m}`
  (HUD popover, 2011-01-14), dead `SavedSearchesController.m`

### Files present in NV, absent in the fork

`AugmentedScrollView` (→ ET* views), `ImportingUtils` (→
NotationFileManager), `NVRootedDirectory{,_Private}`,
`NVStrandedKeyResponder`, `StartupRecovery`, `DiffMatchPatch/`,
`ExternalEditingSession/` (→ ODBEditor), `es.lproj`, `docs/`,
`toolbar-issue/`.

### Shared classes with heavy fork modification (line deltas)

| File | NV | nvALT | Δ | Why |
|---|---|---|---|---|
| `LinkingEditor.m` | 1610 | 5916 | +4306 | the entire 3.x editor (section 6) |
| `AppController.m` | 2313 | 4148 | +1835 | URL handling, share, word count, status item, Import URL, themes, layout |
| `PrefsWindowController.m` | 510 | 1213 | +703 | new panes (Editing rebuilt in code, Fonts & Colors themes, Debug) |
| `TagEditingManager.m` | 95 | 721 | +626 | Finder tags + token editing |
| `NotesTableView.m` | 1171 | 1759 | +588 | themes, tag tokens, row styling |
| `GlobalPrefs.m` | 924 | 1271 | +347 | ~25 new defaults |
| `NotationPrefsViewController.m` | 539 | 896 | +357 | sync UI, Convert to Markdown, Finder-tag toggle |
| `LinearDividerShader.m` | 116 | 241 | +125 | themed window chrome |
| `NSFileManager_NV.m` | 219 | 333 | +114 | file ops for Finder tags |
| `NotationPrefs.m` | 896 | 1123 | +227 | DiskUUIDEntry, tags, Simplenote code auth |
| `SimplenoteSession.m` | 1116 | 1307 | +191 | email login-code flow |
| `NoteObject.m` | 1824 | 1935 | +111 | Finder tags, `nvalt://find` URL gen, TagSearch |
| `WALController.m` | 791 | 573 | −218 | simplified post-OpenSSL crypto |
| `NotationDirectoryManager.m` | 840 | 669 | −171 | OpenMeta removal |
| `TemporaryFileCachePreparer.m` | 440 | 277 | −163 | ODB refactor |
| `PassphraseRetriever.m` | 215 | 129 | −86 | simplification |
| `URLGetter.m` | — | — | −70 | simplification |
| `ExporterManager.m`, `SearchCommand.{h,m}`, `BookmarksController.m`, `Notation.sdef` | — | — | ≈unchanged | base NV features kept |

---

## 3. URL SCHEMES

Registered schemes `nvalt` and `nv` (nvALT/Info.plist:163-164). Single
entry point: `handleGetURLEvent:withReplyEvent:` (AppController.m:569,
registered AppController.m:180); deferred to `interpretNVURL:` on
launch if no notation yet (AppController.m:584-587). Route table in
`AppController_Importing.m:161+`:

| Route | Parameters | Behavior | Evidence |
|---|---|---|---|
| `nvalt://share-auth/...` | OAuth callback from share.markedapp.com | `[previewController handleShareAuthURL:]` | AppController.m:573-574; PreviewController.m:32, 896 |
| `nvalt://add-theme` (alias `import-theme`) | `?url=<encoded file/http/https URL>` or `?path=<path>` | import editor theme / preview CSS | AppController.m:576-578; NVThemeManager.h:8-10, `importThemeFromAppURL:` |
| `nvalt://find/<search terms>` | query = `NV=<base64 UUID>` and/or `<ServiceName>=<key>` (e.g. `SN=...`) | sets search to terms; resolves the note by UUID/sync key and reveals it; pushes the current note to the snapback stack | AppController_Importing.m:173-212; URL format doc at :163-166 |
| `nvalt://markdownify` | `?url=` or `?html=`, optional `title=`, `readability=` override | fetch/convert page to a Markdown note, overriding prefs if given; http/https only | AppController_Importing.m:214-215, 396-430 |
| `nvalt://make` | `title=`, body via `txt=` (plain) or `html=` (converted to attributed), optional `tags=`, or `url=` (import/markdownify the page instead, clearing txt/html) | creates the note; body-without-title goes through pasteboard import; cmd-clicking a wiki link to a nonexistent note synthesizes `make?title=X&txt=[[X]]` | AppController_Importing.m:217-309; LinkingEditor.m:2474 |
| `nvalt://<anything else>` | host treated as search terms | search for the host string | AppController_Importing.m:310-318 |
| failure | — | NSBeep | AppController.m:584-585 |

`x-callback-url`: **none found** (grep over *.m/*.h/*.swift). Also
used internally: `x-marked://stream/` (3.0.1, AppController.m:750) and
the share return URL constant at PreviewController.m:32.

---

## 4. PREFERENCES SURFACE

Settings window: sidebar panes **General, Notes, Editing, Fonts &
Colors, Debug** (PrefsWindowController.m:957,966). Model: `GlobalPrefs`
(app-global, selector-keyed observer pattern, `GlobalPrefs.h`) +
`NotationPrefs` (per-database: encryption, storage format, sync,
extensions — NotationPrefs.h:22-23). Registered defaults:
GlobalPrefs.m:138-197.

- **General:** notes folder picker ("Read notes from folder",
  alias-based), external text editor (`TextEditor` key, combo +
  "Default" reset), list text size/font (`TableFontSizeKey`,
  `TableFont`), Bring-to-Front hotkey (PTHotKeys,
  `AppActivationKeyCombo`), menu bar icon (`StatusBarItem`), Hide Dock
  Icon (`ShowDockIcon`, restarts app), quit-when-closing-window, RTL,
  confirm note deletion.
- **Editing** (rebuilt in code, PrefsWindowController.m:444+):
  Markdown marker choices: bold `*`/`__`, italic `*`/`_`, list
  `-`/`*`/`+` (GlobalPrefs.m:596-617, defaults `*`,`*`,`*`);
  **Enable Markdown Completion** (`UsesMarkdownCompletions`, default
  YES); Enter-in-list-renames-note (default YES); Tab key indents
  (radio; Option-Tab always indents); soft tabs + 4 spaces;
  auto-indent new lines; auto-format list bullets; auto-format
  `@done`; auto-pair; spelling as you type; suggest note-links /
  auto-select by title (`AutoSuggestLinksKey`,
  `AutoCompleteSearchesKey`); URLs clickable; styled paste
  (`PastePreservesStyleKey`); search-term highlight + color
  (`HighlightSearchTermsKey`).
- **Fonts & Colors:** body font (default SF Pro 16; legacy Helvetica
  12 upgraded once — `fontIsLegacyBodyDefault`), list font (SF Pro 14,
  added 3.0.5/6), user color scheme fg/bg (affects "User Scheme"
  only), search-highlight color, **editor theme** + **preview theme**
  pickers with Import and Open Themes Folder
  (PrefsWindowController.h:111-114), readable note width + max width
  slider (`KeepsMaxTextWidth`, `NoteBodyMaxWidth` default 660,
  GlobalPrefs.m:165-166).
- **Notes** (NotationPrefsViewController.h): storage format popup
  (PlainText/RTF/HTML/WordDoc/WordXML — NotationPrefs.h:27), allowed
  extensions/types tables + make-default + confirm file deletion,
  encryption enable/change password/key-length stepper/keychain
  storage/secure text entry, Simplenote sync block (account, emailed
  login code, frequency, merge, enable, encrypted-folder warning —
  :44-60), **Use Finder Tagging** (`useFinderTags`), **Convert to
  Markdown** (`convertNotesToMarkdown:`).
- **Debug** (3.0.37): "Log errors and warnings" (default YES), "Log
  all diagnostic messages" (default NO), Open Console
  (GlobalPrefs.m:150-151; NVLog).
- **Hidden keys:** `ColorScheme` (0/1/2), `markupPreviewMode` (default
  MultiMarkdownPreview=2), `WebKitDeveloperExtras`,
  `NVOfferedMarkdownConversion` (one-time Rich-Text→Markdown prompt),
  session restore `LastSearchStringKey` / `LastSelectedNoteUUIDBytesKey`
  / `LastScrollOffsetKey` / `LastNoteTopCharIndexKey` /
  `LastSelectedPreferencesPaneKey`, list columns
  (`NoteAttributesVisibleKey`, `TableSortColumnKey`,
  `TableIsReverseSortedKey`, `TableColumnsHaveBodyPreviewKey`),
  `BookmarksVisible`, `ToolbarHidden`.

---

## 5. ACTIVE BRANCHES

All 14 remote branches are **merged into master** (verified
`merge-base --is-ancestor`); two hold superseded/unmerged tips:

- `preview-scrolls-to-edit`: 02a3cd5 "Add preview scroll-to-edit",
  13b9ec2 "Add the preview sync line helper" (shipped as
  `PreviewSync.h`, 3.0.34).
- `share-menus`: 21f2928 "Add editor and share menus", 1180b0b "Share
  the preview as rich text" (shipped as `NoteShare.h`, 3.0.34).
- Historical lines (all merged): `2.2.5` (2017 High Sierra fixes),
  `2.2-(113)` (2016 libcrypto), `v2.2105` (Sparkle feed change),
  `temp`/`dbFixes` (2013 DB fixes), `simperium` (2013
  Simplenote→Simperium API), `ODBEditor` (2011), `tagsync` (Simplenote
  tag sync via JSON POST), `teststicky` (2011 BWToolkit),
  `tokenized-tags` (tip 929652b "Edit tags as tokens in the list"
  2026-10-28 → 3.0.13/14), `fix/preview-scroll-position` (tip 9da07f3
  anchor scrolling 2026-11-05 → 3.0.25).
- Recent master activity rides contributor feature branches merged
  before deletion (`feature/note-scroll-position` by djp3 → 5cf50a3
  "Remember where you left each note", 3.0.40 era). **No branch
  encodes unshipped future work**; the future-facing signals are the
  two superseded tips above (preview-sync refinement and richer share
  menus).

---

## 6. nvALT 3.x EDITOR (the UX contract to replicate)

All editor logic lives in `LinkingEditor.m` (5916 lines) plus small
header-only C modules with tests in `tests/`.

- **Syntax highlighting** — `MarkdownHighlight.h`. Scans the whole
  note per edit (headings, emphasis, bold, code spans/fences/indented
  code, list markers incl. nested, links/ref-links/images). **Cap:
  `NVMarkdownSyntaxHighlightLimit = 300 * 1024` UTF-16 chars ≈ 300KB**
  (MarkdownHighlight.h:6); past it the editor renders plain text while
  staying responsive (comment at :3-5; changelog 3.0.38 "turns off
  past about 300KB so a long note stays responsive while you type").
  Below the cap only the edited lines are re-colored (3.0.38;
  `paintMarkdown...`/`highlightCriticMarkupInString:` pipeline around
  LinkingEditor.m:1830-1846).
- **==highlight== and CriticMarkup** — `CriticMarkup.h` +
  `NVThemeStyle.criticStyleForKind:fromThemeStyle:` /
  `highlightStyleFromThemeStyle:` (NVTheme.h:39-40). `=`/`~` wrap a
  selection as `==...==`/`~~...~~`, otherwise insert literally
  (NVAutoPair return-code 3). Colors come from the theme; fallback
  yellow/red/green/cyan (3.0.36). Deletions struck, insertions
  underlined, substitutions both.
- **Tasks** — `TaskList.h` (pure-C parser `NVTaskParseLine`): Opt-Cmd-T
  toggles `- [ ]` on the current line (LinkingEditor.m:1867-1869);
  clicking the checkbox space toggles `x`
  (`nv_toggleTaskCheckboxForEvent:` :2263, `NVTaskToggleEdit`);
  completed items struck via `NVTaskDoneStrikethroughRange` (honors a
  trailing ` @done`); Return continues with `[ ] `
  (`NVTaskContinuationSuffix`); Return on an empty checkbox clears the
  line (`NVTaskLineIsEmptyItem`); Tab/Backtab near the checkbox
  indents instead of inserting link brackets
  (`NVTaskCursorBelongsToCheckbox`); everything is suppressed inside
  code fences (`NVTaskIndexIsInsideFence`).
- **Lists** — Return continues bullets/numbers with correct nesting
  (`insertMarkdownStructureNewline` :4983+); **auto-renumbering** of
  numbered lists debounced 0.05s (`scheduleNumberedListRenumber`
  :3594, `renumberNumberedLists` :3605, single undo group "Renumber
  List" :3713), gated by `autoFormatsListBullets` and skipped past the
  300KB cap (:2762-2763). Outdent re-joins the list it lands on:
  bullet→next number, number→bullet (`ListOutdent.h`; fixed in
  3.0.40). Cmd-Opt-L cycles `-`→`*`→`+`→numbered→none on the line or a
  multiline selection (`nv_cycleListMarkersOnLineRanges:` :3899,
  `performKeyEquivalent:` :1863).
- **Code fences** — Return inside an open fence continues it with the
  same fence char/length; an empty fence line closes it; fence
  detection shared via `NVTaskFenceInfo`. The closer check counts
  earlier fences only within a bounded walk, specifically to kill the
  return-key hang on long notes (comment :4985 "Closing a fence counts
  every earlier fence. On a long note that walk is the return-key
  hang."); notes past 300KB skip fence-closing entirely (3.0.38).
- **Auto-pairing** — `NVAutoPair.h`: symmetric chars type over the
  closer; backtick pairs then types over; a backtick right after
  another stays single so triple-backtick opens a fence; `=`/`~`
  selection-affix behavior. Toggle pref `UseAutoPairing` (default NO,
  GlobalPrefs.m:176).
- **Reference links / footnotes** — `LinkReferenceCompletion.m`:
  completion of `[text][id]` from existing `[id]:` definitions and of
  unused ids on a new definition line; case-insensitive; fenced code
  ignored; defined ref-links get link color
  (`NVCompleteReferenceLinkRanges`). F6 = insert-or-jump reference,
  F7 = footnote (`handleMarkdownFunctionShortcut:` :4956-4968,
  `insertOrJumpMagicReference`/`insertOrJumpMagicFootnote`,
  `highestMagicNumberForFootnote:` :5738); jumps scroll the caret into
  view.
- **Shortcut table** — `performKeyEquivalent:` (:1849) +
  `handleMarkdownCommandShortcut:` (:4914): Cmd-Opt-K insert
  `[sel]()`; Cmd-B/Cmd-I bold/italic; Ctrl-Cmd-Left/Right
  indent/outdent lines (`shiftLeftAction/shiftRightAction` :4926);
  **Ctrl-Cmd-Up/Down moveCurrentParagraphUp/Down** (:4936);
  **Cmd-Opt-Up/Down expandMarkdownSelection/shrinkMarkdownSelection**
  by Markdown structure level (:4943; state
  `markdownSelectionLevel/anchor` :5674-5710); Return with the caret
  on a link opens it (:1879-1893). The 3.0.7 changelog entry
  introduced this batch.
- **Long-note contract (the caps and their rationale):** 300KB is one
  constant reused everywhere: highlighting off (MarkdownHighlight.h:8),
  renumbering + fence-closing skipped and word count blank (changelog
  3.0.38; NotationController/AppController honor it), preview switches
  to caret slices (`PreviewSlice.h`: 64KB around the caret, 16KB fence
  lookback, so Apex/WebKit never see the whole file) and its update
  delay rises from 0.05s to 0.6s (`NVPreviewUpdateDelayForLength`,
  asserted in tests/PreviewSliceTests.m:66-74). Rationale stated in
  headers: keep typing/Return responsive (MarkdownHighlight.h:3-5;
  LinkingEditor.m:4985; PreviewSlice.h:3-6). Other pixel-level UX:
  click below the last line lands at end-of-text (`ClickBelowText.h`),
  selection redraw grows 1pt to avoid hairline artifacts
  (`SelectionRedraw.h`), caret restored per note (3.0.40, GlobalPrefs
  `LastNoteTopCharIndexKey`).
- **Search-highlight contract** — `SearchHighlight.h:NVSearchHighlightQuery`:
  highlights persist while arrowing between result notes (even after
  the field shows the title), clear when the body takes focus or the
  search is cleared; legibility picks dark/light text by
  luminance/saturation of the highlight color (:44-50). Tag words
  excluded (`NVStringByRemovingTagWords`).
- **Word count** — toolbar `WordCountToken` (NSTokenField subclass,
  theme-colored); blank past the 300KB cap (3.0.38).

**Parity items from original NV kept intact:** encryption/keychain,
multi-format storage, Simplenote sync, bookmarks/snapback,
columns/sorting/layout (horizontal/vertical), exports, Stickies,
services, PTHotKeys activation hotkey, encoding fixes ("Fix Text
Encoding"), RBSplitView panes.

**Explicitly not found:** any `x-callback-url` handler; any feature
named "TRIM" or "auto-save"; any per-note alias mechanism beyond
folder aliases; Simperium in the 3.x runtime (config stub only);
`handleMarkdownFunctionShortcut` uses keyCodes 97/98 (F6/F7) with no
UI remapping.
