# nvUltra Feature Survey (Commonplace parity baseline)

**Report date:** 2026-11-17. All web sources fetched 2026-11-17 unless the source carries its own date.
**Question as dispatched:** Everything publicly known about nvUltra (Brett Terpstra and Fletcher Penney's closed-source macOS successor to Notational Velocity and nvALT), as a citation-backed feature survey feeding a 1:1 parity roadmap for Commonplace (Rust/GTK4 port of the NV/nvALT line).
**Depth:** full workup.

## Findings first

nvUltra is an unreleased, macOS-only, keyboard-modal note app that treats any folder of plain MultiMarkdown files as a notebook. Its public surface is unusually well documented for an app that never shipped: a full manual lives at `nvultra.com/help/` and a changelog records builds from the 2019 private beta through 2025.10.28. The parity surface for Commonplace is: modal search/create bar; prefix-indexed full-text search with boolean operators and tag operators over MultiMarkdown metadata tags; unlimited simultaneous notebooks with subfolders and a Folder Navigator; wiki links and `{{transclusion}}`; an embedded MultiMarkdown 6 toolchain; live side-by-side preview with themes, MathJax or KaTeX, and highlight.js; CriticMarkup with track changes; saved searches on hotkeys; an `x-nvultra://` URL handler whose find verb is deliberately nvALT/NV compatible; and AppleScript plus external-editor hooks. It has no built-in sync of any kind by design: notebooks are ordinary folders that users point at Dropbox, iCloud Drive, or Google Drive. Simplenote sync and RTF storage were explicitly cut from the nvALT feature set. As of late 2026 the app has been in beta for seven and a half years; Brett revived nvALT (3.x, October 2026) as the interim app and says he will "return to nvUltra now," still "shooting for 2026" as the release year. No price, launch date, or iOS commitment has ever been published; the one concrete distribution fact is a planned Mac App Store release with in-app purchase and subscription support (feature-frozen and in MAS testing as of May 2022, StoreKit work visible in the changelog through 2024).

## Feature summary table

Sources are bracketed numbers resolving in the Sources list. "Yes" means documented in an official nvUltra source.

| Feature | Documented? | Source |
|---|---|---|
| Modal search/create bar, one field for both (⌘L) | Yes | [1] [3] [4] |
| Plain text (MultiMarkdown) files; no database or proprietary format | Yes | [1] [3] [4] |
| Any folder is a notebook; unlimited notebooks; multiple open at once | Yes | [1] [4] |
| Nested subfolders, searched recursively, shown flat as `Sub/Note.md` | Yes | [3] [4] |
| Folder Navigator with fuzzy matching returns to any opened folder (⌃⌘N) | Yes | [9] [33] |
| Titles containing `/` create subfolders; Tab completes subfolder names | Yes | [4] [12] |
| Non-text files (images, TaskPaper, OPML, etc.) live in notebooks, indexed by name, Quick Look in editor pane | Yes | [1] [4] |
| PDFs fully indexed for search (slower on large files) | Yes | [1] [4] [6] |
| Prefix-indexed full-text search (f/fo/foo), SQLite-backed, disk-cached | Yes | [6] [14] |
| Boolean AND/OR/NOT with `&&`, `||`, `!` aliases and parentheses | Yes | [5] [6] |
| Tag searches `#tag` and `tag:tag`, full tag name required, autocomplete | Yes | [5] [7] |
| Quoted exact phrases; case folding; diacritic folding | Yes | [5] [6] |
| Known limits: no mid-word matches, no regex, punctuation not indexed | Yes | [6] |
| Saved searches with ⌘1–⌘9 and ⌘0, reorderable | Yes | [4] [22] [35] |
| Connections mode ranks similar notes by full-content comparison | Yes | [1] [9] [31] |
| MultiMarkdown 6 engine embedded (libMultiMarkdown, MIT) | Yes | [28] [48] |
| "Strict Markdown" toggle limiting to original Markdown spec | Yes | [7] [16] |
| MMD metadata blocks (`tags:`, `title:`, unlimited keys) at note top | Yes | [7] |
| Inline `#hashtags` searchable; optional two-way Finder tag sync | Yes | [7] [15] |
| `[[Wiki links]]` clickable in editor and preview; partial-title match | Yes | [9] [18] |
| Transclusion `{{filename.md}}` in editor and preview | Yes | [9] [16] |
| Drag notes in as link, image, wiki link, or transclusion; two-stage ⌘Z | Yes | [9] [10] |
| Live side-by-side preview; synchronized scrolling ("new algorithm") | Yes | [1] [2] [17] |
| MathJax or KaTeX (CDN) for math; highlight.js for code | Yes | [1] [17] |
| Editor themes and preview CSS themes, light and dark, user-editable; separate print theme | Yes | [1] [9] [19] |
| Copy As HTML (⇧⌘C) and LaTeX (⌥⌘C) | Yes | [13] [16] |
| Marked 2 handoff for export; Print Preview to PDF (macOS 11+) | Yes | [1] [25] |
| Smart pairs (customizable), type-over periods | Yes | [18] |
| Smart Return: lists, blockquotes, tables, code indent, heading case and closing hashes | Yes | [18] |
| Paste As blockquote/list/numbered list/table/table-rows | Yes | [10] |
| Convert Selection To (any MMD syntax to any other) | Yes | [10] |
| Cleanup tools: list numbering, rewrap, table reformat, metadata cleanup | Yes | [10] |
| Elastic tabstops | Yes | [1] [18] |
| Title/sentence/upper/lower case transformations | Yes | [1] |
| CriticMarkup incl. automatic Track Changes (⌃⌥⌘X), accept/reject, initials and datestamp | Yes | [9] [13] [21] |
| Text expansion: JSON snippet files, strftime dates, nesting | Yes | [12] [20] |
| Autocomplete labels/footnotes; document-word autocomplete; custom dictionaries | Yes | [12] [18] [20] |
| History undo mode; undo coalescing by word/sentence/paragraph | Yes | [12] [20] |
| Typewriter mode (⌃⌘T) with configurable center height | Yes | [8] [19] |
| Focus: hide file list (⇧⌘F), toggle preview (⌃⌘P) | Yes | [8] [13] |
| Invisible characters (⌥⌘I); line numbers (⇧⌘L); paragraph numbers | Yes | [8] |
| Info bar word/character/line/paragraph counts (⇧⌘I) | Yes | [8] |
| Selection scope grow/shrink; column selection (⌥); multi-selection (⌘) | Yes | [10] [13] |
| Paragraph and header navigation keys; in-note find (⌘F/⌘G) | Yes | [11] [13] |
| Multi-document find/replace across the notebook | Yes | [25] |
| Autosave plus save-on-focus-loss | Yes | [9] |
| External editors for any note or file (⇧⌘O), including system default | Yes | [1] [13] |
| Auto-refresh when another program changes a file | Yes | [15] |
| `x-nvultra://` URL handler: find (nvALT/NV compatible), make, open, addtheme | Yes | [12] [38] |
| AppleScript: `file/note/note link of window` (bundle id com.multimarkdown.nvUltra) | Yes | [12] |
| Hidden defaults for table metrics, fonts, focus and whitelist behavior | Yes | [12] |
| Global activation hotkey; menu bar mode without Dock icon | Yes | [14] [33] |
| Configurable pane order (list left/right, preview left, vertical layout) | Yes | [14] |
| Sort orders (newest/oldest/alpha/reverse); ignore patterns with `%` and `_` wildcards | Yes | [14] |
| Line endings choice (Unix/Windows/Mac classic); default extension | Yes | [15] |
| Import paths: nvALT/NV, Bear, Evernote via Joplin, Notes.app, Notion, Obsidian, Simplenote, Ulysses | Yes | [26] |
| No built-in sync; folders live on Dropbox/iCloud Drive/Google Drive | Yes | [4] [33] |

## 1. Complete public feature list

The authoritative sources are the nvUltra homepage [1] and the manual at `nvultra.com/help/` [2]; the manual's own note that it is a work in progress means gaps are possible. What follows is the documented surface, organized by area.

### Core model and notebooks

nvUltra works only with folders, never loose files: first launch asks for a notes folder, and everything inside (including subfolders, searched recursively) becomes the notebook [4]. The FAQ restates it: "nvUltra opens folders rather than individual files"; dragging a file into the file list copies it into the notebook [24]. Any number of notebooks can be open at once, in separate windows, with drag and drop between them; the Folder Navigator (⌃⌘N, fuzzy filter, Return to open) returns to any folder ever opened in nvUltra [1] [4] [9] [33]. Note titles containing forward slashes create nested subfolders, and Tab completes existing subfolder names during creation [4] [12]. Subfolder notes display flat, as `Subfolder name/Note name.md` [3] [4]. Notebooks can hold any file type; non-text files are indexed by filename and get Quick Look in the editor pane, while PDFs are fully text-indexed (noted as slow for large files, hence optional disk index caching) [1] [4] [6].

### Search

Search and creation share one field (⌘L): typing filters; Return on an unmatched title creates the note; arrow keys walk results [1] [3] [4]. The documented query language [5]:

- Plain words all must match (implicit AND).
- Tags: `#tag` or `tag:tag`; the full tag name is required (no partial matching); typing the prefix triggers autocomplete.
- Boolean: `AND`/`&&`/`&`, `OR`/`||`/`|`, `NOT`/`!`, with parentheses for grouping.
- Double-quoted phrases must appear verbatim and in order.
- Case and diacritics fold (`bucher` matches `Bücher`).

Indexing internals [6]: the engine uses SQLite plus incremental prefix indexes ("foo" is indexed as "f", "fo", "foo"); complexity is O(n) in document count with document length irrelevant; accuracy is documented as zero false negatives with false positives under 2 in 10,000 per query (boolean NOT inverts that profile); hyphenated and underscore-joined words are split into subwords; Arabic numeral runs index as words; Finder tags and MMD metadata tags are both indexed. Documented limits: no mid-word matching, no regular expressions, punctuation not indexed. Indexes cache to disk (General preferences) and load into memory per folder [6] [14].

Saved searches are bookmarked from the magnifying-glass dropdown, renamed and reordered in preferences, and the first ten get automatic hotkeys ⌘1 through ⌘9 and ⌘0 in list order [4] [22] [35].

Connections mode replaces the file list with a ranking of notes similar to the current one, computed by comparing the note's full text against all other notes (image filenames and PDF contents included); it was announced at beta start as the novel addition to the NV lineage [1] [9] [31].

### Notes, links, and metadata

Tags are stored as MultiMarkdown metadata at the top of the note (key, colon, space, value; block ends at a blank line), e.g. `tags: nvultra, docs` and `title: ...`; unlimited additional keys are allowed; metadata does not render in the preview and exports as HTML `<meta>` tags via Marked or MultiMarkdown Composer [7]. Inline `#hashtags` are searchable but do not sync to the filesystem; an optional Files preference ("Sync macOS and MMD tags") two-way syncs Finder tags with metadata tags, prompting on mismatch [7] [15]. Metadata handling requires full MultiMarkdown processing; the "Limit to 'Strict' Markdown" toggle disables it [7].

Wiki links use doubled square brackets around a note title, `[[Note Title]]`, clickable in both editor and preview, with partial titles resolving to the closest match; toggleable in Editing preferences [9] [18]. Transclusion is MultiMarkdown's double-curly-brace syntax, `{{filename.md}}`, supported in editor and preview [9] [16]. Dragging a note into the editor inserts an image, file link, wiki link, or transclusion depending on filetype and the General preference governing dragged Markdown notes; the first ⌘Z strips to a bare path, the second removes the insertion entirely [9] [10]. The editor makes only Markdown links clickable, not bare URLs; wrap in angle brackets for clickability [24].

### Editing

The editor is built from Fletcher Penney's libSmartText/libSmartHighlighter libraries [28]. Documented editing behavior: MMD syntax highlighting (default-on, with an extension blacklist/whitelist to invert per filetype) [16]; customizable smart pairs and selection wrapping, plus type-over periods [18]; Smart Return options covering heading title-casing, closing ATX hashes, list continuation and spacing, blockquote prefixing, code-block indentation, table row pipes, and linebreak continuation [18]; Paste As blockquote, bullet list, numbered list, table (CSV-ish first row as header), and table rows [10]; Convert Selection To any other MMD syntax [10]; Cleanup tools for list numbering, paragraph rewrap, table reformatting, and metadata [10]; elastic tabstops [1] [18]; title, sentence, upper, and lower case transformations [1]; CriticMarkup with automatic Track Changes insertion (⌃⌥⌘X), navigation, and per-change or bulk accept/reject, with automatic initials comments and strftime datestamps [9] [13] [21]; text expansion driven by JSON snippet files in an expansions folder, with strftime placeholders, nested `%snippet:...%` references, and delimiter behavior options [12] [20]; autocomplete for link labels and footnotes inside `[]` references, document-word "smart autocomplete" with case-adapting completions, and user dictionaries as plain text files [12] [18] [20]; regular or History undo mode with coalescing by word, sentence, or paragraph [12] [20]; typewriter mode with a 0 to 1 configurable center height [8] [19]; invisible characters, line numbers, paragraph numbers [8]; counts bar [8]; selection scope grow/shrink, option-drag column selection, command multi-selection [10] [13]; paragraph and header navigation shortcuts (ATX and Setext aware) [11]; in-note find with ⌘F/⌘G/⇧⌘G and a multi-document find/replace workflow [11] [13] [25]. Saving is automatic, with an extra save whenever the editor loses focus [9].

### Preview and themes

A live side-by-side HTML preview updates on every text change or on save (preference), with synchronized scrolling using "a new algorithm" the beta page solicits problem files for [1] [2] [17]. Math typesetting loads KaTeX or MathJax from CDN (one or the other); code highlighting via highlight.js CDN [1] [17]. Preview appearance is a CSS stylesheet ("Default CSS"), overridable per notebook via View, with an Open CSS Folder button for user themes [9] [17]. Editor themes are separate, chosen in Appearance preferences with their own themes folder; a distinct theme can be set for printing [9] [19]. The homepage theme gallery shows pairs including Modern Red/Gregarious, Default/Gregarious, Default Dark/Resolute Dark, Solarized Dark/GitHub, Solarized/GitHub, and Pretentious/Pretentious [1]. Mermaid diagramming is possible by injecting the script via note body or `xhtml` metadata key and using a `mermaid` fenced block; refresh with ⌘R [25]. Export paths: Copy As HTML and Copy As LaTeX [13] [16], File's Preview in Marked 2 (recommended for PDF), and Print Preview to the system print-to-PDF dialog, requiring macOS 11 or newer [25].

### File handling, integration, and extensibility

Any note or file can open in any capable external editor (⇧⌘O), with a shortcut for the system default app [1] [13]; a Files preference auto-refreshes notes changed by other programs, recommended on [15]. Line endings (Unix, Windows, Mac classic) and the default file extension are configurable [15]. General preferences add: pane ordering (file list left or right, preview left, vertical layout), Dock and menu bar visibility (menu bar mode hides the Dock icon), a global activation hotkey, sort orders, hide-extension display, files-to-ignore patterns matched against path suffixes with `%` (multi-char) and `_` (single char) wildcards, and disk index caching with a Clear Cache button [14]. Hidden `defaults write com.multimarkdown.nvUltra` keys adjust notes-list metrics (row size, fonts, spacing, selection indicator width), info bar fonts, focus-to-search-on-activation, a text file extension whitelist, and wiki link completion behavior [12].

The URL handler `x-nvultra://` exposes four verbs: `find` (nvALT and Notational Velocity compatible, via `?txt=` or a path form), `make` (create a note with text and optional title and notebook; slashes in names create subfolders), `open` (richer find accepting notebook/note paths), and `addtheme` (install editor themes or preview CSS from percent-encoded strings) [12]. AppleScript exposes `file of window`, `note of window`, and `note link of window` (an x-nvultra link) [12]. Third parties have built on this: Brett's Gather CLI creates titled notes through the URL handler [38], and Hookmark ships a nvUltra integration (Copy Link as `nvUltra://` URLs, Hook to New note, reveal in Finder at ⌥⌘R) [47].

Import paths are documented for nvALT/Notational Velocity (open the plain text folder; database users must switch nvALT to plain text storage first; OpenMeta tags need a conversion script to Finder tags), Bear (Markdown export), Evernote (via Joplin import then Markdown export), Notes.app (Automator workflow requiring Pandoc), Notion (Markdown and CSV export with unzipping), Obsidian (open the vault directly; alias-style wiki links `[[Note|Title]]` not fully supported; file embeds should become image syntax or transclusions), Simplenote (zip export; the JSON tag file is not read yet), and Ulysses (External Folder copy-out) [26].

## 2. Design philosophy

The throughline in seven years of statements: keep Notational Velocity's modal, keyboard-first capture model; replace the single database bucket with plain text folders; and make MultiMarkdown the native tongue.

- Lineage and modal model. The announcement post (April 10, 2019) says nvUltra "works a lot like nvALT (and Notational Velocity, naturally)": pop it up, start typing, and search-or-create happens in seconds [30]. The homepage's own "Modal Entry" copy: "It's easiest to take notes and jot down ideas when you don't have to think about where to put them" [1].
- Folders, not a database. nvUltra "works only with a folder of Markdown files", which Brett called "the only way I recommend using nvALT" when pleading with stragglers to convert before nvALT died [39]. The homepage: notes are "never locked in a database or a proprietary format. Edit them with any other application, even while you're using nvUltra. We'll keep up!" [1].
- The "one big bucket" inheritance with multiple buckets. The Folder Navigator exists instead of a three-pane library sidebar, reflecting "the 'one big bucket' philosophy of Notational Velocity", but any number of buckets can be open at once, including one on Dropbox or iCloud and one on an encrypted private disk [33].
- What was refused. No Simplenote sync, because "Simplenote requires a single bucket" [33]. No RTF, ever: the app is "doubling down" on Markdown for portability and longevity [33]. No regex search, because scanning full text "defeats the purpose of using an index" [6].
- Small-team honesty. "It's 2 part-time guys working with 1000+ users" (September 2019); Fletcher Penney is an ER doctor, and both the 2020 and 2021 delay posts attribute schedule slip primarily to his hospital workload and to scope growth that forced "completely rewriting" core components [34] [35] [36].
- Successor framing. nvALT's project page: "nvUltra is the app I am building to follow it" [51]. In October 2026 Brett still calls nvUltra "a far superior app" (a complete rewrite with better Markdown, multiple notebooks, subfolders) and says he revived nvALT because he had "left a lot of people in the lurch", to "keep things running until I can offer the viable alternative" [41].

## 3. Divergence from nvALT

nvALT's current documented behavior comes from Brett's nvALT project page (captured 2026-11-16, describing nvALT 3) [51]; nvUltra's from its manual and posts. Feature by feature:

| Aspect | nvALT (incl. nvALT 3, 2026) | nvUltra |
|---|---|---|
| Storage | One notes folder, or the legacy SQLite-style note database; rich text libraries supported (convertible once); nvALT 3 defaults new libraries to plain text `.md` [51] | Folders of plain text only; no database mode, no RTF [3] [4] [33] |
| Notebooks | Single library per window | Unlimited notebooks, multiple windows, drag between them, Folder Navigator [1] [4] [9] |
| Subfolders | Not part of the model (flat notes folder) | Native, searched recursively, created by slashes in titles [4] [30] |
| Tags | Written to disk as Finder tags (OpenMeta migrated on launch); #tag prefix search with partial tag matching (`#uni` matches `unix-admin`) [51] | MultiMarkdown metadata `tags:` primary; inline hashtags; optional Finder tag sync; tag search requires the full tag name [5] [7] |
| Search | Title, body, and tags; every word must match; quoted phrases; colon splits words [51] | Prefix-indexed full text; boolean AND/OR/NOT with aliases and grouping; quoted phrases; tag operators; diacritic folding [5] [6] |
| Simplenote | Native Simplenote sync in Notes settings [51] | None, by stated design [33] |
| Preview | Apex Markdown preview; nvALT 3 added editor and preview themes [41] [51] | Own live MMD preview: sync scrolling, MathJax/KaTeX, highlight.js, CSS themes [1] [17] |
| Markdown depth | Markdown editing with completion, highlighting, CriticMarkup highlights in nvALT 3 [51] | Full MultiMarkdown 6: metadata, tables, footnotes, citations, transclusion, CriticMarkup accept/reject tooling [10] [16] [21] [28] |
| Inter-note links | Not in the model | Wiki links, transclusion, drag-as-link [9] [18] |
| Similar notes | None | Connections mode [1] [31] |
| Saved searches | Not documented on the project page | Saved searches with ⌘1 through ⌘9 plus ⌘0 hotkeys [4] [22] [35] |
| URL scheme | `nvalt://` (find; nvALT 3 adds add-theme) [41] | `x-nvultra://` with find kept nvALT/NV compatible, plus make, open, addtheme [12] [41] |
| External editors | Note menu lists configured editors [51] | ⇧⌘O edit in any external editor or system default [1] [13] |
| Publishing/sharing | nvALT 3: Peggd removed, publishing via Marked Share (one-off share, not sync) [41] | Nothing documented |
| Layout | List above or beside editor [51] | Three panes, configurable order and orientation [14] |

Cut outright from the nvALT world: Simplenote sync, the note database, RTF notes, and (on the nvALT 3 side) Peggd publishing [33] [39] [41]. Added on the nvUltra side: multiple notebooks, subfolders, boolean search, metadata tags, wiki links, transclusion, Connections, saved searches, live preview with math and code highlighting, CriticMarkup change tracking, text expansion, History undo, and the URL handler's make/open/addtheme verbs. Continuity is deliberate: the URL handler's find verb preserves the nvALT/NV calling convention [12], and nvALT database users reach nvUltra only by flipping nvALT to plain text storage first [26] [39].

## 4. Sync stance

nvUltra ships no sync service and documents none as planned. The position is structural: notebooks are ordinary folders, so sync is the filesystem's job. Getting Started says folders "can live on sync services (Dropbox, iCloud Drive, Google Drive)" and notes remain editable in any plain text editor anywhere [4]. The August 2019 post makes the philosophy explicit: no Simplenote sync because "Simplenote requires a single bucket", with Dropbox and iCloud Drive named as the preferred roads [33]. The FAQ documents the one iCloud caveat: files must be fully downloaded locally before nvUltra can index them [24], and the changelog records iCloud indexing improvements in the 2021.12.24 build [29]. Finder tag sync (an on-disk attribute, not note sync) is the only data that can move both ways, and only when enabled [7] [15]. On the adjacent nvALT 3 side, Marked Share publishing is described as one-off sharing only, "not full Simplenote-style sync yet", citing hosting costs [41]; no equivalent nvUltra sharing feature is documented. Single-source note: the search indexing page motivates index efficiency partly by scaling problems "especially on iOS", which hints at iOS having been considered, but no iOS app or sync architecture is documented anywhere [6].

## 5. MultiMarkdown and Fletcher Penney

Fletcher T. Penney created MultiMarkdown (copyright lines on his site run 2005 to 2017) and develops MultiMarkdown Composer, the long-form Mac editor nvUltra borrows its editing capabilities from [1] [49]. The collaboration's origin: Brett had been building an nvALT successor called BitWriter with David Halter, the codebase died, and Penney invited Brett onto his own in-progress project, which became nvUltra [30]. Division of labor has always leaned on Fletcher as the engineer: Brett describes consulting him as "nvUltra's developer" in 2022, and every delay post ties pace to Fletcher's hospital schedule [34] [35] [36] [38].

Technically, nvUltra embeds MultiMarkdown 6 (libMultiMarkdown, MIT licensed, credited on the manual's credits page alongside Penney's own libSmartText and libSmartHighlighter libraries, both 2019) [28]. The MMD 6 surface visible in the product: metadata blocks as the tag and title store [7]; transclusion via `{{filename}}` [16]; smart quotes with per-language styles; automatic header anchors with optional randomized IDs to avoid collisions when notes merge; footnote support [16]; CriticMarkup processing including accept/reject [21] [48]; snippet-versus-complete-document control over Copy As HTML [16]; and a strict Markdown mode that strips the MMD additions (and with them, metadata) [7] [16]. Copy As LaTeX rides the same engine [13] [16].

Upstream, the MultiMarkdown-6 repository is now marked deprecated in favor of MultiMarkdown-7 (v6.8.0 was the last line; the deprecation notice carries no date on the page) [48]. nvUltra's credits still list MultiMarkdown-6 as of this fetch, and no public statement addresses an MMD 7 migration [28].

## 6. Status and availability

Timeline assembled from Brett's posts, the official changelog, and the live site:

| Date | Event | Source |
|---|---|---|
| 2019-04-10 | Project announced as "codename nvUltra", nearing beta | [30] |
| 2019-05-29 | Private beta begins, about 20 hand-picked testers | [31] |
| 2019-06-07 | Name finalized as nvUltra | [32] |
| 2019-08-07 | ~500 testers; menu bar mode, Folder Navigator; Simplenote and RTF ruled out | [33] |
| 2019-09-25 | 1000+ testers; "feature complete for 1.0"; feature requests deferred to post-launch | [34] |
| 2020-04-01 | Beta full; saved searches added after feature completeness | [35] |
| 2021-03-23 | Major reconfiguration of underpinnings underway in beta | [36] |
| 2022-05-20 | Feature freeze; "finalizing the MAS testing for the in-app purchase and subscription setup" to enable a free trial and sustainable pricing | [37] |
| 2024-04-20 | nvALT declared end of life; nvUltra still in beta, invite by email | [39] |
| 2025-10-28 | Newest changelog build (1.0.0, date-stamped build id) | [29] |
| 2025-12-29 | "Fletcher and I made a lot of progress on nvUltra this year"; beta "basically public", invite on request; Marked 3 work to port over after that ships | [40] |
| 2026-10-24 | nvALT 3 revived instead (3.0.40, build 169, macOS 12+, Apple Silicon); nvUltra still unfinished; Brett "shooting for 2026" and expecting users to pay | [41] |
| 2026-10-27 | Brett: he will "return to nvUltra now and try to do things I can actually get paid for" | [42] |
| 2026-11-17 | Live homepage still shows struck-through Free Trial/Buy/Mac App Store buttons on "coming soon" placeholders and the "private beta" banner | [1] |

Versioning: every changelog entry is labeled version 1.0.0 with date-stamped build identifiers (for example 2025.10.28.15.10, 2024.08.07.21.53); the beta-era entries use short build numbers from (20) "Initial beta release" upward [29]. Pricing: no number has ever been published. The documented model is Mac App Store distribution with in-app purchase and subscription support (MAS testing in May 2022; StoreKit/IAP work in the 202301 and 202406 builds) [29] [37], against Brett's stated distaste for subscriptions (August 2020, via the Overtired podcast as relayed on MPU Talk: code ready, pricing unresolved, subscriptions disliked but possibly necessary) [44]. Platform: macOS only; nothing documents Windows or Linux, and the only version floor visible is Print Preview requiring macOS 11 or newer [25]. iOS: no public commitment exists; see Unknowns. Activity as of late 2026: the last code movement on record is the October 2025 build plus Brett's December 2025 statement of porting Marked 3 work over; 2026's visible Terpstra effort went to the nvALT 3 revival (two releases in October 2026) with an explicit promise to return to nvUltra afterward [29] [40] [41] [42]. Context: the original Notational Velocity itself shipped 2.5.2 on 2026-10-16 after a long dormancy [50], which Brett cited as encouragement for the nvALT revival [41].

## 7. Community knowledge

- Beta user impressions (MPU Talk, August 2020, user anon41602260): nvUltra is folder-based like nvALT, with double-bracket links, MultiMarkdown, and whole-file transclusion; verdict: "No, it is not handsome. It is functional and rich", and it "out-Obsidians Obsidian" [44].
- Blake Watson (written 2019 from a beta invite, republished 2024-09-29 with the note "It's been five years and nvUltra is still in beta"): nvUltra won on typography and theming and offered "maximum portability", but the default design was "lackluster and needs some TLC", and "for my needs, nvUltra is over-powered"; he also found the keyboard-centric model a poor fit for his mouse-centric accessibility needs, and the beta lacked a clickable tag list [43].
- Release-delay fatigue (MPU Talk, August 2020): vco1 called the timeline "a joke" and asked who would subscribe "given its already very bad track record before release"; D_Rehak attributed it to engineering perfectionism; MitchWagan defended Brett's contributions to the Mac community [44].
- Hacker News (April 2024, on the nvALT end-of-life post): Leftium noted the successor "has been in development/private beta for several years"; D13Fd put it at "at least 8 years" and said Bear, Ulysses, and Obsidian "have kind of eaten its lunch"; runjake, holding a beta key, called pursuing one "not worth the time" and reported slow progress complaints on the beta forum, while still saying "I love Brett as a human" [45].
- Writing Slowly (Richard, 2026-10-25): the revived nvALT is "an interim solution" while nvUltra remains unavailable; he suspects most people who needed a simple Markdown notes app have already moved (naming MarkEdit, nvENVY, The Archive, Zettlr, Obsidian, Bear) [46].
- Acknowledged feature requests: the beta page states plainly that the developers "can't promise to implement every feature request" [2]; Gather's `--title-only` flag was added on a user request [38]; Brett has lobbied Fletcher for a native web clipper (Gather being his interim workaround) [38]; the Simplenote importer's missing tag support ends with "keep the JSON in case a script appears" [26]; and September 2019 policy was to defer new requests until after 1.0 ships [34].

## Sources

| # | URL | What it supported | As-of |
|---|---|---|---|
| 1 | https://nvultra.com/ | Homepage feature list, themes gallery, purchase placeholders, beta banner (cross-checked against local snapshot research/raw/nvultra-home.html captured 2024-05-13 and its raw `<del>` button markup) | 2026-11-17 |
| 2 | https://nvultra.com/help/ | Help site TOC; beta tester page (sync-scroll algorithm note, feature-request disclaimer) | 2026-11-17 |
| 3 | https://nvultra.com/help/welcome.html | Plain text philosophy, modal model, flat subfolder display, MMD positioning, import links | 2026-11-17 |
| 4 | https://nvultra.com/help/getting-started.html | Folders-not-files, sync-service folders, layout, creation with slashes, saved searches, Quick Look, PDF indexing | 2026-11-17 |
| 5 | https://nvultra.com/help/search-syntax.html | Full query language: tags, booleans, phrases, folding | 2026-11-17 |
| 6 | https://nvultra.com/help/search-indexing.html | Index internals, accuracy figures, limits, iOS scaling remark | 2026-11-17 |
| 7 | https://nvultra.com/help/tagging-and-metadata.html | Metadata syntax, hashtag behavior, Finder tag sync, strict-mode caveat | 2026-11-17 |
| 8 | https://nvultra.com/help/the-editor.html | Invisible chars, line/paragraph numbers, info bar, focus, typewriter | 2026-11-17 |
| 9 | https://nvultra.com/help/features.html | Folder Navigator, drag and drop semantics, Connections, themes, CriticMarkup, wiki links, autosave | 2026-11-17 |
| 10 | https://nvultra.com/help/markdown-editing.html | Editing shortcuts, selection modes, paste-as, convert-to, cleanup, drag insert | 2026-11-17 |
| 11 | https://nvultra.com/help/document-navigation.html | Movement keys, paragraph/header navigation, in-note find | 2026-11-17 |
| 12 | https://nvultra.com/help/advanced-features.html | Autocomplete, tab-complete folders, text expansion, history undo, AppleScript, URL handler, hidden defaults | 2026-11-17 |
| 13 | https://nvultra.com/help/keyboard-shortcuts.html | Complete shortcut tables incl. Copy As HTML/LaTeX, Edit File(s) | 2026-11-17 |
| 14 | https://nvultra.com/help/preferences-general.html | Pane order, menu bar mode, hotkey, sorting, ignore wildcards, index cache | 2026-11-17 |
| 15 | https://nvultra.com/help/preferences-files.html | Extension default, Finder tag sync, external-change refresh, line endings | 2026-11-17 |
| 16 | https://nvultra.com/help/preferences-mmd.html | Strict mode, highlighting lists, smart quotes, anchors, transclusion, snippet-vs-document | 2026-11-17 |
| 17 | https://nvultra.com/help/preferences-preview.html | Update modes, KaTeX/MathJax, highlight.js, CSS, scroll sync options | 2026-11-17 |
| 18 | https://nvultra.com/help/preferences-editing.html | Smart pairs, type-over, smart return options, wiki links toggle | 2026-11-17 |
| 19 | https://nvultra.com/help/preferences-appearance.html | Fonts, print theme, typewriter height | 2026-11-17 |
| 20 | https://nvultra.com/help/preferences-advanced.html | Undo modes, coalescing, overscrolling, autocomplete dictionaries, expansion options | 2026-11-17 |
| 21 | https://nvultra.com/help/preferences-criticmarkup.html | CriticMarkup toggles, accept/reject all, initials, datestamps | 2026-11-17 |
| 22 | https://nvultra.com/help/preferences-saved-searches.html | Saved search management and hotkey assignment | 2026-11-17 |
| 23 | https://nvultra.com/help/preferences-announcements.html | In-app announcements toggle | 2026-11-17 |
| 24 | https://nvultra.com/help/FAQ.html | iCloud download caveat, versioning workaround, link clickability, folders-not-files | 2026-11-17 |
| 25 | https://nvultra.com/help/tips-and-tricks.html | Multi-document find/replace, Mermaid, Marked export, Print Preview floor | 2026-11-17 |
| 26 | https://nvultra.com/help/importing.html | Import paths for eight apps, OpenMeta conversion, Simplenote JSON caveat | 2026-11-17 |
| 27 | https://nvultra.com/help/conventions.html | Checked; documents the manual's own notation only | 2026-11-17 |
| 28 | https://nvultra.com/help/credits.html | Penney authorship, libSmartText/libSmartHighlighter, MultiMarkdown 6 credit, bundled libraries | 2026-11-17 |
| 29 | https://nvultra.com/help/changelog | Build history from initial beta to 2025.10.28.15.10; StoreKit/IAP entries; iCloud indexing entry | 2026-11-17 |
| 30 | https://brettterpstra.com/2019/04/10/codename-nvultra/ | Announcement, BitWriter origin, Penney invitation, multiple-folder motivation | 2026-11-17 |
| 31 | https://brettterpstra.com/2019/05/29/nvultra-update-the-beta-begins/ | Beta start, tester counts, Connections introduction | 2026-11-17 |
| 32 | https://brettterpstra.com/2019/06/07/nvultra-is-nvultra/ | Name origin (Fletcher's coinage), successor framing | 2026-11-17 |
| 33 | https://brettterpstra.com/2019/08/07/your-august-nvultra-update/ | Menu bar mode, Folder Navigator philosophy, no Simplenote, no RTF, folder-based sync | 2026-11-17 |
| 34 | https://brettterpstra.com/2019/09/25/nvultra-news-september/ | 1000+ testers, feature-complete claim, request deferral, two part-time guys | 2026-11-17 |
| 35 | https://brettterpstra.com/2020/04/01/your-april-nvultra-update/ | Beta capacity, saved searches addition, Fletcher's ER work | 2026-11-17 |
| 36 | https://brettterpstra.com/2021/03/23/an-overdue-nvultra-update/ | Rewrites, COVID impact, near-release claim | 2026-11-17 |
| 37 | https://brettterpstra.com/2022/05/20/ive-missed-you-too/ | Feature freeze, MAS IAP/subscription testing for free trial | 2026-11-17 |
| 38 | https://brettterpstra.com/2022/08/31/gather-to-nvultra/ | Gather clipper via URL handler, Fletcher as "nvUltra's developer", web clipper lobbying | 2026-11-17 |
| 39 | https://brettterpstra.com/2024/04/20/a-plea-to-nvalt-users/ | nvALT end of life, folder-of-Markdown requirement, no importer built | 2026-11-17 |
| 40 | https://brettterpstra.com/2025/12/29/bretts-favorites-2025/ | 2025 progress, beta "basically public", Marked 3 porting plan | 2026-11-17 |
| 41 | https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/ | nvALT 3 revival details, nvUltra status quotes, Marked Share boundary | 2026-11-17 |
| 42 | https://brettterpstra.com/2026/10/27/nvalt-gets-themeable/ | nvALT theming, nvUltra-compatible theme formats, "return to nvUltra" quote | 2026-11-17 |
| 43 | http://blakewatson.com/journal/why-i-chose-simplenote-over-standard-notes-and-nvultra/ | Community praise and complaints (2019 beta, republished 2024) | 2026-11-17 |
| 44 | https://talk.macpowerusers.com/t/nvultra-news-update/18869 | Forum thread: beta impressions, delay and pricing complaints, podcast pricing report | 2026-11-17 |
| 45 | https://news.ycombinator.com/item?id=40115264 | HN thread: beta access fatigue, alternatives | 2026-11-17 |
| 46 | https://writingslowly.com/2026/10/25/nvalt-is-back-from-the.html | Third-party coverage of the revival, migration sentiment | 2026-11-17 |
| 47 | https://hookproductivity.com/help/integration/using-hook-with-nvultra | Hookmark integration, URL scheme in the wild, reveal shortcut | 2026-11-17 |
| 48 | https://github.com/fletcher/MultiMarkdown-6 | MMD 6 capabilities, v6.8.0, deprecation toward MMD 7 | 2026-11-17 |
| 49 | https://multimarkdown.com/ | Penney's company and products, copyright years | 2026-11-17 |
| 50 | https://notational.net/ (via local snapshot research/raw/notational-net.html) | NV 2.5.2 released 2026-10-16, GPL, Simplenote lineage facts | snapshot captured 2026-10/11 |
| 51 | https://brettterpstra.com/projects/nvalt/ (via local snapshot research/raw/nvalt-project.html) | nvALT 3 behavior: storage, tags, search, Simplenote sync, editors, layout | snapshot captured 2026-11-16 |

Local seed snapshots read before searching: research/raw/nvultra-home.html, research/raw/nvalt-project.html, research/raw/brett-nvultra-topic.html, research/raw/notational-net.html.

## Dead-end log

- `www.reddit.com/r/bearapp/comments/wshg0g/...` and its `old.reddit.com` mirror: both fetches returned a login or landing shell with no thread content; the thread's existence and one-line gist are known only from search-result summaries, so it is not cited above.
- `https://nvultra.com/help/conventions.html`: documents only the manual's own notation (menu style, symbol table); no product conventions.
- `https://multimarkdown.com/`: marketing homepage only; no nvUltra mention and no version feature list. The MMD 6 feature detail came from the GitHub README instead [48].
- First URL harvest from the topic snapshot used an absolute-URL pattern and matched nothing (the page uses relative hrefs); resolved by listing href attributes directly.
- `blakewatson.com` first fetch returned a 301 to a trailing-slash http host; the refetch succeeded and is the URL cited [43].

## Unknowns

- Price, exact purchase tiers, and whether the subscription survives to release. Only the mechanism (MAS, IAP, subscription, free trial) is documented [29] [37].
- Release date. The site's modal says no concrete ETA; "shooting for 2026" is a hope, not a schedule [1] [41].
- iOS, iPadOS, or any non-macOS client: no public statement found. The indexing page's "especially on iOS" aside is the only trace, and it is not a commitment [6].
- MultiMarkdown 7 adoption: MMD 6 is deprecated upstream; nvUltra's credits still list 6, with no migration statement [28] [48].
- Current tester count and beta build number beyond the changelog's newest entry (2025.10.28) [29].
- The 2019 "feature complete for 1.0" claim cannot be reconciled with later rewrites from public sources; which of the 2019 and current feature sets maps to which build is unverifiable [34] [36].
- Undocumented internals fair to assume but not published: index file format details beyond SQLite, theme file formats for the editor (nvALT 3 accepts .theme/.plist/.json, but nvUltra's editor theme format is not specified on its pages) [6] [42].
- Reddit commentary beyond the search-result gist (login-walled; see dead-end log).

**Confidence:** high for the feature surface and philosophy (official manual and homepage, read in full, plus dated primary posts); high for the nvALT divergence (both apps' official pages captured); medium for community texture (forum threads and personal blogs, each single-source per claim); medium for status specifics that rest on one summarized changelog read (build identifiers) though the newest-build date is corroborated by the December 2025 post. Every load-bearing fact above carries a source; single-source facts are marked in text.
