# House prior-art report: Commonplace DECISION gates vs. the sibling repos

Research note. Produced 2026-11-17 from a read-only survey of the
house's shipped GTK4 apps (Framework, Docket, Viaduct, Atrium, Quire),
answering the DECISION gates in `roadmap.md` with file:line evidence.

All paths absolute in the original survey; abbreviated here to repo
names. Framework is C+GTK4; Docket/Viaduct/Atrium/Quire are Rust on
gtk4 0.11 (Atrium/Viaduct with `v4_16`, Docket/Quire `v4_10`).

---

## Q1. WebKit for the live Markdown preview (Framework + Viaduct)

**Which generation / version.**
- Framework builds against `webkitgtk-6.0` (the GTK4 API generation),
  pinned >= 2.46 (`Framework/src/meson.build:26`), plus a configure
  probe locating WebKit's helper-process dir (`Framework/meson.build:29-38`).
- The Rust house binding is the `webkit6` crate 0.6 (workspace dep) in
  Viaduct (`Viaduct/Cargo.toml:38`, used at `Viaduct/viaduct/Cargo.toml:14`).
  For Commonplace (Rust) the house answer is `webkit6 = "0.6"` against
  webkitgtk-6.0.

**Content injection pattern.**
- `webkit_web_view_load_html(html, base_uri)`: Framework's wrapper is
  `fw_webview_load_html` at `Framework/src/fw-webview.c:650-687`. The
  critical trick (lines 677-686): the **base URI is a custom-scheme
  origin** (`framework-img://doc-<id>/`) so the document and
  scheme-handler-served images are same-origin; without it WebKit's
  mixed-origin rules silently drop image fetches.
- Same doc → markdown: Framework renders .md through md4c (vendored,
  GitHub dialect) with `MD_FLAG_NOHTML` so an untrusted .md cannot
  inject `<script>` into the JS-enabled WebView
  (`Framework/src/fw-reflow-document-md.c:1-11`). Directly relevant to
  previewing untrusted notes.
- Rust twin: `view.load_html(&final_html, base_uri)`:
  `Viaduct/viaduct/src/ui/article_renderer.rs:1752,1759` and
  `article_pane_view.rs:746`. Viaduct re-renders a full `load_html`
  per article selection; no in-place DOM swap.
- Post-load decoration is `evaluate_javascript`, not re-load:
  theme/typography CSS custom properties pushed as JS `setProperty`
  calls (`fw_webview_set_reading_style`, `fw-webview.c:792-823`);
  publisher-CSS toggle flips `stylesheet.disabled` in the DOM
  (fw-webview.c:826-848). If load is not done, JS is queued in
  `pending_*` fields and flushed at `WEBKIT_LOAD_FINISHED`
  (fw-webview.c:289-330).

**Scroll position read/track.**
- Injected user script + script-message handler, not polling: a
  passive 200 ms-debounced `scroll` listener (injected at DOCUMENT_END,
  top frame) computes `{anchor: id of the topmost rendered [id]
  element, scroll_y, frac}` and posts it via
  `window.webkit.messageHandlers.fwpos.postMessage(...)`: user script
  at `fw-webview.c:372-399`, handler wiring at 594-606, receiver
  `on_position_message` at 338-367.
- The latest JSON is cached host-side (`last_position`,
  fw-webview.c:52-54) because "a JS round-trip can't finish during
  dispose" (fw-webview.c:334-337): read synchronously at save time via
  `fw_webview_get_cached_position` (761-766). This is the house-proven
  answer to getting scroll out of WebKit synchronously.
- A rendered-box guard excludes non-rendered `<style id>` nodes from
  anchor picking (fw-webview.c:381-389), a gotcha worth carrying into
  preview anchors.

**Debounce of rendering.**
- Scroll *reporting* is debounced 200 ms in-page (above). Content
  *reload on file change* is debounced host-side: GFileMonitor
  CHANGES_DONE_HINT/CREATED → reset a `g_timeout_add(200ms)` source
  (`fw-window.c:2372-2395`, `reload_debounce_id`), firing
  `reload_document_idle` (2355-2370). Caveat: that monitor is wired
  only on the fixed-layout path (`fw_window_start_monitor` at
  fw-window.c:2719); the reflow/WebKit path has none yet (tracked as
  Framework Phase 21 rank 1, fw-window.c:2638-2640). Debounced WebKit
  reload is proven as machinery, but Framework itself has not shipped
  it on the WebKit path.
- Viaduct adds a bind-side debounce: thumbnail fetches wait 220 ms
  after row bind with a stale-row guard so fast scrolls spawn nothing
  (`timeline.rs:608-633`).

**In-place content swap with scroll preservation: proven?**
- Yes, as reload-with-restore, not DOM patching. Flow: state persists
  the cached `{anchor, scroll_y, frac}` JSON per file
  (`fw_window_save_state`, fw-window.c:2825-2846); after the new
  `load_html`, `fw_webview_restore_position` (735-759) runs JS that
  first tries `getElementById(anchor).scrollIntoView({block:'start'})`,
  falling back to `window.scrollTo(0, scroll_y)`; restore calls queue
  until LOAD_FINISHED via `pending_position` (296-299, 740-744). The
  stale-cache hazard is handled by clearing `last_position` at the
  start of each load_html (670-673).
- **No house precedent for true in-place DOM swap between markers**
  (nvALT's marker-comment trick). Framework's nearest analog is the
  id-anchor scheme: anchors are stable element ids, so full reload +
  anchor restore *behaves like* an in-place swap. That is the pattern
  to adapt; nobody has proven DOM patching.

---

## Q2. GtkSourceView editor pane (Docket)

Stack: `gtk4 = "0.11"`, `sourceview5 = "0.11"`
(`Docket/crates/docket/Cargo.toml:14-20`).

**Language / highlighting setup.**
- Bundled custom `.lang` spec, not a stock markdown lang: `docket.lang`
  is `include_bytes!`'d, written out at startup to
  `$XDG_DATA_HOME/docket/lang`, and the default LanguageManager's
  search path is extended BEFORE first buffer creation (managers
  snapshot search paths on first use):
  `Docket/crates/docket/src/styles.rs:1-70` (install 12-35, language
  38-40, scheme 43-45).
- Buffer construction: `sourceview5::Buffer::with_language(&lang)` with
  fallback to plain, then `set_style_scheme`:
  `Docket/crates/docket/src/main.rs:103-109`. Style scheme likewise
  bundled (`docket-dark.xml`, Kanagawa Dragon) and prepend_search_path'd
  (styles.rs:31-33).
- The lang file is regex-context based with map-to fallbacks so foreign
  schemes still render (`resources/styles/docket.lang:1-40`). Stock
  GtkSourceView ships `markdown.lang`, so
  `LanguageManager.default().language("markdown")` is the zero-work
  option; Docket proves the custom-lang path if NV-specific contexts
  are needed.

**Autosave.**
- **Not debounced: synchronous write per keystroke.** `wire_autosave`
  connects `buffer.connect_changed` and calls `autosave()` doing
  `std::fs::write(path, full_buffer_text)`:
  `main.rs:1175-1190` and 1060-1073. Re-entrancy handled with a
  `loading: RefCell<bool>` guard (main.rs:40-42): programmatic
  set_text sets it so the changed handler neither autosaves nor races
  the file monitor; whole-doc edits call autosave() explicitly
  (1035-1058).
- External-change side: `gio::FileMonitor::monitor_file`; since the
  buffer never holds unsaved state, external change always reloads
  with caret/scroll clamped: `main.rs:1192-1219`.
- House gap: no debounce-timer precedent in Docket itself. Debounce
  precedents live in Viaduct (150 ms search timeout,
  `timeline_view.rs:174-198`) and Framework (200 ms reload,
  `fw-window.c:2392-2394`). Phase 12's debounced autosave combines
  Docket's guard/monitor discipline with Viaduct's cancel-and-re-arm
  timer.

**Undo.**
- Buffer-native undo via `begin_user_action`/`end_user_action`, no
  custom stack: whole-document swap inside one undo group with caret
  clamp (`apply_edit`/`replace_document`, main.rs:1035-1058);
  completion insert as one undo step (`completion.rs:143-147`). Quire
  adds `begin_irreversible_action` for programmatic splices that must
  not be undo units (`Quire/crates/quire/src/page.rs`, RefSpliceGuard
  ~985ff, referenced 606-614). Phase 12's per-note undo *stacks* have
  no direct house precedent (all house editors are single-buffer);
  the grouping vocabulary (user action + irreversible action + signal
  blocking) is all proven.

**Keybindings wiring.**
- Two layers: (1) `EventControllerKey` with `PropagationPhase::Capture`
  on the view, `connect_key_pressed` for editing keys (Enter, Tab,
  Backspace-at-col-0, Ctrl-D, Ctrl-Alt-Up/Down...) returning
  `Propagation::Stop`: `main.rs:432-520`; (2) GIO actions +
  `set_accels_for_action` for command keys (~20 bindings,
  main.rs:1244-1494).
- Gotcha at main.rs:240-244: the view must `grab_focus()` at startup
  or "the key controller and app accels stay inert until the first
  click."

**Per-line/incremental highlighting given this code.**
- Docket recomputes whole-document state per change for folds
  (`refresh_folds`, main.rs:321-343, wired via connect_changed at
  300-308) and delegates painting to the GtkSourceView lang engine,
  which is inherently line-local/incremental. Tag-application
  precedent: `FoldEngine::reapply` removes the invisible tag over the
  whole buffer then re-applies per line range (`fold.rs:96-129`).
- The best house model for Phase 13's "only edited lines re-colored"
  is Quire's signature-guarded tag pass: compute a cheap signature of
  the per-line spans, skip teardown/rebuild when unchanged ("most
  bursts edit one line"), else `remove_tag_by_name` over the buffer
  then `apply_tag_by_name` per computed line span:
  `page.rs:952-1000` (`apply_error_highlights`). That is exactly the
  Phase 13 deliverable shape (pure ranges-from-buffer module + thin
  sourceview wiring), proven; the 300KB cap itself has no house
  precedent.

---

## Q3. Lists at scale (Viaduct + Atrium; Docket sidebar)

**Widget choice: `GtkListView`, not ColumnView.**
- Viaduct timeline: `gtk::ListView` + `gtk::SingleSelection` over a
  `gio::ListStore` + `gtk::SignalListItemFactory`:
  `timeline.rs:99-110` (`setup_timeline_list_view`); assembled in a
  CompositeTemplate at `timeline_view.rs:42-47`.
- Atrium task list: same trio, with a diff applier turning worker
  TaskChanges into in-place ListStore mutations "without a full
  reload: preserving selection, scroll, and animations":
  `task_list.rs:1-11`, factory builder 282-307. Grouped views
  deliberately stay on the single ListView ("harder to express through
  a single GtkListView": `window/lists.rs:55-56`).
- **No house repo uses GtkColumnView.** Docket's sidebar is a small
  `gtk::ListBox` rebuilt wholesale (`sidebar.rs:1-10`): fine at
  sidebar scale, not the note-list answer.

**Domain items as GObjects.**
- Viaduct wraps the domain Article in a `glib::wrapper!` GObject
  (`ArticleNode`) with `#[property(get,set)] read/starred` so factories
  subscribe to `notify::read` and restyle without waiting for a
  recycle: `timeline.rs:16-49, 81-88`; the notify wiring + clean
  connect_unbind disconnect at 366-405. Pattern for "selection
  auto-marks read → row restyles live."

**Selection + keyboard navigation.**
- `SingleSelection` with `set_autoselect(false)`, `set_can_unselect(true)`
  (timeline.rs:105-107): important so clearing the search field can
  deselect (Phase 11's empty-query contract). Deselect via
  `set_selected(gtk::INVALID_LIST_POSITION)`: `window.rs:1447`.
- Up/Down = `selection.set_selected(target)` then
  `list_view().scroll_to(target, ListScrollFlags::FOCUS | SELECT, None)`
  with boundary no-ops and INVALID_LIST_POSITION treated as "start
  from the ends": `window.rs:1104-1130` (`select_row`), plus
  unread-skip variant `advance_unread` (1135-1170). Direct precedent
  for Phase 11's Up/Down forwarding.

**Virtualization / factory reuse.**
- ListView gives it for free; both repos encode recycling discipline:
  Atrium clears all stale CSS classes on every bind
  (`AREA_ACCENT_CLASSES` wipe, task_list.rs:44-81), uses fixed
  setup-time widget order + `next_sibling` walks in bind instead of
  lookups (460-490), and notes "ListView only re-binds rows that
  visually move" (481-484). Viaduct stamps the article id onto the
  widget as a stale-row guard for async work (timeline.rs:346-363,
  guard 627-632) and disconnects notify handlers in unbind (393-405).

**Two-line rows + chips composition.**
- Viaduct row tree (setup at timeline.rs:112-227): row_hbox ←
  [thumb_picture | content_vbox(top_hbox(title|media icon+count),
  feed_name, preview 2-line) | date_label]. Label economics: preview
  `set_lines(2)` + `wrap(WordChar)` + `ellipsize(End)` +
  **`max_width_chars` cap because wrap=true makes natural width
  potentially huge** (timeline.rs:196-207); **`PREVIEW_MAX_CHARS = 400`
  hard cap before `set_text`** because "Pango shapes the *entire* text
  to compute line breaks before ellipsizing... a 60-KB article =
  hundreds of ms of shaping per row bind": the Digital Antiquarian
  regression: timeline.rs:495-566 (test at 750-765). For a notes list
  showing body previews of arbitrarily large notes, this cap is the
  load-bearing lesson.
- Atrium chips: per-tag pills via ONE Label with Pango markup
  (`<span foreground>` per pill color) rather than one widget per chip
  (task_list.rs:398-408), plus fixed-shape pills rendering at natural
  width while the title pays the ellipsis cost (387-436). Context chip
  pattern with `max_width_chars(40)` at 410-422. Atrium's in-row title
  editing uses a title Stack (Label page / Entry page):
  task_list.rs:338-367: which is the in-row-editing shape for
  Phase 17 without ColumnView cells.

---

## Q4. Live-input latency (Quire + Atrium + Viaduct + Docket)

**Quire: recompute-as-you-type on the buffer.**
- `buffer.connect_changed` → coalescing `Cell<bool>` guards (pending,
  tick_scheduled) → one `add_tick_callback` per burst; evaluation runs
  in the frame's update phase so "the answers land in the SAME frame
  as the typed text instead of trailing it (an idle at priority 200
  paints a frame behind the 120-priority echo frame)":
  `page.rs:606-680` (comment 619-628, handler 629-642).
- Highlight updates are signature-guarded (skip when unchanged):
  `apply_error_highlights`, page.rs:952-975.
- Tag application sits outside the undo stream; programmatic rewrites
  use `block_signal(handler)` + `begin_irreversible_action` via a Drop
  guard (page.rs:606-614).
- Cursor-line band rides `connect_changed` + `connect_mark_set` (insert
  mark) with an early-return when the line didn't change: 404-459.

**Type-to-filter lists.**
- Viaduct: `SearchEntry::connect_search_changed` → cancel previous
  `timeout_add_local_once` and re-arm a 150 ms timer before the FTS5
  query, with a comment that the `_once` handle must be cleared after
  firing or the next `.remove()` hits a dead id ("a GLib CRITICAL,
  fatal under G_DEBUG=fatal-criticals"): `timeline_view.rs:171-200`.
  Empty query → `view.clear()` (revert to sidebar selection).
- Atrium: uses the `GtkSearchEntry` built-in `search-delay` property
  (100 ms, set in .ui) so search-changed already fires debounced:
  `window/search.rs:81-83`, `window/sidebar.rs:289`. Empty query falls
  back to the Today list (84-97). It also parses the query on the UI
  thread per keystroke and defers only the DB SELECT to the debounce:
  the "cheap parse now, expensive query debounced" split.
- Docket's search field fires immediately with no debounce (parse +
  refilter per keystroke; sidebar rebuild per keystroke):
  `main.rs:1502-1555`. Works because TaskPaper files are small;
  Viaduct/Atrium are the latency-honest precedents. Docket's handler
  also carries the "deleting-into-a-completion is a deletion"
  last-text diff (1503-1514) and a completion-with-selected-suffix
  re-entry loop (1523-1539): direct prior art for Phase 11's
  completion-remainder insertion.

---

## Q5. Single instance, scheme handlers, WebKit isolation

**Single-instance activation.**
- GTK Application is unique-per-app-id by default (GApplication owns
  the D-Bus name); second invocations route as open/activate.
  Framework: `G_APPLICATION_HANDLES_OPEN` at `fw-application.c:233`.
- The house pattern to copy is Viaduct's (exactly Phase 18's shape):
  `.flags(gio::ApplicationFlags::HANDLES_OPEN)` with a comment that
  "DBusActivatable routes the second invocation to the primary
  automatically": `Viaduct/viaduct/src/main.rs:132-139`;
  `connect_open` extracts the URI, normalizes, then an async
  window-present retry loop (up to 50 × 20 ms) because a fresh launch
  may still be inside build_ui: main.rs:150-181; re-activation
  presents the existing window (304-308).
- Scheme registration: `MimeType=x-scheme-handler/feed;x-scheme-handler/feeds;`
  in the desktop file: `Viaduct/data/io.github.virinvictus.Viaduct.desktop:10`.
  (Docket is deliberately `NON_UNIQUE | HANDLES_OPEN`:
  main.rs:76: multi-window-by-design, the opposite posture.)
- No house repo registers an app-owned GDBus server or an in-code
  application-scheme handler (Viaduct's `viaduct-img://` schemes are
  WebKit URI schemes, not OS-level). OS-route evidence yes; in-app
  GDBus evidence no.

**WebKit process isolation / lockdown.**
- Viaduct `apply_locked_down_settings`:
  `article_renderer.rs:560-582`: JS off (+ javascript_markup off),
  WebGL/WebRTC off, html5 db/localStorage/offline-app-cache off, dev
  extras off, back-forward gestures off, JS-can't-open-windows, media
  needs user gesture. A separate per-purpose WebView gets its own
  profile: the video player is a JS-enabled WebView on the default
  context, distinct from the article renderer's locked-down one:
  `article_pane_view.rs:708-740`. For a Markdown preview (needs JS for
  math/mermaid/highlight.js), the Framework profile is the template:
  JS on, dev extras/html5-db/local-storage off
  (`fw-webview.c:610-616`).
- Framework ops notes in `main.c:133-164`:
  - WebKitGTK 6.0's bubblewrap sandbox cannot coexist with Landlock
    (bwrap's `mount("/", MS_SLAVE)` fails under a Landlock ruleset);
    posture was to keep Landlock and set
    `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1` (respecting a
    user-set value, `g_setenv(..., FALSE)`), set before any WebKit
    symbol is referenced.
  - `WEBKIT_DISABLE_DMABUF_RENDERER=1` fixes stale-buffer artifacts
    on Wayland scroll.
  - WebKit helper dirs are allowed exec under Landlock in
    `fw-sandbox.c:96-202`.
- Framework compiles a WebKitUserContentFilter (content-blocker JSON)
  blocking all http(s)/ws(s) subresources for local documents, cached
  under the user cache dir: `fw-webview.c:464-510`: plus a
  navigation-policy hook allowing only same-document fragment jumps,
  handing http/mailto to `GtkUriLauncher`, ignoring everything else
  (fw-webview.c:401-462). Both reusable for the preview pane (notes
  may contain remote links/images).
- Viaduct's network-sandbox alternative: rewrite
  `<img src="https://...">` to `viaduct-img://i/<encoded>` served by an
  ImageCache scheme handler, plus a CSP:
  `article_renderer.rs:26ff, 585-600`. Framework's scheme-handler
  threading rules (registry under mutex, lookups on the WebKit IPC
  thread, refs before unlock: `fw-webview.c:9-15, 129-178`) are the
  concurrency contract for any such handler.

---

## Recommendations for Commonplace's DECISION gates

**Phase 10 (list widget + watching).** Choose `GtkListView` +
`SingleSelection` + `SignalListItemFactory` over GtkColumnView: the
only list shape any house repo ships at scale; Phase 17's in-row
editing is already solved in-house with Atrium's title Stack (Label
page / Entry page, task_list.rs:338-367). Copy: domain items as
GObjects with notify-able properties; recycle discipline (clear stale
CSS per bind, sibling-walk, unbind-disconnect); the 400-char preview
cap before Pango as the body-preview contract; autoselect(false) /
can_unselect(true) so empty-query deselect works. Watching: Docket's
`loading` guard + "disk is the truth, always reload" monitor and
Framework's 200 ms debounce reset-per-event are the two halves of the
recipe.

**Phase 11 (search-field state machine).** Latency: Atrium's
`GtkSearchEntry` search-delay (100 ms) or Viaduct's cancel-and-re-arm
150 ms timer for the expensive filter, cheap parse per keystroke;
same-frame feedback via Quire's tick-callback coalescing. Reuse
Docket's field behaviors: completion-with-selected-suffix re-entry,
deletion-vs-completion diff via last-text, empty → clear+deselect;
Viaduct's set_selected(INVALID_LIST_POSITION) + scroll_to(FOCUS|SELECT)
for Up/Down forwarding.

**Phase 12 (editor pane).** GtkSourceView setup is a straight Docket
port (bundled lang + scheme extracted to XDG, search-path mutation
before first buffer); stock markdown lang first. Autosave: Docket
ships synchronous no-debounce autosave with the loading guard;
Commonplace notes can be far larger than TaskPaper files, so the
DECISION answer is debounced (Viaduct timer pattern, 250-500 ms) with
Docket's guard discipline and explicit autosave() on programmatic
edits; the debounce timing test has no house test precedent to copy.
Undo: buffer-native user-action groups + Quire's irreversible-action
Drop guard for programmatic splices; per-note undo stacks remain
unproven in-house (single-buffer apps): say so in the ledger.

**Phase 13 (highlighting + 300KB cap).** The split exists in-house:
pure ranges-from-buffer computation (Quire's headless quire-eval
crate) + thin sourceview wiring via signature-guarded whole-buffer tag
passes. No house repo does true per-edit-range incremental
application; the honest gate answer is "GtkSourceView engine for
baseline markdown + Quire-style guarded passes for ==highlight==/
CriticMarkup/task spans, with the 300KB cap enforced before the pass."
Keybindings: Docket's two-layer scheme (Capture-phase
EventControllerKey for editing keys, GIO accels for commands) with the
grab_focus()-at-startup fix.

**Phase 15 (preview pane).** WebKitGTK 6.0 via `webkit6 = "0.6"`.
Architecture proven by Framework: one WebView composited as a plain
GtkWidget; load_html with a custom-scheme base_uri for same-origin
resources; post-load JS queued until LOAD_FINISHED; scroll tracked by
an injected 200 ms-debounced user script posting {anchor, scroll_y,
frac} with host-side synchronous cache; reload = debounced full
load_html + restore_position (anchor-then-scrollTo fallback). In-place
DOM swap between markers: no house precedent; record Framework's
reload-with-anchor-restore as the Phase 15 baseline and treat DOM
patching as unproven. Sourcepos ↔ line math has no house analog:
net-new. Security posture: MD_FLAG_NOHTML-style raw-HTML policy or
Viaduct's img-scheme + CSP; nav-policy allowing only fragment jumps +
external links to GtkUriLauncher; optionally Framework's content
filter. Carry Framework's two env vars into packaged-run notes.

**Phase 18 (URL scheme + single instance).** The entire route is
proven end-to-end by Viaduct: desktop-file MimeType → HANDLES_OPEN →
connect_open normalizes and lands the URI on the running window with a
window-present retry loop → activation presents, never duplicates.
`commonplace://find/...` maps onto `feed://...` shape-for-shape;
`x-nvultra://find` is a second MimeType entry. No-evidence items for
the ledger: in-app GDBus service (none in house), scheme-handler
registration in code (desktop-file + GApplication route only), and
MathJax-vs-KaTeX (neither appears in any house repo).
