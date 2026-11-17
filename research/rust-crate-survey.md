# Rust crate survey for Commonplace (GTK4 notes port)

- **Written:** 2026-11-17
- **As-of:** every version, date, license, and MSRV below was read from a live
  source on 2026-11-17 (crates.io API, docs.rs, upstream repo files). Nothing
  here is from memory.
- **Question as dispatched:** the late-2026 Rust ecosystem for the crates a
  GTK4 note-taking port will need, feeding the roadmap's DECISION gates
  (renderer Phase 6, trash Phase 4, file watching Phase 10, preview Phase 15,
  importers Phase 21, encryption gate Phase 22, editor highlighting Phases
  13-14, per `roadmap.md`). Every section ends with a recommendation and a
  risk note.
- **Method:** sub-question decomposition per area; crates.io structured
  endpoints for version/cadence/license/MSRV facts; docs.rs and upstream
  source for API semantics; two independent sources for load-bearing claims
  where they exist. Bracketed ids like [S1] resolve in the Sources table.
  Local context: `spec.md` and `roadmap.md` in this repo (read 2026-11-17).

## Summary table

| # | Area | Recommendation | Confidence |
|---|------|----------------|------------|
| 1 | Markdown to HTML renderer | **comrak 1.0.0** in the frontend crates (roadmap's standing pick confirmed) | High |
| 2 | File watching | **notify 8.2 + notify-debouncer-full 0.7**, app crate only | High |
| 3 | XDG trash | **trash 5.4** in the app crate, not hand-rolled in core | High |
| 4 | Encryption (gated Phase 22) | Keep the defer answer; if built: **age 0.12 + keyring 4** (dbus secret-service store); never gpgme | Medium |
| 5 | Readability + HTML to Markdown | **dom_smoothie (extract) + htmd (convert)**, CLI/app side only | Medium |
| 6 | WebKitGTK preview | **webkit6 0.6**; scroll save/restore via `evaluate_javascript` (there is no scroll API) | High on facts, Medium on effort |
| 7 | GtkSourceView editor | **sourceview5 0.11**; custom `.lang` via user dir or `LanguageManager` search-path append; stock `markdown.lang` ships upstream | Medium-high |

## 1. Markdown to HTML renderers

### comrak

- **Version/cadence:** 1.0.0, released 2026-11-17 (today; the release marks the
  project's tenth birthday). The 2026 run alone: 0.55.0 (09-06), 0.56.0
  (10-06), 0.57.0 (10-21), 0.58.0 and 0.59.0 (10-26), rc.1 (11-12), rc.2
  (11-16), 1.0.0 (11-17). Repo last push 2026-11-17. [S1][S4]
- **License/MSRV:** BSD-2-Clause; MSRV 1.89. The 1.0 release notes set a
  semver policy: MSRV bumps never in patch releases, minor releases may bump
  to a compiler at least six months old; option defaults never change except
  in majors. [S1][S4]
- **GFM coverage:** CommonMark 0.31.2 plus the five GFM extensions (tables,
  task list items, strikethrough, autolinks, tagfilter) with one removal at
  1.0: tagfilter (Disallowed Raw HTML) was removed outright after
  deprecation. Beyond GFM the 1.0 `Extension` options struct carries:
  `footnotes` and `inline_footnotes`, `description_lists`, `autolink`,
  `superscript`, `subscript`, `underline`, `highlight` (`==x==`), `insert`,
  `spoiler`, GitHub `alerts` (`> [!NOTE]`), Obsidian `ofm_callouts`,
  `multiline_block_quotes`, four math toggles (`math_dollars`,
  `math_dollars_block`, `math_latex`, `math_code`), `front_matter_delimiter`
  (MMD-style metadata blocks), `header_id_prefix`, and wikilinks in both
  orders (`wikilinks_title_after_pipe`, `wikilinks_title_before_pipe`, plus
  `wikilinks_ignore_embeds`). [S2][S4] That list alone covers parity-table
  rows 23 (metadata blocks), 25 (wikilinks), and half the Phase 15 math gate.
- **Sourcepos:** `Render.sourcepos`, doc verbatim: "Include source position
  attributes in HTML and XML output." Reliability per the same docs: good for
  core block items except lists and list items, all inlines, most extensions;
  the description-lists extension has known issues. The 1.0 cycle added a
  parse option to report sourcepos columns as Unicode character counts
  instead of UTF-8 byte offsets, and fixed multiline inline source positions.
  [S2][S3][S4] This is the direct mechanism the Phase 15 PreviewSync mapping
  wants: attributes land in the rendered HTML itself, so the scroll-sync
  layer can locate the block containing a caret line without a second parse.
- **HTML safety:** safe by default. README "Security" section: "Comrak will
  scrub raw HTML and potentially dangerous links"; to allow them you set the
  `unsafe` render option; with raw HTML allowed, "we recommend the use of a
  sanitisation library like `ammonia` configured specific to your needs."
  There is also `escape`, which escapes raw HTML instead of omitting it.
  [S3][S5] So the preview's MathJax/KaTeX injection (which needs raw script
  or the injected JS route) is the one place the app must pair `unsafe` with
  a sanitize pass or accept trusted-content-only input.
- **1.0 API shape:** Options family made `#[non_exhaustive]` with builders
  (`Options::builder()`); `Render` is no longer `Copy`; payload structs got
  constructors; optional deps (clap, syntect, etc.) no longer create implicit
  features, moved behind `cli`/`shortcodes` features. [S4]

### pulldown-cmark

- **Version/cadence:** 0.13.6 (2026-10-30); 0.13.x patch line through 2026
  (13.1 in February, 13.2/13.3 in March, 13.4 in May, 13.5 in October). MIT;
  MSRV 1.71.1. 211M downloads, the most-downloaded parser in the set.
  [S6]
- **Extension flags (0.13.6):** `ENABLE_TABLES`, `ENABLE_FOOTNOTES`,
  `ENABLE_STRIKETHROUGH`, `ENABLE_TASKLISTS`,
  `ENABLE_SMART_PUNCTUATION`, `ENABLE_HEADING_ATTRIBUTES`,
  `ENABLE_YAML_STYLE_METADATA_BLOCKS`,
  `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS`, `ENABLE_OLD_FOOTNOTES`,
  `ENABLE_MATH`, `ENABLE_GFM` (blockquote tags: `[!NOTE]` and kin),
  `ENABLE_DEFINITION_LIST`, `ENABLE_SUPERSCRIPT`, `ENABLE_SUBSCRIPT`,
  `ENABLE_WIKILINKS`. No bare-autolink toggle exists in the flag set.
  [S7] Coverage is narrower than comrak's but hits every GFM table/strike/
  tasklist requirement plus footnotes and wikilinks.
- **Source mapping:** no sourcepos in HTML output at all. The mechanism is
  `Parser::into_offset_iter()`, yielding `(Event, Range)` pairs where the
  range maps to the corresponding span of the markdown source. [S8] For
  scroll sync you would walk the offset iterator and emit your own
  position-bearing attributes while streaming to HTML: more code, more
  control, and no dependency on the renderer's attribute policy.
- **Raw HTML:** surfaces as explicit `Event::Html` / `Event::InlineHtml`
  variants (blocks arrive via `Tag::HtmlBlock`), so a consumer can filter or
  escape at the event level rather than trusting a render flag. [S9] The
  stock `push_html` treatment of those variants is not stated on the docs
  pages read (see Unknowns).

### Third options, honestly

- **markdown-rs** (`markdown` crate): 1.0.0, last release 2025-04-23; nothing
  in the 19 months since. MIT, MSRV 1.56. [S10] Not abandoned by any
  announcement, but the release evidence says slow, and its GFM extension
  surface has historically lagged comrak's.
- **markdown-it** (`markdown-it` crate): 0.6.1, last release 2024-07-07
  (over two years). [S10] Effectively dormant for new adoption.
- **MultiMarkdown 6 FFI:** no Rust bindings crate exists. crates.io search
  for "multimarkdown" and "mmd libmultimarkdown" returns nothing relevant
  (only an unrelated vertigo-cmark hit). Upstream `fletcher/MultiMarkdown-6`
  is alive (repo push 2026-07-29, README announces v6, version 6.8.0,
  copyright through 2026) and ships a C API (`libMultiMarkdown.h`), so the
  only route is DIY bindgen plus a C library build dep. [S11] Note: one
  search summary claimed MMD-6 was deprecated in 2023 in favor of an MMD-7;
  the upstream README contradicts that, and this report trusts the README.

**Transclusion:** none anywhere, as the dispatch expected. comrak's option
list has wikilinks but no expansion/embed execution (the wikilinks fields
only control parsing; `wikilinks_ignore_embeds` merely skips `![[...]]`
embeds) [S2]; pulldown-cmark's flag set has no transclusion notion [S7].
Roadmap row 26 already plans `{{transclusion}}` as a pre-render expansion
step in Commonplace's own code, which is the only correct shape regardless.

**Recommendation (Phase 6 gate):** keep comrak, pinned `1.0`, wired in the
frontend crates only, exactly as the roadmap's option 1 states. Its
sourcepos-in-HTML is the cheapest correct substrate for Phase 15's
PreviewSync; its extension set covers metadata blocks, wikilinks, and math
dollars that pulldown-cmark would force you to hand-roll; its
safe-by-default rendering matches an app that renders untrusted-ish note
bodies. pulldown-cmark remains the right pick only if the engine ever needs
a pull-parser event stream for a different consumer (the CLI exporter could
take it later without touching the GUI).

**Risk note:** comrak 1.0 landed this morning; pin `=1.0.0` or a tight
`1.0` and re-check the semver page before any bump. MSRV 1.89 is the
highest in this survey (notify 9's rc wants 1.91); confirm the toolchain
floor in CI the day the dependency lands. If the preview enables `unsafe`
for MathJax, the sanitize story (ammonia or strict template control) must
be decided in the same phase, not deferred.

## 2. File watching: notify

- **Current major:** stable is **8.2.2** (2026-11-08; 8.2.1 on 2026-10-25),
  CC0-1.0, MSRV 1.88 for 8.2.2. **9.0.0 is in release candidates**, at
  rc.9 (2026-11-15), MSRV 1.91, with a candidate series running since
  February 2026. [S12]
- **Rename semantics on Linux (read from the source, not the docs):** in
  8.2.2's inotify backend, `IN_MOVED_FROM` becomes
  `EventKind::Modify(ModifyKind::Name(RenameMode::From))` with the kernel
  rename cookie stored as the event tracker; `IN_MOVED_TO` becomes
  `RenameMode::To` with the same tracker. When the incoming cookie matches
  the stored From, notify emits a merged
  `Modify(Name(RenameMode::Both))` carrying both paths. [S13] So yes, the
  cookie pairing exists and produces a single Both event, with two
  qualifications:
  1. Both halves must be under watches (a recursive watch over the notes
     folder covers intra-folder renames, which is Commonplace's own rename
     flow). A move into or out of the watched tree arrives single-sided as
     To or From alone.
  2. v8 stores **one** pending From (`self.rename_event: Option<...>`), so
     interleaved multi-rename batches can mispair. The v9 line keeps a list
     of pending sources instead, fixing that case. [S13][S14]
  Additionally `MOVE_SELF` (the watched directory itself renamed) emits From
  only, with an upstream TODO to resolve the new path. [S13]
- **Content-write events:** `IN_MODIFY` maps to `Modify(Data(Any))` and
  `IN_CLOSE_WRITE` to `Access(Close(Write))`. [S14]
- **Atomic-rename saves (editors, and Commonplace's own writes):** the
  standard save pattern of write-temp-then-rename arrives as Create(temp)
  plus Modify(Name(To/Both)), not as a Modify(Data) event; the
  notify-debouncer-full README explicitly says it avoids emitting Modify
  events following a Create. Reconciliation should therefore key on the
  final path plus a re-stat, never on event-kind alone; this matches spec
  invariant 4 and the Phase 10 plan (reconcile stays the truth, watcher is
  a hint).
- **Debounce:** notify-debouncer-full 0.7.0 stable (2026-01-23; MIT OR
  Apache-2.0; MSRV 1.85), with 0.8.0-rc.4 (2026-11-15) tracking notify 9.
  README behaviors: "Only emits a single Rename event if the rename From
  and To events can be matched", merges multiple Rename events, updates
  paths of pending events that occurred before a rename, collapses
  duplicate Creates, and optionally stitches renames via a FileIdMap on the
  backends that need it (FSEvents, Windows). [S15] That inventory is
  precisely the noise-shaping a notes folder wants.
- **Raw inotify alternative:** the `inotify` crate (hannobraun/inotify-rs)
  0.11.5 (2026-08-14), ISC, MSRV 1.70, actively released through 2026.
  [S16] It is a thin idiomatic wrapper: you own watch-descriptor
  bookkeeping, path association, and cookie pairing yourself. Worth it only
  for a single-file, single-directory daemon with exotic needs; for a
  recursive notes folder it re-derives what notify already ships.

**Recommendation (Phase 10 gate):** notify 8.2 + notify-debouncer-full 0.7
in the app crate, core untouched (spec invariant 6). Subscribe expecting
`Modify(Name(Both))` for in-folder renames and single-sided From/To for
boundary crossings; treat every event as a hint into the reconcile pass.

**Risk note:** v9 is at rc.9 with MSRV 1.91; do not chase it for 1.0, but
write the watcher wrapper behind a tiny trait so a future 8-to-9 migration
(the debouncer 0.7-to-0.8 jump rides with it) is a one-file change. The
v8 single-pending-From mispairing is real but bounded: Commonplace's own
renames are serialized by the engine, and external mass renames (syncthing)
are exactly what the mtime+size reconcile pass exists to absorb.

## 3. XDG trash

- **Version/maintenance:** trash 5.4.0, released 2026-11-17; 5.3.0 on
  11-05, 5.2.9 on 09-13, 5.2.8 on 09-08, 5.2.7 on 09-03, 5.2.6 on 05-03.
  MIT, MSRV 1.87.0. Actively maintained through 2026. [S17]
- **Freedesktop compliance (read from `src/freedesktop.rs`):** [S18]
  - Home trash is `XDG_DATA_HOME/Trash`, falling back to
    `~/.local/share/Trash`, matching the spec.
  - For paths whose topdir is not the home, it selects the containing
    mount's topdir and uses the admin form `.Trash/<uid>` (checked) with
    the fallback `.Trash-<uid>`; on PermissionDenied it falls back to the
    home trash, which the spec's MUST clauses require.
  - Files land under `<trash>/files/`, metadata under `<trash>/info/` as
    `<name>.trashinfo`, with the DeletionDate written via the `jiff` time
    dependency.
  - Name collisions are handled by creating the info file exclusively
    (`create_new(true)`) and retrying with `.2`, `.3` suffixes, plus
    no-replace moves.
  - The trashinfo `Path=` value is percent-encoded (urlencoding crate,
    binary-safe) and decoded on read, with first-occurrence-of-key-wins
    per spec section 6.
  - One gap: the legacy octal-escaped trashinfo encoding is not
    implemented on read; only percent-encoded Path= is. That only matters
    when parsing trashinfo files written by decades-old implementations,
    which Commonplace never needs to do.
- **Dependency weight:** default features are `freedesktop` + `jiff`; on
  Linux that pulls libc, scopeguard, urlencoding, jiff, log. The zbus and
  rustix dependencies exist only behind the `portal` feature (Flatpak
  sandbox route), off by default. [S19] No async runtime, no dbus in the
  default build.

**Recommendation (Phase 4 gate):** use the `trash` crate in the app crate.
The roadmap's alternative, a hand-rolled in-core helper, is feasible (the
spec is short) but quietly owns cross-mount topdir detection, uid fallback
forms, collision suffixing, and exclusive-create semantics; the crate is
small, MIT, and exercised by its own release cadence. Keeping it out of
`commonplace-core` preserves the dependency-free engine invariant; the
engine's delete verb can accept an injected trash function the app wires.

**Risk note:** MSRV 1.87 is high for a utility dep (same class as comrak's
1.89); pin deliberately. Under Flatpak (Phase 23), verify whether the
sandbox exposes `~/.local/share/Trash` directly or needs the `portal`
feature; that test belongs in the packaging phase, not at first use.

## 4. Encryption context (gated Phase 22)

One-liners as dispatched; the roadmap already recommends answer (c) defer,
and nothing here changes that.

- **age (library crate):** 0.12.1 (2026-07-14), MIT OR Apache-2.0, MSRV
  1.74. Self-described beta ("all crate versions prior to 1.0 are beta
  releases for testing purposes only") but it is the standard Rust
  implementation of the age v1 spec: x25519 and post-quantum recipients,
  scrypt passphrases, optional SSH keys, armor, plugins, sync and stream
  APIs. The **rage** CLI lives in the same repo (str4d/rage) and files
  interop with the Go implementation. [S20] Best fit if per-note age
  encryption (roadmap option b) is ever chosen; it breaks tree grep-ability
  as the roadmap already notes.
- **gpgme (bindings):** 0.11.0, last crate release 2022-11-01 (four years
  ago); repo last push 2024-11-20. [S21] Dormant, plus a runtime dependency
  on the system gpgme/gpg-agent stack. No reason for a files-first notes
  app to carry it.
- **keyring:** 4.2.0 (2026-08-29), MIT OR Apache-2.0, MSRV 1.88, now under
  open-source-cooperative/keyring-rs with steady 2026 releases. v4 is an
  umbrella (v1-compat default feature) over `keyring-core` 1.0.1 plus
  per-store crates. On Linux the Secret Service route comes as
  `dbus-secret-service-keyring-store` 1.0.1 or
  `zbus-secret-service-keyring-store` 1.0.1 (both 2026-08-15), talking the
  Secret Service D-Bus API directly; the standalone `secret-service`
  crate sits at 6.0.0-rc.1 (2026-11-06). [S22] Note the mechanism is
  D-Bus, not libsecret-the-library; pick the sync-dbus or zbus flavor to
  match the app's async posture (GTK main loop, no tokio requirement).

**Recommendation (Phase 22 gate):** record defer (option c) with this
survey as the evidence that the building blocks (age + keyring) are alive
and permissively licensed whenever the gate reopens; if option (b) is ever
taken, age 0.12 for bytes and keyring 4 with the zbus secret-service store
for passphrase/key custody is the pairing.

**Risk note:** age's own beta framing means a 1.0 may still shift its API;
and per-note encryption forfeits plain-file tooling (grep, syncthing
dedup), which is the roadmap's stated reason to defer. gpgme should be
recorded as rejected-on-maintenance, so the question does not get
re-researched.

## 5. Readability extraction and HTML to Markdown (Phase 21 importers)

- **readability (kumabook):** 0.3.0, last release 2023-12-20; three years
  stale. [S23] Dead end; do not adopt.
- **dom_smoothie:** 0.19.0 (2026-10-27), MIT, MSRV 1.85 (0.18.x line was
  1.75). Active 2026 cadence (0.17 in March, 0.18.0 in June, 0.18.1/0.18.2
  in September, 0.19 in October). Readability-style content extraction;
  pure Rust; normal deps include dom_query, tendril, html-escape, gjson,
  phf, aho-corasick, thiserror, serde (12 direct). [S24]
- **htmd:** 0.5.6 (2026-10-15), Apache-2.0. Active 2026 cadence, 11.4M
  downloads. turndown.js-inspired HTML to Markdown; deps are three:
  html5ever 0.38, markup5ever_rcdom 0.38, phf. [S25] Caveat read from the
  dependency's own docs: markup5ever_rcdom "is not intended to be a
  production-quality DOM implementation, and has not been fuzzed or tested
  against arbitrary, malicious, or nontrivial inputs." [S26] The input to
  it in this pipeline is post-extraction article HTML from a fetch the app
  performed, not arbitrary local files, but the sentence belongs in the
  risk column.
- **html2md:** 0.2.17 (2026-08-20), GPL-3.0+ (license-compatible with this
  repo, would be viral for any future relicense). Cadence is spotty:
  0.2.13 (2021), 0.2.14 (2022-12), 0.2.15 (2025-01), 0.2.16/0.2.17 (both
  2026-08-20). GitLab-hosted. [S27]
- **html2text:** 0.17.4 (2026-10-18), active, but renders plain text, not
  Markdown; wrong tool for the markdownify verb. [S27]

**Recommendation (Phase 21 gate):** dom_smoothie for the Readability pass
and htmd for HTML-to-Markdown, both wired in the CLI/app layer, never in
core. Both are pure Rust, permissively licensed, and released within the
last six weeks of this survey. For dependency-minimalists the roadmap's
other option (hand-rolled converter porting upstream HTMLToMarkdown
semantics) remains viable for the converter half, but the extraction half
is where hand-rolling actually hurts; do not hand-roll readability.

**Risk note:** no conversion-quality comparison was run in this survey
(maintenance and dependency signals only; see Unknowns), so gate Phase 21
on a fixture test: import two or three real saved pages and eyeball the
Markdown before committing to htmd. dom_smoothie's MSRV moved 1.75 to 1.85
inside 2026; its floor can move again.

## 6. WebKitGTK for GTK4 (webkit6)

- **Crate:** webkit6 0.6.1 (2026-03-11), MIT, bindings for the WebKitGTK
  6.0 (GTK4) API; depends on gtk4, soup3, javascriptcore6. Release history
  0.4.0 (2024-07), 0.5.0 (2025-08), 0.6.0/0.6.1 (2026-03). [S28] Repo
  (World/Rust/webkit6-rs on GNOME GitLab) last commits 2026-03-11,
  matching the release. [S29] The cadence is slow because it is a gir
  binding tracking a stable C API, not because it is dead; MSRV is not
  published in crates.io metadata.
- **JavaScript API (read from the generated bindings source):** there is
  **no `run_javascript` in the 6.0 API**; the GTK4 generation renamed it.
  What exists on WebView: `evaluate_javascript(script, world_name,
  source_uri, cancellable, callback)` and its
  `evaluate_javascript_future` variant, plus
  `call_async_javascript_function`(+future) for argumentized calls.
  Content loading: `load_html(content, base_uri)`, `load_bytes`,
  `load_alternate_html`. [S30] The dispatch's phrasing ("run_javascript/
  injection APIs") should be read as evaluate_javascript in this API
  generation.
- **Scroll position:** there is **no scroll getter or setter anywhere on
  WebView** in this binding [S30], and none in the C API it mirrors. The
  scroll story is JavaScript through and through: read
  `window.scrollY`/`scrollX` via `evaluate_javascript_future` before a
  content swap, and `window.scrollTo(...)` after the new document reports
  loaded. A swap via `load_html` builds a fresh document (scroll resets,
  no session history to restore from); a swap done by replacing the body
  from script in the already-loaded document keeps the scroll container
  alive. Restoring too early, before layout completes, gets overwritten;
  the restore belongs on load-finished, after layout, which is the same
  sequencing nvALT's PreviewSync solved on macOS.
- **Pain points for an in-process preview pane that swaps content and
  restores scroll:** (a) no native scroll API, so all position state makes
  a JS round trip and must be async-scheduled off the load signal;
  (b) `load_html` resets everything, so either accept save/restore on each
  swap or switch to in-document DOM replacement for the hot path (the
  Phase 15 slicing design, ±64KB windows, favors DOM replacement);
  (c) the futures variants are the ergonomic route from a GTK main loop;
  (d) keep a stable `base_uri` scheme for note pages so relative resource
  resolution and any injected JS state behave predictably. The workspace
  already runs a WebKitGTK GTK4 app (Viaduct), so the integration shape
  has in-house precedent.

**Recommendation (Phase 15 gate):** webkit6 0.6 as the preview surface,
with a small JS-bridge module owning evaluate_javascript calls (scroll
get/set, MathJax/KaTeX re-render hooks, anchor scrolling) and the
sourcepos-attributed comrak HTML as the document body.

**Risk note:** the binding's release cadence trails the C library's; check
webkit6-rs before any WebKitGTK system upgrade in CI, and pin the Flatpak
runtime (Phase 23) rather than relying on host WebKit. The scroll-restore
sequencing is the piece most likely to need iteration; budget a fixture
(long note, math blocks, images) that exercises restore-after-layout.

## 7. GtkSourceView bindings (sourceview5)

- **Crate:** sourceview5 0.11.3 (2026-11-14), MIT. Releases: 0.11.0
  (2026-02), 0.11.1/0.11.2 (2026-07), 0.11.3 (2026-11-14); repo (World/Rust
  sourceview5-rs) commits through 2026-11-14 including binding regenerations.
  [S31][S32] Actively tracking the gtk4-rs generation.
- **Custom `.lang` is a documented, supported path.** GtkSourceView's own
  language-definition intro (docs/lang-intro.md in the repo) states the
  search order: `~/.local/share/gtksourceview-5/language-specs/` for custom
  `*.lang` files, `/usr/share/gtksourceview-5/language-specs/` for the
  shipped ones, precisely `${XDG_DATA_HOME}/gtksourceview-${GSV_API_VERSION}/language-specs/`
  first, then each `${XDG_DATA_DIRS}` entry. [S33] The Rust binding exposes
  the full `LanguageManager` surface needed to do this in code:
  `set_search_path`, `append_search_path`/`prepend_search_path` (behind the
  crate's v5_4 feature), `language(id)`, `guess_language(filename,
  content_type)`. [S34] So Commonplace can either drop a
  `commonplace-markdown.lang` into the user dir (zero code) or ship it in
  a GResource and append the search path (no filesystem side effects,
  friendlier to spec invariant 4).
- **Stock markdown exists:** `markdown.lang` ships in GtkSourceView 5's
  data/language-specs (verified in the repo tree, 193 spec files).
  [S33] The custom file is for the bespoke additions: task-list spans,
  fence continuation highlighting, metadata blocks, wikilinks.
- **Ceiling to respect:** `.lang` definitions are declarative regex-context
  machines. nvALT's MarkdownHighlight was a programmatic scanner; some of
  its behaviors (the 300KB incremental model, marker cycling) live above
  the syntax layer anyway. The roadmap already splits this correctly: a
  pure highlight-model module in the engine crate, with sourceview wiring
  kept thin. The `.lang` file covers token coloring; it does not replace
  the model.

**Recommendation (Phases 13-14):** sourceview5 0.11 with a custom
`commonplace-markdown.lang` loaded via `LanguageManager` search-path append
from a GResource (falling back to the documented user dir), on top of the
engine's own highlight model for behavior.

**Risk note:** the GLib-regex context engine has real limits on overlapping
constructs; prototype the task-list-inside-list-inside-fence case in the
first week of Phase 13 rather than assuming the stock markdown.lang plus
contexts can express it. The v5_4 feature gate on append/prepend is a
non-issue on any current runtime but pin it consciously in Cargo.toml.

## Sources

All read 2026-11-17. crates.io endpoints carry version, date, license,
MSRV, and cadence unless noted otherwise.

| Id | URL | What it supported |
|----|-----|-------------------|
| S1 | https://crates.io/api/v1/crates/comrak and /comrak/versions | 1.0.0 today, BSD-2-Clause, MSRV 1.89, 2026 cadence |
| S2 | https://docs.rs/comrak/latest/comrak/options/struct.Extension.html | full extension field list incl. footnotes, wikilinks, math, metadata blocks |
| S3 | https://docs.rs/comrak/latest/comrak/options/struct.Render.html | sourcepos/unsafe/escape doc text verbatim |
| S4 | https://github.com/kivikakk/comrak/releases/tag/v1.0.0 | 1.0 semver policy, non_exhaustive/builders, tagfilter removal, char-column sourcepos |
| S5 | https://raw.githubusercontent.com/kivikakk/comrak/main/README.md | Security section, ammonia recommendation, char-count sourcepos CLI flag |
| S6 | https://crates.io/api/v1/crates/pulldown-cmark and /versions | 0.13.6 (10-30), MIT, MSRV 1.71.1, cadence |
| S7 | https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html | complete ENABLE_ flag list |
| S8 | https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Parser.html | into_offset_iter, (Event, Range) |
| S9 | https://docs.rs/pulldown-cmark/latest/pulldown_cmark/enum.Event.html | Html/InlineHtml raw variants, HtmlBlock tag |
| S10 | https://crates.io/api/v1/crates/markdown and /markdown-it | markdown-rs 1.0.0 last 2025-04-23; markdown-it 0.6.1 last 2024-07-07 |
| S11 | https://raw.githubusercontent.com/fletcher/MultiMarkdown-6/develop/README.md and https://api.github.com/repos/fletcher/MultiMarkdown-6 | v6.8.0 alive, push 2026-07-29; no Rust bindings exist (crates.io search) |
| S12 | https://crates.io/api/v1/crates/notify and /versions | stable 8.2.2 (11-08), CC0, MSRV 1.88; 9.0.0-rc.9 (11-15), MSRV 1.91 |
| S13 | https://docs.rs/crate/notify/8.2.2/source/src/inotify.rs | v8 cookie pairing into RenameMode::Both; single pending From; MOVE_SELF TODO |
| S14 | https://raw.githubusercontent.com/notify-rs/notify/main/notify/src/inotify.rs | v9 Vec-based pairing; IN_MODIFY/IN_CLOSE_WRITE mappings |
| S15 | https://raw.githubusercontent.com/notify-rs/notify/main/notify-debouncer-full/README.md | rename merging, path updating, FileIdMap; 0.8 uses notify 9 |
| S16 | https://crates.io/api/v1/crates/inotify | 0.11.5 (08-14), ISC, MSRV 1.70 |
| S17 | https://crates.io/api/v1/crates/trash and /versions | 5.4.0 today, MIT, MSRV 1.87.0, cadence |
| S18 | https://raw.githubusercontent.com/ArturKovacs/trash/master/src/freedesktop.rs | home/per-volume trash selection, info/files split, exclusive-create suffixes, percent-encoded Path=, no octal variant |
| S19 | https://raw.githubusercontent.com/ArturKovacs/trash/master/Cargo.toml | default features freedesktop+jiff; portal feature gates zbus/rustix |
| S20 | https://crates.io/api/v1/crates/age and https://docs.rs/age/latest/age/ | 0.12.1, beta statement, feature set, rage interop |
| S21 | https://crates.io/api/v1/crates/gpgme and https://api.github.com/repos/gpg-rs/gpgme | last release 2022-11-01, last push 2024-11-20 |
| S22 | https://crates.io/api/v1/crates/keyring (+keyring-core, store search) and https://raw.githubusercontent.com/open-source-cooperative/keyring-rs/main/README.md | 4.2.0 architecture, Linux store crates 1.0.1, secret-service 6.0.0-rc.1 |
| S23 | https://crates.io/api/v1/crates/readability | 0.3.0, last 2023-12-20 |
| S24 | https://crates.io/api/v1/crates/dom_smoothie (+0.19.0/dependencies) | 0.19.0 (10-27), MIT, MSRV moves, dep list |
| S25 | https://crates.io/api/v1/crates/htmd (+0.5.6/dependencies) | 0.5.6 (10-15), Apache-2.0, html5ever+rcdom+phf |
| S26 | https://docs.rs/markup5ever_rcdom/latest/markup5ever_rcdom/index.html | "not intended to be a production-quality DOM implementation" |
| S27 | https://crates.io/api/v1/crates/html2md and /html2text | html2md GPL-3.0+ cadence; html2text plain-text scope |
| S28 | https://crates.io/api/v1/crates/webkit6 and /versions | 0.6.1 (03-11), MIT, history |
| S29 | https://gitlab.gnome.org/api/v4/projects/World%2FRust%2Fwebkit6-rs/repository/commits | last commits 2026-03-11 |
| S30 | https://gitlab.gnome.org/World/Rust/webkit6-rs/-/raw/main/webkit6/src/auto/web_view.rs | evaluate_javascript(+future), call_async_javascript_function(+future), load_html/bytes/alternate; no scroll methods |
| S31 | https://crates.io/api/v1/crates/sourceview5 and /versions | 0.11.3 (11-14), MIT |
| S32 | https://gitlab.gnome.org/api/v4/projects/World%2FRust%2Fsourceview5-rs/repository/commits | commits through 2026-11-14 |
| S33 | https://gitlab.gnome.org/GNOME/gtksourceview repository: data/language-specs tree and docs/lang-intro.md (master, via API raw) | markdown.lang ships; search path incl. user dir |
| S34 | https://docs.rs/sourceview5/latest/sourceview5/struct.LanguageManager.html | set_search_path, append/prepend (v5_4), language(id), guess_language |
| L1 | /home/bdkl/.gitrepos/Commonplace/spec.md and roadmap.md (local) | decision gates, parity rows, phase numbering |

## Dead-end log

- crates.io search for MultiMarkdown bindings ("multimarkdown", "mmd
  libmultimarkdown"): no crate exists; only an unrelated vertigo-cmark
  hit. The MMD6 FFI answer rests on this absence plus the upstream repo's
  own README.
- A web search summary asserted MMD-6 was deprecated (June 2023) in favor
  of MMD-7. The upstream README contradicts it (v6.8.0, active copyright
  2026, repo pushed 2026-07-29). Recorded as a refuted second-hand claim;
  primary sources won.
- api.github.com/repos/ArturKovacs/trash returned a null body once (rate
  remaining was 49, so not throttling; transient). Worked around via
  raw.githubusercontent file fetches; crates.io history supplies the
  maintenance evidence.
- raw.githubusercontent.com/notify-rs/notify/main/notify/src/inotify/backend.rs
  404 (v9 flattened the path to src/inotify.rs). Resolved via the repo tree
  API and docs.rs' source browser for the v8.2.2 file.
- docs.rs/webkit6/trait.WebViewExt.html 404. Resolved via the repo's
  generated auto/web_view.rs, which is the authoritative surface anyway.
- gnome.pages.gitlab.gnome.org langdef URLs 404 on direct fetch (stale
  search links). Resolved via docs/lang-intro.md read from the repo
  through the GitLab files API.
- notify's RenameMode docs page is silent on which backends emit Both
  (API docs only). Resolved by reading both the v8.2.2 and main inotify
  backend sources.
- No WebKitGTK-specific document on load_html scroll behavior was found;
  the fresh-document scroll reset is stated here as general webview
  behavior, not as a WebKitGTK-documented fact.

## Unknowns

- comrak's lib-only dependency footprint was not build-verified; the
  crates.io endpoint lists clap/syntect among dependencies while the 1.0
  notes move them behind `cli`/`shortcodes` features. A `cargo tree -e
  normal` on a scratch project settles it in one command at Phase 6.
- pulldown-cmark offset Range units (bytes vs chars) are not stated on the
  docs page read; bytes is the ecosystem convention but unconfirmed here.
- Whether the stock pulldown-cmark `push_html` escapes or passes
  `Event::Html`/`InlineHtml` through is not stated on the pages read;
  consumer-side event filtering is the safe assumption either way.
- notify 9's stable date is unknowable (rc.9 as of today); the migration
  cost note is written against that uncertainty.
- trash under Flatpak sandboxing (portal feature vs direct home trash) is
  untested; flagged to Phase 23.
- dom_smoothie vs htmd output quality against real article fixtures was
  not measured; adoption signals are maintenance and license only.
- The GtkSourceView `.lang` regex engine's ability to express the full
  bespoke flavor (task spans inside fences inside lists) is untested;
  flagged as the first Phase 13 prototype.
- pulldown-cmark bare-URL autolink behavior: no ENABLE flag exists in
  0.13.6 (verified); whether the parser handles bare autolinks natively
  was not established.
- ammonia's current version and maintenance were not checked in this
  pass; it is named from comrak's own README recommendation only.

Confidence: high on renderer, watching, trash, webkit6, and sourceview5
facts (primary sources read directly, two-source where possible); medium
on the encryption and importer recommendations (facts solid, choices
gated on usage this survey could not exercise).
