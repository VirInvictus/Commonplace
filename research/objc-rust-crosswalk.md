# nvALT → Rust/GTK4 idiom inventory (crosswalk)

Research note. Produced 2026-11-17 from a read-only sweep of the
nvALT fork's ~120 root-level `*.m`/`*.h` files (~53k lines). All
counts exclude vendored trees (`ThirdParty/`, `PTHotKeys/`,
`RBSplitView/`, `Sparkle.framework/`, `JSON/`, `readability/`,
`Markdown_1.0.1/`, `Textile_2.12/`, `ODBEditor/`, `library/`,
`SafariExtension/`). The GTK-widget-level ground is
`research/gtk-prior-art.md`; nothing here repeats it. House precedent
line numbers were verified against the live repos.

---

## 1. Delegate protocols

**Defined (formal `@protocol`, 3):**

| Protocol | Declared | Purpose | Implementor | Rust answer |
|---|---|---|---|---|
| `SynchronizedNote <NSCoding, NSObject>` | `SynchronizedNoteProtocol.h:20` | Per-note sync contract (UUID bytes, sync MD dict, LSN) | `NoteObject` (checked via `conformsToProtocol:` at SimplenoteEntryCollector.m:111,187, WALController.m:551) | trait in the core crate; the NSCoding part becomes the journal record type |
| `SyncServiceSession <NSObject>` | `SyncServiceSessionProtocol.h:26` | One sync-service session | `SimplenoteSession` (asserted SyncSessionController.m:194) | trait object (moot: sync is out of scope, kept for completeness) |
| `SyncServiceTask` | `SyncServiceSessionProtocol.h:74` | Unit of sync work | fetchers | enum of task variants |

**Defined (informal protocols: delegate methods declared as
`@interface NSObject (Name)` categories, ~14):** `GlobalPrefsDelegate`
(GlobalPrefs.h:232), `NotationControllerDelegate`
(NotationController.h:211), `NoteObjectDelegate` (NoteObject.h:246),
`NotationFileManagerDelegate` (NotationFileManager.h:74),
`BookmarksControllerRevealDelegate` + `BookmarksControllerDataSource`
(BookmarksController.h:126,132), `SyncResponseFetcherDelegate`
(SyncResponseFetcher.h:64), `SyncServiceSessionDelegate`
(SyncServiceSessionProtocol.h:81), `TemporaryFileCachePreparerDelegate`
(TemporaryFileCachePreparer.h:55), `FastListDataSourceColumnEditing`
(FastListDataSource.h:46), `PassphrasePickerDelegate`
(PassphrasePicker.h:46), `URLGetterDelegate` (URLGetter.h:56),
`AlienNoteImporter (DialogDelegate)` (AlienNoteImporter.h:82),
`KeyDerivationDelaySliderDelegate` (KeyDerivationDelaySlider.h:38).
Every delegate ivar is untyped `id delegate`; every call site guards
with `respondsToSelector:` (59 sites).

**Consumed (framework delegates):** AppController.h:54
`<NSToolbarDelegate, NSTableViewDelegate, NSWindowDelegate,
NSTextFieldDelegate, NSTextViewDelegate, NSSharingServicePickerDelegate>`;
BookmarksController.h:67; NotationPrefsViewController.h:33;
PreviewController.h:15; PrefsWindowController.h:22 + m:27;
TagEditingManager.h:23,49; SyncSessionController.h:29. Plus informal
`validateMenuItem:` (NSMenuValidation) in 5 files (AppController.m:673,
BookmarksController.m:303, MAAttachedWindow.m:753,
SavedSearchesController.m:238, LinkingEditor.m:2343);
`NSURLConnectionDelegate` (GGReadabilityParser.m:77-100).

**Rust answer:** consumed AppKit delegates → GTK signal handlers. The
three formal protocols → traits in the engine crate. The 14 informal
delegate categories → typed closures (`Box<dyn Fn(...)>`) or
enum-commands; `respondsToSelector:` optional methods map to
`Option<callback>` fields. Sync completion callbacks stored as
**NSInvocation objects** (SyncResponseFetcher.h `successInvocation`,
InvocationRecorder whole file: a recorder producing *comparable*
invocations; 6+ NSInvocation sites in SimplenoteSession.m /
SyncResponseFetcher.m) become stored closures or a `Command` enum
(same slot as Atrium's worker `Command`,
`Atrium/atrium-core/src/db/worker.rs:41-47`); equality-comparing
recorded calls has no Rust analog: replace with explicit
request-dedup keys.

## 2. Target/action and responder-chain tricks

| Idiom | Sites (count + representative) | Rust/GTK4 translation | House precedent | Risk |
|---|---|---|---|---|
| `performSelector` total | 100 sites | per-row below | | |
| **`afterDelay:0` = next runloop tick** | 56 sites (38 `afterDelay:0.0` + 18 `afterDelay:0`), e.g. AppController.m:1248, DualField.m:338, NotationController.m:595-690, LinkingEditor.m:1716 | `glib::idle_add_local_once` | Atrium `src/ui/board.rs:451`, `shell.rs:98`, `task_list.rs:994` | none; see Surprises #6 for the queue asymmetry |
| non-zero `afterDelay:` (debounce) | 17 sites (§10) | `timeout_add_local_once` + stored SourceId, cancel-and-rearm | Viaduct `timeline_view.rs:180-190` | low |
| `performSelectorOnMainThread` | 3: AppController.m:3971,3973; LinkingEditor.m:1247 | `MainContext::invoke` / `idle_add_local` | Atrium `reminders.rs:119,324` | low |
| SEL as data (selector-driven dispatch) | FastListDataSource.m:130 + NoteAttributeColumn.m:57 (column→SEL), AlienNoteImporter.m:325, SavedSearchesController.m:271, SimplenoteEntryCollector.m:263, PreviewController.m:634, AppController.m:1522,1528 | enum + `match` (each SEL is a known variant) | none needed | low |
| Menu items as target/action (built in code) | ~25 `setTarget:`/`setAction:` sites (BookmarksController.m:153,240-272,466,506; EncodingsManager.m:173,184; ExternalEditorListController.m:375-414; NotesTableView.m:633-778 context menu; GlobalPrefs.m:461-462) | `gio::SimpleAction` + accels/menu models | Docket `main.rs:1244-1494` | low |
| **Key-forwarding: unmatched keyDown from the list goes to the search field's field editor** | NotesTableView.m:976-984 (`makeFirstResponder:controlField` then `[fieldEditor keyDown:]`); same in QuickSearchTable.m:29, BookmarksTable.m:29 | Capture-phase `EventControllerKey` that forwards text keys to the search entry | Docket `main.rs:434` | medium: GTK has no "forward this event to another widget" primitive; synthesize by focus or re-emit |
| Custom Shift-arrow selection affinity in list | NotesTableView.m:944-968 | own selection logic on `SingleSelection` | Viaduct `window.rs:1104-1130` | medium |
| `NSApp tryToPerform:` responder-chain command | DualField.m:148 (`cancelOperation:`) | route Esc through the focused widget's key controller + a global Esc action | Docket capture layer | low |
| Manual key-view loop rebuild | AppController.m:3464-3467; LinkingEditor.m:2042,2076; LabelEditor.m:34,39,43; NotesTableView.m:915,923,1085 | explicit `grab_focus()` handoff per key event; GTK has no key-view loop | Docket `main.rs:240-244` | medium: relayouts must re-run the focus handoff |
| Field-editor manipulation | AppController.m:1701-1740, 1827-1904, 2260-2264; NotesTableView.m:1426-1428; TagEditingManager.m:106 | buffer splices directly; the field-editor concept collapses | Docket `main.rs:40-42, 1175-1190` | low |
| `acceptsFirstResponder` conditionals | LabelEditor.m:29; NotationPrefsViewController.m:36 | `can_focus` toggling | | low |
| **`flagsChanged` modifier-only handling** | AppController.m:3752-3789 (option-only / ctrl-only held 1.2 s → pop word count / pop preview) | `EventControllerKey` has no modifier-only callback; needs a hack or the feature is dropped | none in house | medium: hidden UX; candidate to drop (ledger) |
| Carbon global hotkey (activate app) | GlobalPrefs.m:31-33, 440-462 via PTHotKeys | XDG GlobalShortcuts portal (no house precedent) | none | high (feature may not translate) |

## 3. ObjC categories on foreign classes

~20 public/private categories on Foundation/AppKit types (own-class
organization categories like `AppController (Importing)` are just
Rust modules):

| Category | Declared | Adds |
|---|---|---|
| `NSString (NV)` | NSString_NV.h:21 | ~30 methods: relative dates, word-range search, safe path extensions, **synthetic title extraction**, tabbify, UUID bytes, base64, URL heuristics |
| `NSMutableString (NV)` | NSString_NV.h:65 | tab replacement; encoding-guessing file loader |
| `NSScanner (NV)`, `NSCharacterSet (NV)`, `NSEvent (NV)` | NSString_NV.h:72,76,84 | small helpers |
| `NSData (NVUtilities)` | NSData_transformations.h:6 | base64, encoding-guessed string, alias/FSRef, web-archive path |
| `NSMutableData (NVCryptoRelated)` | NSData_transformations.h:31 | reverseBytes, alignment, AES |
| `NSFileManager (NV)` | NSFileManager_NV.h:25 | Finder/OpenMeta tags via xattr, text-encoding xattrs, FSRef |
| `NSFileManager (DirectoryLocations)` | NSFileManager+DirectoryLocations.h:28 | standard dirs |
| `NSAttributedString (AttributedPlainText)` | AttributedPlainText.h:48 | `allLinks`, `findNextLinkAtIndex:`, attribute checks |
| `NSMutableAttributedString (AttributedPlainText)` | AttributedPlainText.h:27 | **`sanitizeForeignStylesForImporting`** (the import sanitizer), `removeAttachments`, done-tag strikethrough, restyle-to-font, list indentation |
| `NSDictionary (FontTraits)` | NSCollection_utils.h:22,26 | font-trait mapping (incl. NSObliqueness/NSStrokeWidth, §8) |
| `NSDictionary (HTTP)` | NSCollection_utils.h:32 | URL encoding, timeouts |
| `NSSet (Utilities)/(Private)`, `NSArray (NoteUtilities)/(Sorting)` | NSCollection_utils.h:39,45,50,58 | set intersect; UUID lookup; qsort with C function pointers |
| `NSBezierPath (NV)`, `NSImage (NV)/(CenteredDrawing)` | NSBezierPath_NV.h:22,30; LinearDividerShader.h:49 | drawing helpers (die with GTK CSS) |
| `NSString (CustomTruncation)/(PreviewTruncation)` | NSString_CustomTruncation.h:22; PreviewTruncation.h:5 | custom ellipsis |
| `NSString (Markdown)` ×2 (NSString-Markdown.h:1 perl-via-NSTask AND NSString_Markdown.h:6 native), `NSString (MultiMarkdown)`, `NSString (Textile)` | | markdown flavor conversion; the perl/NSTask variants are what pure-Rust converters replace |

**Rust convention:** orphan rule forbids trait-on-foreign-type, so
the convention is **free functions in per-class modules**
(`string_nv::synthetic_title(...)`). The two categories that behave
like intrinsic model behavior (`AttributedPlainText` on the note
body, `NSString (NV)` title extraction) are absorbed as **inherent
methods on Commonplace's own types** (`NoteBody`, `NoteTitle`).
Free functions everywhere else.

## 4. NSNotificationCenter: complete custom-notification inventory

(4 `extern` constants + 11 ad-hoc string literals; 77 notify/observe
lines in corpus.)

| Name | Posted by | Observed by |
|---|---|---|
| `TextViewHasChangedContents` | AppController.m:3899 | AppController.m:171 → preview update |
| `AppShouldToggleDockIcon` | PrefsWindowController.m:1166 | AppController.m:172 |
| `AppShouldToggleStatusItem` | PrefsWindowController.m:1172 | AppController.m:173 |
| `ModTimersShouldReset` | AppController.m:1409,1767,2441,2452,3773; LinkingEditor.m:1851 | AppController.m:175 (`resetModTimers:`) |
| `NVThemesDidChangeNotification` (constant, NVThemeManager.h:15) | NVThemeManager.m:92 | AppController.m:176; PrefsWindowController.m:910 |
| `TagEditorShouldRelease` | TagEditingManager.m:695 | AppController.m:177 |
| `TextFinderShouldHide` | AppController.m:1432,1945 | LinkingEditor.m:4463 |
| `TextFindContextShouldUpdate` | AppController.m:1812,2040 | LinkingEditor.m:4460 |
| `TextFindContextShouldReset` | AppController.m:1765 | LinkingEditor.m:4462 |
| `TextFindContextShouldNoteChanges` | AppController.m:2066 | LinkingEditor.m:4461 |
| `NotationPrefsDidChangeNotification` (constant, NotationPrefs.h:29) | DeletionManager.m:235; SyncSessionController.m:211 | NotationPrefsViewController.m:101 |
| `ExternalEditorsChangedNotification` (ExternalEditorListController.m:32) | ExternalEditorListController.m:350 | PrefsWindowController.m:895 |
| `SyncSessionsChangedVisibleStatusNotification` ("SSCVSN", SyncSessionController.m:28) | SyncSessionController.m:362 | AppController.m:635 |
| `StatusItemMenuShouldDrop` | StatusItemView.m:50,64 | **none: dead** |
| `NVShouldActivate` | StatusItemView.m:53 | **none: dead** |

System names observed: NSTextStorageDidProcessEditing (LinkingEditor.m:173),
NSUserDefaultsDidChange (174), NSUndoManager DidUndo/DidRedo
(NoteObject.m:1916-1922), NSControlTextDidChange/DidEndEditing,
NSWindow DidBecomeMain/DidResignMain, NSApplication
DidBecomeActive/WillResignActive, NSTaskDidTerminate,
NSWindowDidResize, plus password-field observers in 4 dialogs.

**Rust answer:** this table IS the typed event-bus spec. Cross-layer
events → an `Event` enum over an mpsc channel consumed on the GTK
main loop; engine→app direction only where nvALT posts it. Precedent:
Atrium `atrium-core/src/db/worker.rs:5-8` ("UI thread holds a
WorkerHandle (mpsc::Sender\<Command\>) ... UnboundedReceiver\<TaskChanges\>
returned by spawn") is the house bus shape; Viaduct's GObject
`notify::read` (timeline.rs:16-49) covers item-level restyle. The two
dead notifications are dropped.

## 5. KVO / Cocoa bindings

**KVO: zero.** No `addObserver:forKeyPath:`, no
`observeValueForKeyPath:`, no `will/didChangeValueForKey:` anywhere:
the codebase is delegate/notify-driven, confirming earlier surveys.

**Cocoa bindings: 5 sites, exactly one feature** (surprise):
LinkingEditor.m:178-182 binds `automaticQuoteSubstitutionEnabled`,
`automaticDashSubstitutionEnabled`, `automaticTextReplacementEnabled`,
`continuousSpellCheckingEnabled`, `smartInsertDeleteEnabled` to
`NSUserDefaultsController` keyPaths with continuously-updates-value;
unbound at 3230-3234. Rust answer: read the settings keys at init and
re-apply on the settings `changed` signal; live-sync is trivial; no
`bind_property` needed.

## 6. Blocks and GCD

| Idiom | Sites | Off-main work | Rust per class | House precedent |
|---|---|---|---|---|
| `dispatch_async(global)`: parse off-main | GGReadabilityParser.m:103 (DEFAULT), :144 (re-dispatch to HIGH if on main, check at :142) | readability extraction, encoding detection | `std::thread` + mpsc, or the engine worker | Atrium `db/worker.rs:5-15` |
| `dispatch_async(main)`: hop back for UI | GGReadabilityParser.m:56,373; TagEditingManager.m:453,507; PreviewController.m:1004; NVThemeManager.m:316 | nothing | `idle_add_local_once` | Atrium `board.rs:451`, `task_list.rs:994` |
| `dispatch_after(main)` | 1: AppController.m:2594 (0.03 s select-text) | UI-adjacent | `timeout_add_local_once` | Viaduct `timeline_view.rs:185` |
| `dispatch_source` **vnode watch** | NVThemeManager.m:100-122 (O_EVTONLY fd + VNODE source, WRITE/DELETE/RENAME/EXTEND/ATTRIB, cancel closes fd; 0.2 s debounce at :95-98) | main-queue delivery | `gio::FileMonitor` + 200 ms debounce | Docket `main.rs:1192-1219` |
| Block literals as callbacks | ExporterManager.m:41; NVThemeManager.m:400; LinkReferenceCompletion.m:149; TagEditingManager.m:108; NVThemeManager.m:316 | UI-adjacent | closures / iterators | |
| `dispatch_apply` | 1, commented out (NVPasswordGenerator.m:95) | | | |
| NSOperationQueue / NSThread spawn | none in root corpus (thread checks only: GGReadabilityParser.m:142, LinkingEditor.m:1246) | | | |

## 7. Singletons and global state

| Accessor | Declared | Idiom |
|---|---|---|
| `+defaultPrefs` (GlobalPrefs) | GlobalPrefs.h:70 | ivar-cached; owns NSUserDefaults, NotationPrefs, bookmarks controller, cached font dicts (h:49-68) |
| `+sharedManager` (EncodingsManager, ExporterManager, NVThemeManager) | EncodingsManager.h:33; ExporterManager.h:23; NVThemeManager.h:25 | static pointer, unsynchronized lazy init (EncodingsManager.m:78-83) |
| `+sharedInstance` (SecureTextEntryManager, ExternalEditorListController, MultiTextFinder) | h:29/63; MultiTextFinder.m:61 | static `sharedInstance = nil` |
| `NSApp delegate` as service locator | dozens of sites (EmptyView.m:37, GlobalPrefs.m:861) | |
| `dispatch_once` one-shot init | AttributedPlainText.m:216-217; NVLog.m:35-36; HTMLToMarkdownConverter.m:242,617 | |

**Rust config-store ownership:** `GlobalPrefs` → a `Prefs` struct
owned by the application, exposed as `Arc<Prefs>` snapshots; the
setter → `SEND_CALLBACKS()` fan-out (GlobalPrefs.m:312-327) becomes
the prefs-changed event on the §4 bus. `NotationPrefs` → per-folder
config owned by the engine, mirrored to disk. The five manager
singletons → `OnceLock` module state or plain fields (all
main-thread-only). The selector-keyed observer registry
(`selectorObservers`, GlobalPrefs.m:280-327) with **variadic SEL
registration** (h:72) and `excludingSender:` echo suppression
(m:323) becomes typed setting-key subscriptions reproducing that
fan-out one-for-one, including the echo suppression.

## 8. NSAttributedString API surface (styling layer + sanitizer sizing)

| Attribute | Count | Where |
|---|---|---|
| NSForegroundColorAttributeName | 56 | editor body, completion colors, theme |
| NSFontAttributeName | 40 | body/table fonts (GlobalPrefs.m:846-880) |
| NSBackgroundColorAttributeName | 22 | search highlight (GlobalPrefs.m:684-701), editor bg |
| NSStrikethroughStyleAttributeName | 20 | `@done` task styling (AttributedPlainText.m:350-390) |
| NSParagraphStyleAttributeName | 20 | monospace body style, RTL |
| NSLinkAttributeName | 15 | wiki links per edit range (LinkingEditor.m:2749-2751) |
| NSStrokeWidthAttributeName | 11 | **faux bold** (-3.50, AttributedPlainText.m:181) |
| NSObliquenessAttributeName | 11 | **faux italic** (0.20, :176) |
| NSUnderlineStyleAttributeName | 8 | links/highlight |
| NSAttachmentAttributeName | 5 | tag-token attachment cells (TagEditingManager.m:108,334,410); stripped on import (AttributedPlainText.m:85; AppController_Markdown.m:205) |
| NSCursorAttributeName | 2 | link hover |
| NSSuperscriptAttributeName | 1 | sanitizer check (AppController_Markdown.m:208) |
| custom NVDoneTag/NVTaskDone/NVBulletIndentTag | AttributedPlainText.m:26-28 | hidden task/list state markers |

APIs: add/remove/setAttributes, attributeAtIndex/effectiveRange,
enumerateAttribute, replaceCharactersInRange, RTF/RTFD in-out
(NVRTFConverter), styled/plain paste duality (pastePreservesStyle,
LinkingEditor.m:603-657 strips link/foreground from pasted rich
text).

**AppKit-only behaviors to flag:** (a) synthetic obliqueness/stroke
width: Pango has no equivalents; the styling layer maps
"font lacks italic/bold trait" to a synthesized `pango::FontDescription`
tag or drops faux styles (the markdown-conversion path already treats
them as formatting to convert away, AppController_Markdown.m:205-210).
(b) NSTextAttachment-cell tag tokens: not inline-attachable in GTK;
Atrium's one-Label pill pattern (task_list.rs:398-408) re-expresses
it; the sanitizer already deletes attachments from converted text.
(c) body attrs deliberately omit foreground when black "to make the
database smaller" (GlobalPrefs.m:849-852): documents the old format;
irrelevant here (nothing persists styled text).

## 9. NSUndoManager integration points

| Point | Sites | Behavior |
|---|---|---|
| Per-note undo manager (lazy) | NoteObject.m:1911-1926 | each note lazily owns one; DidUndo/DidRedo observers call `makeNoteDirtyUpdateTime:YES updateFile:YES` (:1928-1931): **undoing an edit marks the note dirty and rewrites the file** |
| Window plumbing | AppController.m:2128-2140 (`windowWillReturnUndoManager:`), 2142-2147 (text view's = currentNote's), 153, 623-624 (swap + removeAllActions on note change) | one manager at a time |
| Composite-op grouping | LinkingEditor.m:3685/3714 (renumber), 3772/3831 (indent), 3933/3991, 4030/4035 (toggle task), 4079/4108, 5069/5116 (quote); LinkingEditor_Indentation.m:84/148,157/203 | `beginUndoGrouping` + `setActionName:` + `endUndoGrouping` |
| Invocation registration | LinkingEditor.m:3319,3353 (`prepareWithInvocationTarget:` incl. `setSelectedRange:`); NotationController.m:890 (note add/remove) | closure-capturing entries |
| Undo-state guards | LinkingEditor.m:2506,3599,3610; NotationController.m:887-892,920-932,949-957 (**nested grouping during undo** so a redone create groups with the remove) | subtle; no house analog |
| Discard | AppController.m:623; NotationSyncServiceManager.m:453,466 (**sync discards undo history**) | |
| Unused | no checkPoint/levelsOfUndo/discardAllEditing | |

**Rust answer:** per-note undo stacks remain unproven in-house
(house §Q2). Shape: `Vec<Action>` per note with explicit group
boundaries and stored closures or a `Command` enum; Quire's
irreversible-action guard covers programmatic splices
(page.rs:1005-1016). "Undo dirties the file" and "sync discards
undo" must be stated in the ledger; the isUndoing-nested-grouping
subtlety noted.

## 10. Timers and deferred work: full delay inventory

Zero-delay next-tick: 56 sites (§2). `cancelPreviousPerformRequests`
cancel-and-rearm: 34 sites (DeletionManager.m:84,111,254;
NotationController.m:617,622,1041,1088,1092,1656; LinkingEditor.m:1172,1224,1715,2712,3228,3601; NVThemeManager.m:96; more).

| Delay | Site | Purpose | New? |
|---|---|---|---|
| 0.03 s | LinkingEditor.m:1225 | full markdown highlight debounce | adjacent to known |
| 0.1 s | AppController.m:3804,3806; NotationController.m:243 | popover after modifier hold; sort-and-redisplay | yes |
| 0.15 s | TagEditingManager.m:710 | tag-edit commit if no re-entry | yes |
| 0.16 / 0.36 / 0.22 s | AppController.m:3990,4008,4025 | status-item click choreography | yes |
| 0.2 s | NVThemeManager.m:97 | theme-dir watch debounce | yes |
| 0.6 s | AppController.m:553 | offer markdown conversion at launch | yes |
| **1.2 s** | AppController.m:3757,3765 (NSTimer) | modifier held → word count (opt) / preview (ctrl) popover | **yes: hidden UX** |
| 1.5 s | NotationController.m:1048,1063 | attribute-list update retry | yes |
| 2 s | NotationSyncServiceManager.m:410 | deferred service init | no |
| **2.7 s** | NotationController.m:1100 (`changeWritingTimer` 0.0/15.0 at :1082, cleared :758) | note-write coalescing; **comment at :1086 documents that a user change invalidates the performSelector-queued write but NOT the timer-queued one** | **yes: behavioral asymmetry** |
| 7 s | SimplenoteSession.m:472 | push debounce | no |
| 15 s | NotationController.m:1094 | WAL fsync | no |
| 30 s | SimplenoteSession.m:470 | sync push | no |
| syncFrequency × 60 | SyncSessionController.m:180 | per-service sync timer | no |
| 0.065 s | TitlebarButton.m:108 | spinner frame | no |
| 0.03 s | AppController.m:2594 (`dispatch_after`) | select-text after activation | no |

Rust translation: one `SourceId` handle per debounce family,
cancel-and-rearm (Viaduct timeline_view.rs:180-190); the 1.2 s
hold-timer is `timeout_add_local` (Atrium debug/mod.rs:100); same-frame
highlight via Quire's tick callback (page.rs:642).

## 11. Archiving / persistence idioms

| Idiom | Sites | Rust plan |
|---|---|---|
| NSKeyedArchiver whole-DB snapshot (FrozenNotation: allNotes + deletedNoteSet + NotationPrefs in one archive, FrozenNotation.h:22-27) | FrozenNotation.m:57,110,140,200; defrost NC:312,362; freeze :629 | No binary equivalent wanted: per-note files stay plain text; the catalog/journal becomes JSONL or TOML written by hand parsers (~300 lines, core stays dependency-free, no serde) |
| WAL: per-note archived record → encrypt → append → fsync | WALController.m:170 (writeNoteObject), 289-313 (write+fsync), replay 477 | journal file with explicit fsync points; crash-replay preserved |
| NSUserDefaults, 63 `forKey:` sites in GlobalPrefs.m; app state incl. last-selected UUID + scroll offset (GlobalPrefs.h:184-189) | GlobalPrefs.m passim | TOML (or GLib Settings for window-state subset); hand parser in core |
| NotationPrefs embedded in the archive historically (NotationPrefs.m:177) | | per-folder `commonplace.toml` |
| xattr Finder/OpenMeta tags | NSFileManager_NV.h:25-31 (impl uses sys/xattr.h) | Linux xattrs or sidecar; platform-mapped feature (Phase 5 decision already: metadata blocks) |
| **RAM disk for encrypted temp files** | TemporaryFileCachePreparer.m:133-154 (`hdiutil`, `newfs_hfs`, `mount` via NSTask) | `$XDG_RUNTIME_DIR` is already tmpfs: the *guarantee* (plaintext never touches persistent disk for encrypted notes' external-editor temp files) is preserved by pointing there; drop the disk-image machinery; the guarantee gets a test |

## 12. ObjC runtime tricks (the no-clean-analog list)

| Trick | Site | Verdict |
|---|---|---|
| **`method_setImplementation` on `[NSCursor class] +IBeamCursor`**: process-wide cursor patch, IMPs cached (LinkingEditor.h:59, m:166-167), flipped when mouse-in && dark background (m:2301-2325) | one site | Drop the global patch: per-widget white I-beam `gdk::Cursor` on enter/leave + theme change. Small risk; no house precedent for themed text cursors. |
| **Dynamic class pair + `class_addMethod`**: runtime subclass `NVPartialTokenAttachmentCell` drawing at 0.45 alpha for partial tokens (TagEditingManager.m:75-103) | one site | CSS `opacity` on the token chip (Atrium pill precedent). Behavior preserved, mechanism gone. |
| IMP caching / direct IMP call (GlobalPrefs.m:123,131) | one site | Dispatch micro-optimization; nothing to port. |
| `objc_set/getAssociatedObject` on NSTextView ("committed height", NoteLayout.m:6-17) | one site | Field on the editor state struct. |
| Variadic SEL registration (GlobalPrefs.h:72) | | Slice of a SettingKey enum. |
| NSClassFromString/NSSelectorFromString | ETNoteScrollView.m:28; LinkingEditor.m:4472 (private NSTextFinder singleton); SyncSessionController.m:60 | Direct references. |
| Absent: no method_exchangeImplementations, no poseAs, no direct objc_msgSend (comment only, NoteObject.m:325), no IMP swapping beyond the cursor patch | | |

## 13. Memory-management relics that encode behavior

| Pattern | Site | Behavior encoded |
|---|---|---|
| **Canary-before-async**: ivar set nil BEFORE the async block that reads it, plus re-check inside | TagEditingManager.m:450-459 | token update must not message a freed field editor |
| Strong local as lifetime extension across a tick | TagEditingManager.m:501-513 | distinguishes "focus moved" from "really dismissed"; Rust: Weak/Strong widget refs across idle_add, same shape |
| Cache invalidated by filesystem observation | NVThemeManager.m:81-88, fed by vnode watch 100-122 | theme re-resolution only on real change |
| Cache invalidated by explicit discipline | LabelsListController.m:111 (from AppController.m:1243); NoteObject.m:1142 invalidateFSRef (DeletionManager.m:98,122) | label images rebuilt on font/color change; FSRefs dropped on rename/delete |
| Next-tick cache clear | DualField.m:338 | followed-link set cleared coalesced |
| Shared mutable scratch buffer (TextExtras heritage) | NSString_NV.m:332-335 (static NSMutableString) | invisible, single-threaded; Rust: owned String, do NOT replicate sharing |
| **Observer discipline asymmetry**: 13 removeObserver sites vs 117 deallocs; per-note undo observers never removed (NoteObject.m:1915-1922) | corpus-wide | process-lifetime reliance; Rust makes it explicit via Drop: note-level subscriptions drop with the note |
| FrozenNotation retains the whole archive NSMutableData while unpacking | FrozenNotation.h:25 | memory-doubling during defrost; irrelevant once format changes |

## Surprises (resists clean translation / hidden behavior)

1. **Two dead notifications.** `StatusItemMenuShouldDrop`,
   `NVShouldActivate`: posted, no observer. Drop; not in the bus.
2. **Cocoa bindings exist, once.** 5 sites (LinkingEditor.m:178-182)
   to NSUserDefaultsController: the sole use of the mechanism;
   reproduce by settings-read + changed-signal.
3. **A process-wide method patch on NSCursor IBeamCursor**
   (LinkingEditor.m:2314): the one true monkey-patch; per-widget
   cursor management is the only correct translation.
4. **RAM-disk lifecycle for encrypted notes**
   (TemporaryFileCachePreparer.m:133-154): external-editor temp files
   on a freshly created HFS+ image so plaintext never touches
   persistent disk, with an NSTaskDidTerminate state machine. Linux:
   `$XDG_RUNTIME_DIR` (tmpfs); the guarantee must be restated and
   tested.
5. **Modifier-hold popovers**: option held 1.2 s pops word count,
   control held 1.2 s pops preview (AppController.m:3752-3789):
   undiscoverable UX driven by flagsChanged, which GTK cannot observe
   directly. Recommend dropping with a ledger note.
6. **Selector-queue vs timer-queue write asymmetry**
   (NotationController.m:1086): "next user change always invalidates
   the queued write from performSelector, but not the queued write
   from timer." The 2.7 s write coalescer and 15 s WAL sync are
   deliberately not mutually cancellable. The Rust debounce design
   must keep two independent handles with this exact cancellation
   matrix, or writes coalesce differently than nvALT.
7. **Undoing dirties the disk**: per-note undo managers mark the note
   for file rewrite on DidUndo/DidRedo (NoteObject.m:1928-1931); sync
   discards undo history (NotationSyncServiceManager.m:453,466).
8. **Type-anywhere-to-search is keyDown forwarding, not focus
   policy** (NotesTableView.m:976-984): unmatched events are
   physically re-posted into the field editor. GTK has no
   event-forwarding primitive; the capture-phase controller must
   synthesize text into the entry (Docket main.rs:434 is the wiring
   point).
9. **The corpus is not pure Objective-C**: NVApexMarkdown.swift and
   NVMarkdownifier.swift sit at the root; treat them as ordinary
   modules in the port.
10. **Duplicate category name**: `NSString (Markdown)` declared twice
    (NSString-Markdown.h:1 perl-via-NSTask vs NSString_Markdown.h:6
    native): the Rust port takes the native semantics and lets the
    pure-Rust converters replace every NSTask shell-out
    (NSString_MultiMarkdown.m, NSString_Textile.m, NVRTFConverter.m).
11. **GGReadabilityParser re-queues itself at HIGH priority when it
    finds itself on the main thread** (GGReadabilityParser.m:142-148):
    a defensive off-main invariant worth carrying as a debug_assert.
12. **NSInvocation as a first-class callback value**
    (SyncResponseFetcher successInvocation; InvocationRecorder's
    comparable invocations): the sync layer's closure-as-data idiom;
    typed Command enum + explicit request-dedup keys is the
    translation (equality-comparing recorded calls has no Rust
    analog).
13. **GlobalPrefs' observer registry is the real bindings system**:
    variadic-SEL registration + selector-string dispatch + cached IMP
    (GlobalPrefs.h:72, m:123,131,280-327), including
    `excludingSender:` echo suppression (m:323). It, not
    NSNotificationCenter, is what the Rust settings-changed fan-out
    must reproduce one-for-one.
