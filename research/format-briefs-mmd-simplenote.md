# Format briefs: MultiMarkdown metadata blocks and the Simplenote export archive

Date: 2026-11-17. All web sources read 2026-11-17; local sources (the nvALT clone, the nvUltra help snapshots) read from disk the same day. Written for the Phase 5 metadata store and the Phase 21 Simplenote importer (spec.md parity rows 23 and 46). Every grammar rule below cites where it is documented or, where the docs are silent, where the rule was read out of the implementation; implementation-derived rules are tagged `[impl]` and the doc silence is stated.

## The question as dispatched

Two file-format specifications Commonplace must parse byte-exactly, with exact grammar rules for its golden tests: (1) the MultiMarkdown metadata block from Fletcher Penney's primary documentation, including placement, termination, key and value syntax, standard keys, MMD 6 vs MMD 7, the MMD-block vs YAML-frontmatter distinction, and how `Title:` interacts with the first-line-as-title convention; (2) the Simplenote export archive from official app sources: zip layout, JSON record schema per note, the markdown-vs-plaintext distinction, and which fields a one-way importer should honor.

## Summary of the two formats

**MultiMarkdown metadata** is a block of `key: value` lines that must be the very first thing in the file, with no blank line before or inside it; the first blank line ends it. Keys start with an ASCII letter or digit and continue with letters, digits, spaces, hyphens, and underscores per the docs (the v6 and v7 scanners additionally allow dot, tab, and NBSP `[impl]`). Keys are matched case-insensitively with all spaces stripped. Values run to end of line and may continue onto following lines; any following line that contains no colon is absorbed as a continuation, which is the rule the docs state only sideways (indent continuations; a colon-bearing continuation must be indented). An optional `---` may fence the block before and after (closer may be `...`) for YAML compatibility. There is no `Tags` or `Keywords` standard key in MMD itself; nvUltra defines `tags:` as a comma-separated list. MMD 6 and MMD 7 implement the same line grammar; the v7 user guide's metadata section is a TODO stub, so v6's guide remains the canonical wording. In nvALT the note title is the first line verbatim, so a note beginning `Title: Foo` lists as "Title: Foo"; nvALT never parses `Title:` for naming. Whether a metadata `Title:` overrides the displayed name in nvUltra is not documented anywhere public; it is a Commonplace design decision, not a fact to copy.

**The Simplenote export** is either a zip (web and desktop: `notes.zip`) or a bare JSON file (Android: `simplenote.json`). The zip contains `source/notes.json` plus one `.txt` per active note at the root and one per trashed note under `trash/`, with the filename derived from the note's first non-blank line (sanitized, 40-char cap, ` (n)` dedup) and a `Tags:` block appended to each `.txt` body. `notes.json` is an object with `activeNotes` and `trashedNotes` arrays; each record carries `id`, `content`, `creationDate` and `lastModified` as ISO 8601 UTC strings, plus optional `pinned: true`, `markdown: true`, sorted `tags`, `publicURL` (`http://simp.ly/p/<code>`), and `collaboratorEmails`. Beneath this export shape sits the raw Simperium wire record (`content`, `creationDate`/`modificationDate` as epoch seconds, `deleted` as boolean or 0/1, `systemTags`, `tags`, `shareURL`, `publishURL`), which is the shape older archives and nvALT's sync code carry; the official importer's honor list is exactly `content`, `creationDate`, `deleted`, `markdown`, `modificationDate`, `pinned`, `tags`. There is no title field anywhere in Simplenote; titles are always derived from content. A field named `lastmdhelp` (named in the dispatch) does not exist in any official source.

---

## Part 1: MultiMarkdown metadata blocks

### 1.1 Placement and termination

Primary source: MultiMarkdown 6 User's Guide, metadata page (repo `fletcher/MultiMarkdown-6`, branch `gh-pages`, `syntax/metadata.md`, "Revised: 2018-06-26"; published at `https://fletcher.github.io/MultiMarkdown-6/syntax/metadata.html`). The MultiMarkdown-4 guide carries identical wording ("Revised: 2014-04-18"), so the grammar has been stable since at least 2014.

| Rule | Statement | Source |
|---|---|---|
| Placement | "The metadata must begin at the very top of the document - no blank lines can precede it." | MMD 6 guide, metadata.md |
| Leading blank lines | A file that starts with a blank line has no metadata block; the scan stops at the first empty line before any key line can start. | v7 `mmd_parser_hand_2.c:1905-1918` (`mmd_scan_metadata_lines` halts on `LINE_EMPTY`) `[impl]` |
| BOM | A UTF-8 BOM before the block is skipped, not treated as content. | v7 `mmd_parser_hand_2.c:1908` (`skip_bom`) `[impl]`; undocumented in the guide |
| Termination | "After the metadata is finished, a blank line triggers the beginning of the rest of the document." | MMD 6 guide |
| Termination (impl) | The block consumes lines until the first blank line; only the document start admits metadata at all (`allow_meta` goes false after the first block). | v7 `block_meta` at `mmd_parser_hand_2.c:737-743`, `allow_meta = 0` at `:1333` and `:1485`; v6 `parser.y:129-130,324-330` `[impl]` |
| Optional fences | "There can optionally be a `---` on the line before and after the metadata. The line after the metadata can also be `...`. This is to provide better compatibility with YAML, though MultiMarkdown doesn't support all YAML metadata." | MMD 6 guide (and MMD 4 guide, same words) |
| Fence (impl) | v6's `scan_meta_line` accepts a leading line of three or more hyphens before the first key line; v7 reclassifies a leading `---` line as YAML-mode metadata and ends that mode at a blank line or a closing `---`. | v6 `scanners.re:438`; v7 `mmd_parse_metadata` (`LINE_SETEXT_2` reclassified to `LINE_YAML`) and `block_meta_yaml` at `mmd_parser_hand_2.c:745-751` `[impl]` |
| Mid-document | A `key: value` line after the first blank line is ordinary paragraph text, not metadata. | v7 `mmd_parser_hand_2.c:1185-1186,1453-1454` (LINE_META handled only while `allow_meta`) `[impl]` |
| Compatibility mode | `-c` compatibility mode disables metadata parsing entirely. | v7 `read_ctx.c:58` (`allow_meta` false under `MMD_OPTION_COMPATIBILITY`); v7 user guide "Compatibility mode" bullet `[impl]` |
| Bare URLs | A line that is a URL or email (`scheme://...`, `mailto:`) is never metadata even though it contains a colon; the URL pattern outranks the meta pattern in the scanner. | v6 `scanners.re:198` ("meta_line can't match url above"); v7 `mmd_line_scanner.re:91-97` `[impl]` |

### 1.2 Key grammar

| Rule | Statement | Source |
|---|---|---|
| Start of line | "The metadata key must begin at the beginning of the line." | MMD 6 guide |
| First character | "It must start with an ASCII letter or a number." | MMD 6 guide |
| Following characters | "then the following characters can consist of ASCII letters, numbers, spaces, hyphens, or underscore characters." | MMD 6 guide |
| Scanner charset | The v6 and v7 scanners accept, after the first character: `A-Za-z0-9`, underscore, space, NBSP (0xA0), tab, hyphen, and dot. The docs understate the set (dot, tab, NBSP are undocumented). | v6 `scanners.re:194`; v7 `mmd_line_scanner.re:95` `[impl]` |
| Terminator | "The end of the metadata key is specified with a colon (':')" | MMD 6 guide |
| Whitespace around the colon | Whitespace is allowed between key and colon and between colon and value in both scanners (`s* ':' s*`). | v6 `scanners.re:198`; v7 `mmd_line_scanner.re:97` `[impl]`; the guide's examples show aligned `Title:    value` with wide gaps |
| Case and spaces | "Metadata keys are case insensitive and stripped of all spaces during processing. This means that `Base Header Level`, `base headerlevel`, and `baseheaderlevel` are all the same." | MMD 6 guide |
| Normalization (impl) | The stored key is the lowercased input with spaces dropped, `. _ - :` and alphanumerics kept, and multi-byte UTF-8 sequences copied through unchanged. | v7 `html_id_from_text`, `mmd_span_parser.c:1379-1425` `[impl]` |
| Empty value | A key with no value (`tags:` alone) is a legal metadata line. | v6 `scanners.re:200` (`empty_meta_line`); v7 `mmd_line_scanner.re:98` `[impl]` |
| Duplicate keys | Metadata is stored "first come, first served": the first occurrence of a key wins; later ones are ignored. Documented for v7's prepend-metadata feature; the v6 guide does not state it. | v7 `docs/user.md` "Prepend Metadata" section |

### 1.3 Value grammar

| Rule | Statement | Source |
|---|---|---|
| Character set | "After the colon comes the metadata value, which can consist of pretty much any characters (including new lines)." | MMD 6 guide |
| First character | In both scanners the value must begin with a non-whitespace character (after the optional spaces following the colon). | v6 `scanners.re:196`; v7 `mmd_line_scanner.re:96` (`meta_value = [^ \t\n\r] [^\n\r]*`) `[impl]` |
| Multi-line values | "To keep multiline metadata values from being confused with additional metadata, I recommend indenting each new line of metadata." Indentation is a recommendation, not a requirement. | MMD 6 guide |
| Colons in values | "If your metadata value includes a colon, it must be indented to keep it from being treated as a new key-value pair." Read together with the previous rule, this implies the positive rule the guide never states outright: a following line with no colon continues the current value, indented or not. | MMD 6 guide (implication); v7 `mmd_token_scanner.re:549-553` makes it explicit: a continuation is any following line that is indented, or contains no colon, or whose only colon is inside a URL or email `[impl]`; v6 absorbs such lines as `LINE_CONTINUATION`/`LINE_FALLBACK` in `parser.y:329-330` `[impl]` |
| Quoted values | Quotation marks have no special meaning; a value is stored verbatim, quotes included. No rule in any guide or scanner treats quotes specially. | Absence verified in v6 `scanners.re` and v7 `mmd_token_scanner.re` `[impl]`; not addressed in the guides |
| Whitespace normalization | Newlines and whitespace runs inside a value collapse to single spaces; one trailing space is trimmed; a backslash at end of line becomes a literal newline in the value. | v7 `clean_meta_value`, `mmd_token_scanner.re:485-536` `[impl]`; undocumented in the guide |
| Lists | Comma-separated lists are a per-key convention, not grammar. The MMD guides never define list syntax; nvUltra defines `tags:` as "a comma-separated list of tags." | nvUltra help, "Tagging and Metadata" (local snapshot `research/raw/nvultra-help-tagging-and-metadata.html`) |
| Markup | "Metadata is processed as plain text, so it should not include MultiMarkdown markup." | MMD 6 guide |
| Trailing spaces | "While not required, I recommend using two spaces at the end of each line of metadata" so the block still renders as a line-broken block under plain Markdown processors. | MMD 6 guide |

### 1.4 Standard keys

The MMD 6 guide's "Standard" Metadata keys, in the guide's own order: Author, Affiliation, Base Header Level (with LaTeX/HTML/ODF Header Level flavors), Biblio Style, BibTeX, Copyright, CSS, Date, HTML Header, HTML Footer, Language, LaTeX Author, LaTeX Begin, LaTeX Config, LaTeX Footer, LaTeX Header, LaTeX Leader, LaTeX Mode, LaTeX Title, MMD Header, MMD Footer, ODF Header, Quotes Language, Title, Transclude Base.

Verdicts on the keys the dispatch named:

| Key | Status | Source |
|---|---|---|
| `Title` | Standard key ("Self-explanatory."). Used for output document titling. | MMD 6 guide |
| `Author`, `Date` | Standard keys. Author feeds LaTeX/ODF/RTF title pages; Date is "a date to be associated with the document." | MMD 6 guide |
| `Keywords` | Not a standard key in MMD 4, 6, or the v7 docs (checked: absent from all three key lists). No MMD tooling semantics; treat as an ordinary user key if encountered. | MMD 4 guide, MMD 6 guide, v7 `docs/user.md` (whose metadata key list is a TODO stub) |
| `Tags` | Not an MMD standard key. It is nvUltra's tagging convention: "Tags... are specified with a tags: line of metadata. This key can contain a comma-separated list of tags." Marked 2 lists it as [ULTRA] tier for exactly this reason. | nvUltra help snapshot; Commonplace spec row 23 |
| `CSS` | Standard key: "specifies a URL to be used as a CSS file for the produced document" (HTML output only). | MMD 6 guide |
| `Base Href` | Not in the MMD 4 or MMD 6 standard key lists (checked both; grep for "Base Href" over both guides returns nothing). It is pre-MMD-4 folklore; no current documentation assigns it semantics. Explicitly flagged: if old notes carry it, it is an opaque user key. | MMD 4 guide, MMD 6 guide |
| `Transclude Base` | Standard key: sets the base folder for `{{transclusion}}` file resolution; defaults to the file's own folder. | MMD 6 guide |
| `Base Header Level` | Standard key: shifts the largest heading level (e.g. 2 makes `#` an `<h2>`); negative values legal for transclusion setups; format-specific flavors exist. | MMD 6 guide |

### 1.5 MMD 6 vs MMD 7

- MMD 7 was announced 2025-11 (fletcherpenney.net, "MultiMarkdown 7"): a rewrite for "cleaner code with better performance and cleaned up edge cases," still alpha, HTML and LaTeX output only so far. The announcement says nothing about metadata syntax changing.
- The v7 user guide's metadata section is literally "TODO include common keys." (`docs/user.md` line 516, at repo commit `ea4001c`, 2026-11-17.) The v6 guide remains the canonical wording.
- The v7 line-level grammar matches v6's (`meta_key` and `meta_value` classes are character-for-character identical across v6 `scanners.re:194-196` and v7 `mmd_line_scanner.re:95-96`), so a parser pinned to the v6 rules reads v7 metadata.
- One v7-only documented behavior: metadata can be prepended on stdin ahead of the file, and "first come, first served" means prepended keys win over in-file keys (v7 `docs/user.md`). Irrelevant to file parsing, relevant if Commonplace ever shells out to `multimarkdown`.

### 1.6 Interoperability: MMD block vs YAML frontmatter

Both shapes exist in the wild and Commonplace must read both (dispatch requirement). Sources: Obsidian "Properties" help page (read 2026-11-17 via rendered fetch, `help.obsidian.md/Editing+and+formatting/Properties`); MMD 6 guide; nvUltra importing page (local snapshot).

| Distinguishing rule | MMD metadata block | YAML frontmatter (Obsidian) |
|---|---|---|
| Opening delimiter | None required; the block simply starts at byte 0 with a key line. | Mandatory: `---` must be the first line ("Type `---` at the very beginning of a file"). |
| Closing delimiter | None; the first blank line ends the block. Optional trailing `---` or `...`. | Mandatory closing `---`. |
| Key charset | Letters, digits, spaces, hyphens, underscores (docs); plus dot/tab/NBSP (scanners). Spaces in keys are idiomatic (`Base Header Level`). | YAML mapping keys; Obsidian says "For the name, you can choose anything you like." Default properties: `tags`, `aliases`, `cssclasses` (singular `tag`/`alias`/`cssclass` deprecated in 1.4, support dropped in 1.9). |
| Key matching | Case-insensitive, spaces stripped. | Case-sensitive YAML keys; each name unique within a note ("you can't have more than one `tags` property"). |
| Values | Untyped strings to end of line plus continuations; quotes meaningless. | Typed YAML: text, number, checkbox (`true`/`false`), date (`2020-08-21`), date-and-time, lists (hyphen + space per line). Internal links need quotes. A JSON object between the fences is also accepted and re-saved as YAML. |
| Tags | Not an MMD key; nvUltra convention `tags: a, b, c` (comma string). | First-class `tags` property, list form (`tags:` then `- journal` lines). |
| Ambiguity zone | A file starting `---`, then `Title: Foo`, then `---`, then a blank line parses as metadata under BOTH regimes (MMD's YAML-compat fences; Obsidian frontmatter). | Same file from the other side. |
| Divergence case A | File starts `tags: foo` with no fences: MMD metadata; Obsidian treats it as body prose (no fence, no properties). | |
| Divergence case B | File starts `---` and the fenced block contains YAML lists, nested maps, or typed values: Obsidian parses them as properties; MMD does not implement YAML ("MultiMarkdown doesn't support all YAML metadata") and parses the interior as flat `key: value` pairs, absorbing `- item` lines as colon-less continuations of the preceding key `[impl]`. | |
| Divergence case C | `...` as the closing line after metadata: accepted by MMD per both guides. Obsidian's documented closer is `---`; whether `...` closes Obsidian frontmatter is not documented. Explicitly ambiguous; golden tests should not guess (test the `---` closer only, or pin Commonplace's own rule). | |

A byte-exact reader for both: if line 1 is exactly `---`, parse a fenced frontmatter block (close on the next `---` line; content parsed as YAML-shaped but MMD-tolerant); otherwise, if line 1 matches the MMD meta-line grammar, parse an MMD block ending at the first blank line. The one collision the golden tests must cover is the fenced block that both regimes claim (Divergence-free case above): agree with MMD there, since Obsidian's typed interpretation adds nothing a plain-text reader needs.

### 1.7 Title metadata vs the first-line-as-title convention

Does the metadata block come before the first line? Yes, by definition: in MMD the block occupies the file's first lines, so a note beginning with metadata has `Title:` (or any key) as its literal line 1, and no prose first line before the blank terminator.

- **nvALT: metadata-blind title.** The title is the first line of content, verbatim, capped at 60 characters with a break at the last space inside a 10-character window past the cap, collapsing to "Untitled Note" when empty. There is no `Title:` special case anywhere in the derivation. (`NSString_NV.m:265-321`, `syntheticTitleAndSeparatorWithContext`; the 60 comes from the caller at `:325`.) A note beginning `Title: Foo` therefore lists as "Title: Foo" in nvALT's note list. The preview path hands the whole body to the MultiMarkdown renderer (`NSString_MultiMarkdown.m:70-81`), which consumes the metadata for the HTML head; the list title never consults it.
- **nvALT: tags are Finder tags, not metadata.** nvALT's tag store is macOS Finder/OpenMeta tags (`NoteObject.m` syncs `orderedLabelTitles` to the file; the "Sync macOS and MMD tags" bridge is nvUltra's feature). So nvALT never reads `tags:` lines. This is why spec row 23 is tiered [ULTRA], not [ALT].
- **nvUltra: MMD metadata is the tag store; the title story is undocumented.** nvUltra help: "Tagging your notes in nvUltra is done by using MultiMarkdown metadata," with `tags:` comma-separated and inline `#tags` also counted; metadata processing requires full MMD mode (the "Limit to Strict Markdown" checkbox disables it, leaving inline `#tags` working). The help's own example note begins with `tags:` and `title:` lines followed by an H1, and the note's filename matches the title value; but no public page states the rule that maps note names to filenames, metadata titles, or first lines. The URL-scheme page states only that `x-nvultra://make` without a title parameter uses "the first line of the text." (Local snapshots: `nvultra-help-tagging-and-metadata.html`, `nvultra-help-advanced-features.html`.)
- **Consequence for Commonplace:** the interaction of spec row 3 (first-line title, [NV]) with row 23 (MMD metadata store, [ULTRA]) is a genuine fork with no upstream authority to copy. The documented facts a golden test can pin without guessing: nvALT behavior (first line verbatim, including the `Title:` prefix); MMD behavior (the block is line 1; `title` is a plain string value). Which display title Commonplace shows when both exist is a [CP] decision to record in spec.md, and the golden tests should encode whichever rule is chosen, plus the nvALT-compatibility case as a fixture.

---

## Part 2: The Simplenote export archive

### 2.1 Where exports come from

Official help (simplenote.com/help, read 2026-11-17): export exists on "the web, Android, and desktop apps for Windows, Linux and macOS," with the macOS caveat that only the Electron build exports (the Mac App Store build does not; iOS has no account export at all, verified by absence in the simplenote-ios source). Web and desktop produce "a .zip file of all your notes"; Android's Settings export writes a JSON file defaulting to `simplenote.json`. The help also states: "The filename for each individual note is the first line of the note."

### 2.2 Zip layout

Source: `simplenote-electron` at commit `41fa2ce` (2026-11-16), `lib/utils/export/to-zip.ts` and `lib/utils/export/index.ts`; corroborated by the Standard Notes import doc ("the notes.json text file found within the folder labelled, 'source'") and the nvUltra importing page (unzip; "nvUltra does not currently read the JSON file with your tags").

| Entry | Content | Source |
|---|---|---|
| `source/notes.json` | `JSON.stringify({activeNotes, trashedNotes}, null, 2)`, line breaks platform-normalized. | `to-zip.ts:101-104` |
| `<name>.txt` (zip root) | One per active note. Body is the note content with an appended block: two newlines, `Tags:`, then two-space-indented comma-separated tag lines wrapped at 75 columns. Zip entry mtime = lastModified. | `to-zip.ts:55-75,106-115` |
| `trash/<name>.txt` | Same, for trashed notes. | `to-zip.ts:117-126` |
| `<name>` derivation | First non-blank line of content, trimmed, passed through `sanitize-filename`, truncated to 40 characters, duplicates suffixed ` (n)`. Default `untitled` when the note has no non-blank line. | `to-zip.ts:6,19-29,88-94` |

The saved archive is named `notes.zip` (`index.ts`, `const filename = 'notes.zip'`).

### 2.3 JSON record schema

Current-generation export (what `source/notes.json` and Android's `simplenote.json` contain). Sources: `lib/utils/export/export-notes.ts:9-38`, `lib/utils/export/types.ts`, `lib/utils/export/README.md` (the example record below), Android `PreferencesFragment.java:596-652`.

Container: a single JSON object, `{"activeNotes": [...], "trashedNotes": [...]}`. Trashed-ness is array membership, not a field.

| Field | Type | Presence | Notes |
|---|---|---|---|
| `id` | string | always | The Simperium entity key (Android: `getSimperiumKey()`). |
| `content` | string | always | Full note text; line endings normalized for the platform on export. |
| `creationDate` | string, ISO 8601 UTC with milliseconds | always | e.g. `2016-11-18T23:28:42.957Z`. Android formats epoch seconds as `yyyy-MM-dd'T'HH:mm:ss.SSS'Z'` in UTC (`Note.java:219-226`); desktop uses `Date.toISOString()`. |
| `lastModified` | string, ISO 8601 UTC | always | Renamed from the internal `modificationDate` on export. |
| `pinned` | boolean `true` | only when the note is pinned | Flattened from `systemTags` containing `pinned`. |
| `markdown` | boolean `true` | only when markdown is enabled | Flattened from `systemTags` containing `markdown`. This is the markdown-vs-plaintext marker: no other field distinguishes them. |
| `tags` | array of strings | only when non-empty | Sorted case-insensitively. Desktop additionally partitions email-shaped tags out into `collaboratorEmails` (`export-notes.ts:14-17`). |
| `publicURL` | string | only when published | `http://simp.ly/p/<publishURL code>`. Desktop emits it when `systemTags` has `published` and `publishURL` is set; Android emits it whenever the publish code is non-empty. Minor condition divergence, same output shape. |
| `collaboratorEmails` | array of strings | desktop only, when shared | Email-shaped tags on a shared note. |

Raw Simperium wire record (the internal Note shape that older archives and sync clients carry; also the field vocabulary of any pre-2017 flat export). Sources: `simplenote-electron/lib/types.ts:16-27`; nvALT `SimplenoteEntryCollector.m:134-193` (parses exactly these off the wire); Android `Note.java:45-53` property constants.

| Field | Type | Notes |
|---|---|---|
| `content` | string | |
| `creationDate` | number, epoch seconds | `types.ts` `SecondsEpoch`; nvALT converts with `dateWithTimeIntervalSince1970`. |
| `modificationDate` | number, epoch seconds | Note the name difference from the export's `lastModified`. |
| `deleted` | boolean or 0/1 | nvALT reads it as integer 0/1; the electron type allows both. |
| `systemTags` | array of strings | Vocabulary below. |
| `tags` | array of strings | |
| `shareURL` | string (optional) | Collaboration link; empty string when unset. |
| `publishURL` | string (optional) | Publish code, not the full URL. |

`systemTags` vocabulary across the official clients: `pinned`, `markdown`, `published`, `shared` (electron `types.ts:16`); plus `unread` (iOS `Note.m:186,205`); plus `preview` as an Android model constant (`Note.java:42`, a server-side content-preview artifact). Android additionally exposes `published`/`shared` through the same mechanism (`Note.m:173-186`). The dispatch's "lastmdhelp" field: grepped across simplenote-electron and simplenote-android; no such field exists in any official source. It is not a thing; the closest real fields are `lastModified` (export) and `modificationDate` (wire).

### 2.4 What a one-way importer should honor

The official importer is the authority: simplenote-electron's import path accepts a `.json` or `.zip` (for zips it looks for exactly `source/notes.json`), requires a top-level `activeNotes` array, accepts `lastModified` or epoch `modificationDate` interchangeably, and then keeps only seven fields.

Honor list (`lib/utils/import/index.ts:6-14`, `propertyAllowList`): `content`, `creationDate`, `deleted`, `markdown`, `modificationDate`, `pinned`, `tags`. Everything else (`id`, `publicURL`, `collaboratorEmails`, `shareURL`, `publishURL`) is dropped. `pinned` and `markdown` are folded back into `systemTags` (`:32-40`); ISO dates are converted to epoch seconds (`:42-45`); missing dates fall back creation-from-modification-from-now (`:48-51`); tags are sanitized by stripping `\r`, `\n`, commas, and whitespace, and empty results are dropped (`:59-71`); membership in `trashedNotes` becomes `deleted: true` (`:94-95`). Android's importer (`Importer.java:99-120`, `Note.fromExportedJson` at `Note.java:122-132`) reads the same six fields per record plus the array split.

Mapping onto Commonplace's plain folder:

| Simplenote field | Recommendation | Authority |
|---|---|---|
| `content` | The note body verbatim (re-derive the title; see below). | Official importer keeps content verbatim. |
| `tags` | Write a `tags:` metadata line, comma-separated (nvUltra convention; spec row 23's store). Apply the official sanitizer (strip newlines, commas, whitespace; drop empties) so the import never manufactures broken metadata values. | nvUltra tagging page; `import/index.ts:59-71`. |
| `pinned` | No plain-file MMD equivalent exists; nvUltra documents none. Either a `pinned: true` key as a [CP] convention or app-index-only pinning; this is a spec decision, not a format fact. | Absence of any pinned key in MMD and nvUltra docs. |
| `deleted` / `trashedNotes` | Skip by default (a one-way importer imports the living library; trash is not content). The official importer does re-create trashed notes as deleted, but Commonplace has no trash-in-import concept and spec invariant 7 (delete to trash) concerns app deletions. Skipping (or an explicit `--with-trash` lane) is the defensible default; record it in spec.md either way. | `import/index.ts:94-95`; Commonplace spec. |
| `markdown` | Choose `.md` vs `.txt` extension (spec row 2's extension set) rather than fabricating a metadata key; the flag has no MMD representation. | export-notes.ts flattening; spec row 2. |
| `creationDate` / `lastModified` | Preserve as file mtime and/or a `created:`/`modified:` metadata pair; the official round-trip keeps both. Which representation Commonplace writes is a [CP] decision (the folder-first invariant means mtime is the only representation other tools see for free). | `import/index.ts:42-51`. |
| `publicURL`, `shareURL`, `collaboratorEmails`, `id` | Drop. The official importer drops them; the links are dead the moment the account is. | `propertyAllowList`. |
| Title | There is no title field. Derive from content: the export's own filename rule (first non-blank line, sanitized, 40-char cap, ` (n)` dedup) matches Commonplace's NV-lineage first-line title well enough to reuse directly. | `to-zip.ts:19-29`; help article "The filename for each individual note is the first line of the note."; Android `Note.java:228-271` (title = first line when a newline falls within 200 chars, markdown heading markers stripped when markdown is on). |

### 2.5 Golden-test targets specific to Simplenote

- Container shapes: `{activeNotes, trashedNotes}` object (current) versus a bare array of wire records (legacy; field names per 2.3's second table, dates as epoch numbers, `deleted` as 0/1). The legacy container shape is not documented by Automattic today; it is reconstructed from the wire type and nvALT's parser, so an importer lane for it should be marked as inferred and tested against a fixture, not against a spec.
- Dates: ISO strings with milliseconds and `Z`; epoch numbers; the `lastModified`-vs-`modificationDate` rename; missing dates.
- Tags: email-shaped tags on desktop exports land in `collaboratorEmails`, not `tags`; tags containing commas or newlines cannot survive the official sanitizer.
- The `.txt` files in the zip carry the appended `Tags:` block; an importer reading the `.txt` files instead of `notes.json` (the nvUltra path) must strip that block or it becomes body text. The block's exact shape is pinned by `to-zip.ts:55-75` (blank line, `Tags:`, two-space indent, comma-space separated, wrapped at 75).

---

## Sources

| Source | What it supported | As-of |
|---|---|---|
| MMD 6 User's Guide, metadata page: `https://fletcher.github.io/MultiMarkdown-6/syntax/metadata.html` (source: `https://raw.githubusercontent.com/fletcher/MultiMarkdown-6/gh-pages/syntax/metadata.md`) | All documented grammar rules in 1.1-1.4; standard key list | Page revised 2018-06-26; read 2026-11-17 |
| MMD 4 guide, metadata page: `https://raw.githubusercontent.com/fletcher/MultiMarkdown-4/gh-pages/metadata.md` | Grammar stability since 2014; absence of Base Href/Keywords | Page revised 2014-04-18; read 2026-11-17 |
| MMD 4 example: `https://raw.githubusercontent.com/fletcher/MultiMarkdown-6/gh-pages/examples/metadata.text` | Canonical multi-line example (Comment key spanning lines) | read 2026-11-17 |
| MultiMarkdown-6 source, develop branch @ `f71254b` (2026-07-29): local clone `/tmp/zcode-research/mmd6` (`src/scanners.re`, `src/parser.y`) | v6 scanner patterns, optional leading `---`, continuation absorption | 2026-11-17 |
| MultiMarkdown-7 source, develop branch @ `ea4001c` (2026-11-17): local clone `/tmp/zcode-research/mmd7` (`src/mmd_line_scanner.re`, `src/mmd_token_scanner.re`, `src/mmd_parser_hand_2.c`, `src/mmd_span_parser.c`, `src/read_ctx.c`, `docs/user.md`) | v7 grammar `[impl]` rules, BOM skip, allow_meta gating, value normalization, TODO-stub status of v7 docs | 2026-11-17 |
| fletcherpenney.net MMD 7 announcement: `https://fletcherpenney.net/2025/11/multimarkdown_7` | v7 alpha status; no metadata-syntax change stated | posted 2025-11; read 2026-11-17 |
| Obsidian Properties help: `https://help.obsidian.md/Editing+and+formatting/Properties` (rendered fetch) | YAML frontmatter rules, property types, default and deprecated keys, JSON variant | read 2026-11-17 |
| nvUltra help snapshots (local): `research/raw/nvultra-help-tagging-and-metadata.html`, `nvultra-help-importing.html`, `nvultra-help-advanced-features.html`, `nvultra-help-getting-started.html` | nvUltra tagging rules, strict-mode caveat, Simplenote import advice, title-from-first-line URL rule, filename-based note names | local files; read 2026-11-17 |
| nvALT clone (local, read-only): `/home/bdkl/.gitrepos/Commonplace/.repos/nvalt` (`NSString_NV.m`, `NSString_MultiMarkdown.m`, `NoteObject.m`, `SimplenoteEntryCollector.m`, `SimplenoteSession.m`) | First-line title derivation; MMD preview pass-through; Finder-tag store; Simplenote wire record fields | read 2026-11-17 |
| simplenote-electron @ `41fa2ce` (2026-11-16): local clone `/tmp/zcode-research/simplenote-electron` (`lib/utils/export/*`, `lib/utils/import/*`, `lib/types.ts`, `desktop/menus/file-menu.js`) | Zip layout, export schema, wire Note type, importer honor list | 2026-11-17 |
| simplenote-android @ `7f54880` (2026-11-17, trunk 2.40): local clone `/tmp/zcode-research/simplenote-android` (`PreferencesFragment.java`, `models/Note.java`, `Importer.java`) | Android export shape, ISO date format, systemTag constants, title derivation, importer fields | 2026-11-17 |
| simplenote-ios @ `20d2afc` (2026-11-13): local clone `/tmp/zcode-research/simplenote-ios` (`Simplenote/Classes/Note.m`) | `unread` systemTag; absence of account export | 2026-11-17 |
| Simplenote help: `https://simplenote.com/help/` | Export surfaces, zip description, first-line filename rule, Android `simplenote.json`, import formats | read 2026-11-17 |
| Standard Notes import doc: `https://standardnotes.com/help/30/how-can-i-import-my-notes-from-simplenote` | Corroborates `source/notes.json` path inside the zip | read 2026-11-17 |

## Dead-end log

- Raw fetch of `raw.githubusercontent.com/fletcher/MultiMarkdown-6/master/documentation/metadata.md` 404'd: the repo's default branch is `develop` and the guide lives on `gh-pages`. Recovered by listing branch contents via the GitHub contents API.
- Obsidian help via WebFetch hit a redirect loop (`help.obsidian.md` to `obsidian.md`) and then returned an empty JS shell. Recovered via the session's rendered web reader.
- Unauthenticated GitHub code-search API returned no usable results for symbol location in MultiMarkdown-7. Recovered by shallow-cloning the repo and grepping locally.
- Grep for a `Base Href` or `Keywords` key across the MMD 4 and MMD 6 guides: no hits; both are absent from every current standard-key list.
- Grep for `lastmdhelp` across simplenote-electron and simplenote-android: no hits; the field does not exist.
- simplenote-ios contains no account-export code path (share sheet only); iOS contributes only the `unread` systemTag fact.
- The pre-2017 flat `notes.json` container shape has no current official documentation; only its record fields are sourceable (electron wire type plus nvALT's parser). Container treated as inferred.

## Unknowns

1. **nvUltra's title rule when metadata is present** is undocumented: no public page states how the displayed note name relates to the filename, a `title:` key, or the first prose line when a metadata block occupies the top of the file. The golden tests must pin Commonplace's chosen rule (spec row 3 vs row 23 interaction) rather than seek an upstream answer.
2. **Whether `...` closes Obsidian frontmatter**: undocumented on Obsidian's properties page. MMD accepts it as a closer per both guides.
3. **MMD value whitespace normalization and the backslash-at-EOL newline escape** are implementation behaviors of v7 with no documentation; v6's equivalent normalization was not traced to source in this pass (the continuation absorption was).
4. **BOM handling in MMD 6** was not verified in source; only v7's `skip_bom` was read.
5. **The legacy flat-array export container** (exact top-level shape of pre-2017 web exports) is unrecoverable from current official sources; if a user presents one, the importer lane is fixture-driven.
6. **The web app's zip internals today** are corroborated (help article, desktop source, Standard Notes) but the web client itself is not open for direct source citation.
7. **Pinned-state representation in a plain folder** has no upstream convention to copy (no MMD/nvUltra pinned key exists); the choice is Commonplace's.

Confidence: high on every rule that carries a guide citation or a read source line above; medium on reconstruction-adjacent items (legacy container shape, v6 normalization details), each marked as such where it appears.
