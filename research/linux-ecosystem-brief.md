# The nvALT-shaped niche on Linux: prior art survey and the 2026 modernization baseline

- Written: 2026-11-17. Every source was fetched live on this date; claims are dated
  by what the page itself states, otherwise by fetch date.
- Question as dispatched: a citation-backed survey of prior art for Commonplace
  (a Rust/GTK4 Linux port of the Notational Velocity line: search-becomes-creation
  over a folder of plain-text notes, notes ARE files, no database required), then a
  concrete gaps list and a modern-baseline checklist a 2026 Linux port must meet.
- Method. WebSearch was unavailable in this harness, so discovery rode keyless
  endpoints (GitHub and GNOME GitLab APIs, PyPI, HN Algolia) plus direct page
  fetches. The question decomposed into: (a) what each NV-lineage app actually does
  (file model, search UX, license, activity); (b) what the broader note-app field
  does to the notes-are-files principle; (c) what Linux/GTK/Rust note apps exist;
  (d) what users complain about; (e) what a 2026 platform baseline demands.
  Star counts and push dates below are GitHub/GitLab API values read 2026-11-17.

## Executive summary

1. The NV line is newly alive: nvALT 3.0 shipped 2026-10-24 and Notational
   Velocity itself is receiving updates again, so Commonplace enters a niche with
   fresh attention and an incumbency gap on Linux
   ([Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/)).
2. No native Linux GUI app implements search-becomes-creation over a folder of
   plain files today. nvpy (Python/Tk) is the closest descendant and is
   Simplenote-centric by design; the rest of Linux activity is editor plugins,
   TUIs, or database-backed GNOME apps (survey below, sections 1.2 to 1.7).
3. nvEnvy is the sharpest modern statement of the essential nvALT feature set:
   instant search with create-on-no-match, plain .md files with YAML frontmatter,
   wikilinks with autocomplete, a tag sidebar, live HTML preview with custom CSS,
   saved searches on hotkeys, and URL schemes
   ([nvEnvy README](https://github.com/kenm47/nvEnvy#readme)).
4. The field has already voted on storage: Zettlr, Notable, Zim, QOwnNotes,
   Obsidian, and The Archive all keep notes as plain files; Joplin and Iotas are
   the database-backed contrast, and both pay for it in folder-transparency terms
   (each says so in its own docs).
5. The historical complaint record is specific: Simplenote sync reliability
   ("The Simplenote sync is very unreliable. I lost content numerous times",
   HN 2015), preview quality, and nvUltra's long unreleased beta
   ([HN via Algolia](https://hn.algolia.com/api/v1/search?query=nvalt&tags=comment),
   [Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/)).
6. nvALT 3.0's own changelog modernized exactly the things a port must decide:
   plain-text storage as the default for new libraries, .md as the default
   extension, tags moved to Finder tags (a macOS-only mechanism with no Linux
   equivalent), and double-bracket wiki links kept working.
7. The 2026 baseline this corpus supports: boolean and phrase search with regex,
   inotify-backed watching with conflict-copy awareness, [[wikilinks]] over a
   derived index, #tag or frontmatter tag conventions (never xattrs), syncthing-safe
   atomic writes with zero lock or sidecar files, instant filtering at 10k+ notes,
   spellcheck and syntax highlighting via libspelling/GtkSourceView, fractional
   scaling and AT-SPI accessibility on Wayland, and Flatpak plus XDG paths plus a
   URL scheme handler.

## Part 1: Prior art survey

### 1.1 The reference point: Notational Velocity and nvALT

Notational Velocity (Zachary Schneirov) defines the interaction: "The same area
is used both for creating notes and searching", a no-match query plus Return
creates the note, title prefixes auto-complete, and selecting a note puts its
title back into the field; it "can search thousands of notes at once"
([notational.net](http://www.notational.net/)). The source is GPL-3.0 and was
last pushed 2026-10-17
([github.com/scrod/nv](https://github.com/scrod/nv)). NV keeps notes in a
single internal database by default, compressed and (optionally) encrypted
before recording to disk, with plain-text files in a Dropbox folder as the
external-interop mode ([notational.net](http://www.notational.net/)); the
notes-are-files model arrived with the forks.

nvALT (Brett Terpstra, BSD-3-Clause,
[github.com/ttscoff/nv](https://github.com/ttscoff/nv), last pushed 2026-11-13)
is the fork that added folder-of-text-files storage, MultiMarkdown preview, and
tagging. Its 3.0 release (2026-10-24) is the current head of the line and its
changelog is effectively a modernization checklist: new libraries are stored as
plain text files "instead of a single database"; the default extension is .md;
tags are stored as Finder tags with OpenMeta tags migrated on launch;
"Double-bracket wiki links still work"; the editor gained Markdown syntax
highlighting; preview moved to Apex; publishing moved to Marked Share with no
full Simplenote sync replacement; and rich-text libraries get a one-time
conversion to Markdown
([Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/)).
The same post confirms nvUltra remains unreleased after a years-long beta, with
multiple notebooks and subfolders planned
([Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/)).

### 1.2 The direct clone: nvpy

nvpy is the closest existing descendant on any platform: "a simplenote-syncing
note-taking tool inspired by Notational Velocity (and a little bit by nvALT too)
on OSX and ResophNotes on Windows. It is significantly uglier, but it is
cross-platform", with Linux as the author's tested first platform
([nvpy README](https://github.com/cpbotha/nvpy#readme)).

- File model: Simplenote is the primary store; nvpy "maintains its own database
  on disk" as a cache and can be configured "to save and load notes as clear
  text" locally, with sync continuable in the background
  ([README](https://github.com/cpbotha/nvpy#readme)). Notes-are-files is an
  option, not the architecture.
- Search UX: one search field filtering as you type, in the NV tradition; the
  README's own screencasts cover a "gstyle" search mode and inter-note linking
  ([README](https://github.com/cpbotha/nvpy#readme)).
- License and activity: BSD licence per the README's liability clause; 856
  stars; last push 2026-08-15; last release 2.3.1 on 2023-09-29, identical on
  GitHub and [PyPI](https://pypi.org/pypi/nvpy/json). Maintained by yuuki0xff
  since 1.0.0 per the README. Slow-moving but not dead.
- Versus the NV philosophy: it ports the interaction faithfully and adds
  cross-platform reach; it drifts by making a cloud account the center of
  gravity, which is precisely the axis Commonplace deliberately refuses
  (spec section 4). Tk keeps its visual quality low by the author's own account
  ("significantly uglier").

### 1.3 The editor-plugin school

These clone the interaction, not the app, and all of them are files-first by
construction: the notes folder is just a directory.

- [notational-fzf-vim](https://github.com/alok/notational-fzf-vim) (1,127
  stars, last push 2025-12-28, badge claims "actively maintained", no license
  file on the repo): fzf fuzzy search over a notes directory where "searching
  for a note and creating one are the same operation" (README). Its README also
  documents why users left nvALT: "It's not meant for large text files, and
  opening them will cause it to lag a lot", no splits, and inferior editing.
- [Notational-Sublime](https://github.com/dansheffler/Notational-Sublime) (24
  stars, last push 2014-07-04, unmaintained by the author's own README
  statement): incremental search panel with title-prefix auto-complete and
  open-or-create on Enter, defaults to ~/Dropbox/Notes with .md. Historically
  valuable because it documents NV's own link behavior: "In Notational Velocity
  text surrounded by square brackets [[like so]] becomes a link to a note with
  that title" with auto-suggestion of existing titles.
- [vscode-notes](https://github.com/dionmunk/vscode-notes) (96 stars, last push
  2026-10-30, docs CC-BY-NC-4.0): notes in "a folder anywhere on your system",
  external additions "show up on their own" via a poll watcher
  (`notes.watchExternalChanges`, `notes.watchIntervalSeconds` settings), typing
  a list filter that creates on no-match, content search delegated to VS Code's
  search, and per-note extension choice (README). Its future-plans list wants
  frontmatter tags, evidence the tag question is still open in this school.
- Also in the school, found in search: [nvvim](https://github.com/cwoac/nvvim)
  (NV-inspired vim mode, MIT, dormant since 2018), and
  [shell-velocity](https://github.com/saccarosium/shell-velocity) (a portable
  notational velocity in POSIX shell, MIT, 2024).

### 1.4 The modern rebuild: nvEnvy

[nvEnvy](https://github.com/kenm47/nvEnvy) (MIT, 18 stars, last push
2026-11-06, release 1.3.1 on 2026-10-15, macOS 14+, Swift/SwiftUI, Mac App
Store distribution) is the freshest rebuild inside the lineage, and its README
is a direct answer to "what is the essential nvALT feature set, modernized":

- Instant search with Return opening "the best title match (exact, then title
  prefix) or create a note if nothing matches", phrase search, and Cmd+Return
  to force creation under a prefixing title.
- "Plain-text Markdown: notes stored as plain .md files with YAML frontmatter
  for tags and dates."
- Keyboard-first navigation (Cmd+L to search, Cmd+J/K to move, Escape back).
- Sync by placing the notes folder in iCloud Drive, "with conflict
  resolution"; no sync engine of its own.
- Wikilinks ([[link to note]]) with autocomplete and click-to-navigate; tags in
  frontmatter with batch tagging, a tag sidebar with counts, and Finder tag
  mirroring; live HTML preview with custom CSS plus export; one-click nvALT
  import with OpenMeta tag migration; bookmarks that save and recall search
  queries on Cmd+1-9; URL schemes `nvenvy://find/title` and
  `nvenvy://make?title=...&body=...`; AppleScript, Shortcuts, and a Services
  menu entry; word count toggle.

Versus the NV philosophy it is fully faithful on search-becomes-creation and
files-on-disk, and it demonstrates that the modern expectations layered on top
are links, tags, preview, and automation hooks, not a database.

### 1.5 The commercial descendant: The Archive

The Archive (Christian Tietze, Zettelkasten.de, macOS, commercial with a 60-day
trial; a third-party review lists it at 19 USD) is the Zettelkasten-flavored
lineage descendant
([product page](https://zettelkasten.de/the-archive/),
[review](https://bicycleforyourmind.com/revival-of-nvalt)).

- File model: "The Archive stores notes as plain text", images referenced with
  Markdown paths against a media subfolder convention, syncable with any cloud
  storage ([help](https://zettelkasten.de/the-archive/help/)).
- Search: boolean operators in caps (AND, OR, NOT or ! prefix) and quoted
  phrases, with the honest caveat "OR and AND do not have correct operator
  precedence" ([help](https://zettelkasten.de/the-archive/help/)).
- Links and tags: "[[Double Bracket Wiki-Links]], which trigger a search";
  notes carry stable date-time IDs in the default format yyyyMMddHHmm; tags are
  plain #hashtags, and Finder tags are refused on purpose "so you don't tag
  yourself into a corner", with a converter that writes Finder metadata back
  into #hashtags ([help](https://zettelkasten.de/the-archive/help/)).
- Automation: URL schemes thearchive://search/TERM, /match/TERM, and
  /matchOrCreate/TERM which "will create a note of the exact title TERM if
  that doesn't exist", plus a Quick Entry popup
  ([help](https://zettelkasten.de/the-archive/help/)).
- Product surface: "Fast search", "Powerful hashtags", auto-completion of tags
  and links, saved searches, themes
  ([product page](https://zettelkasten.de/the-archive/)).

It honors notes-are-files absolutely and shows what a single-folder app can
charge for: speed and tag/link ergonomics over the same plain files.

### 1.6 The broader field (contrast only)

| App | Model | Search | Where it stands on notes-are-files |
|---|---|---|---|
| [Zettlr](https://github.com/Zettlr/Zettlr) (GPL-3.0, 13,924 stars, pushed 2026-11-17) | plain Markdown files, "Zettlr does not mess with your files" ([FAQ](https://docs.zettlr.com/en/reference/faq.html)) | global search across all open workspaces: implicit AND, OR via pipe, ! negation, quoted phrases, case-insensitive, partial words ([docs](https://docs.zettlr.com/en/file-manager/search.html)) | honors files, but writes `.ztr-directory` sidecar files into folders (own FAQ), which a syncthing-pure folder must not tolerate |
| [Notable](https://notable.app/) (repo 23,489 stars, last push 2024-06-21; desktop free, future sync subscription) | "Notes and attachments are simply stored on your disk", individually ([site](https://notable.app/)) | fuzzy search; quick open ([site](https://notable.app/)) | honors files and advertises third-party editing, Git, regex replace; nestable tags; development visibly stalled since 2024 |
| [Zim Wiki](https://zim-wiki.org/) | "All data is stored in plain text files with wiki formatting", outline-like folder structure, wiki text editable with other editors ([site](https://zim-wiki.org/)) | page and full-text search over the notebook | honors files; its wiki markup is its own dialect rather than Markdown |
| [Joplin](https://github.com/laurent22/joplin) (57,222 stars, release 3.7.22 on 2026-10-28) | notes in Markdown inside an app-owned store; sync targets are Joplin Cloud, Nextcloud, S3, WebDAV, Dropbox, OneDrive, or the local filesystem ([sync docs](https://joplinapp.org/help/apps/sync/)); "Joplin sync directory is basically just a database" ([FAQ](https://joplinapp.org/help/faq/)) | "Full text search is available on all platforms" ([README](https://github.com/laurent22/joplin#readme)) | violates for the folder: files are not directly editable; an external-editor command exists per the FAQ; E2EE encrypts the sync copy, master-key based ([E2EE docs](https://joplinapp.org/help/apps/sync/e2ee/)) |
| [Obsidian](https://obsidian.md/) (proprietary; personal use free, commercial 50 USD/user/yr, Sync add-on 4 USD/mo annual with E2EE, [pricing](https://obsidian.md/pricing)) | "Obsidian stores your notes locally as plain text Markdown files" and "uses open file formats, so you're never locked in" ([site](https://obsidian.md/)) | link-first retrieval: "Link anything and everything", backlinks and an interactive graph ([site](https://obsidian.md/)) | accidentally-honors: files-first with a proprietary app and paid sync on top; the closest mass-market proof of the model |
| [QOwnNotes](https://github.com/pbek/QOwnNotes) (GPL-2.0, 5,967 stars, release 26.11.10 on 2026-11-15) | "notes are stored as plain text markdown files", synced via Nextcloud/ownCloud file sync, "other software, like Syncthing or Dropbox can be used too" ([README](https://github.com/pbek/QOwnNotes#readme)) | editor-integrated search across note files | honors files; the most syncthing-explicit statement in the field |
| [FSNotes](https://github.com/glushchenko/fsnotes) (MIT, 7,580 stars, pushed 2026-11-15, Swift, macOS/iOS) | markdown files, multiple folders; named in the field as "a free alternative which does more than both nvALT and The Archive" ([review](https://bicycleforyourmind.com/revival-of-nvalt)) | incremental search | honors files; macOS-only |

### 1.7 Rust, GTK4, and Linux-native apps worth studying

Nothing found here is nvALT-shaped: no search-first unibar GUI over plain files
exists on Linux today. What exists, from GitHub/GitLab searches on 2026-11-17:

- [Rnote](https://github.com/flxzt/rnote) (Rust, GTK4, GPL-3.0, 11,892 stars,
  release 0.14.2 on 2026-05-01, pushed 2026-11-16): vector ink notes, not text
  notes. Worth studying as a large active Rust/GTK4 canvas app and for its
  input handling, not for the niche itself.
- [Folio](https://github.com/toolstack/Folio) (Vala, 298 stars, release 26.01
  on 2026-04-16, pushed 2026-10-14): "beautiful markdown note-taking app for
  GNOME" with near-WYSIWYG rendering, GNOME Shell search integration,
  note-to-note links, trash, Flatpak-first distribution. Notebook-oriented
  editor, not a search-becomes-creation filter.
- [GNote](https://gitlab.gnome.org/GNOME/gnote) (C++, gtkmm 4.10 and
  libadwaita 1.6 per its README, last activity 2026-11-07): the Tomboy port,
  still maintained; a GNOME-ecosystem contrast with no NV-style search model.
- [Iotas](https://gitlab.gnome.org/World/iotas) (Python, GTK, last activity
  2026-11-07): explicitly database-backed: "Notes are stored in SQLite,
  providing for fast search (FTS)... Plain files can be retrieved by making a
  backup (CLI)", with Nextcloud Notes sync and partial loading for large
  collections (README). The cleanest live statement of the DB-road-not-taken.
- [Noteworthy](https://github.com/SeaDve/Noteworthy) (Rust, 63 stars,
  archived 2024): "Modern, Fast, and Version-Controlled Markdown Notes App";
  an abandoned Rust data point, useful mainly as a warning.
- Terminal-side Rust where the niche's Linux energy actually lives:
  [rucola](https://github.com/Linus-Mussmaecher/rucola) (552 stars, GPL-3.0,
  pushed 2026-10-18, terminal markdown note manager),
  [iwe](https://github.com/iwe-org/iwe) (1,827 stars, markdown knowledge graph
  with LSP and MCP surfaces), [textpod](https://github.com/freetonik/textpod)
  (322 stars, one-big-text-file school), and
  [lockbook](https://github.com/lockbook/lockbook) (450 stars, encrypted
  notebook, pushed 2026-11-17).
- Two small active nvALT-inspired Electron/TS apps showed up in search,
  [symiosis](https://github.com/dathinaios/symiosis) (instant search, in-place
  markdown rendering, vim/emacs keybindings) and
  [gravity-notes](https://github.com/resure/gravity-notes); both evidence of
  continued demand, neither native.

## Part 2: The gaps (improvement opportunities for Commonplace)

Each line names the gap and cites where it comes from.

1. Search power beyond substring matching. NV matches words in title or body;
   users now get boolean and phrase search in The Archive (AND/OR/NOT with a
   documented precedence bug) and Zettlr (AND, OR pipe, ! negation, phrases),
   so a Linux port should ship phrase plus boolean plus regex from day one and
   get precedence right.
   ([notational.net](http://www.notational.net/),
   [Archive help](https://zettelkasten.de/the-archive/help/),
   [Zettlr search](https://docs.zettlr.com/en/file-manager/search.html))
2. Kill the cloud-account center of gravity. Simplenote reliability is the
   most-repeated historical complaint in the lineage ("The Simplenote sync is
   very unreliable. I lost content numerous times", HN 2015; "every sync they
   do it fucks up data in some way", HN 2014), and nvALT 3.0 itself ships no
   full Simplenote sync replacement; nvpy still makes the account primary. A
   plain folder plus Syncthing, exactly as QOwnNotes names it, removes the
   complaint class entirely.
   ([HN via Algolia](https://hn.algolia.com/api/v1/search?query=nvalt&tags=comment),
   [Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/),
   [nvpy README](https://github.com/cpbotha/nvpy#readme),
   [QOwnNotes README](https://github.com/pbek/QOwnNotes#readme))
3. Preview that matches the editor. nvALT's preview story needed an external
   renderer every era (bundled MultiMarkdown, then Apex and Marked in 3.0) and
   nvUltra is preannounced on "way better Markdown features and preview"; the
   modern baseline is the live in-app preview with custom CSS that nvEnvy,
   Folio, and Iotas all ship.
   ([Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/),
   [nvEnvy README](https://github.com/kenm47/nvEnvy#readme),
   [Folio README](https://github.com/toolstack/Folio#readme),
   [Iotas README](https://gitlab.gnome.org/World/iotas/-/raw/main/README.md))
4. Handle big notes and long files. The fzf-vim clone exists because "opening
   [large files] will cause it [nvALT] to lag a lot"; incremental rendering
   and lazy loading (Iotas does partial loading for large collections) are the
   field's answers.
   ([notational-fzf-vim README](https://github.com/alok/notational-fzf-vim#readme),
   [Iotas README](https://gitlab.gnome.org/World/iotas/-/raw/main/README.md))
5. External-change watching as a first-class feature. VS Code's notes
   extension ships watcher settings because users edit the same folder from
   everywhere; Commonplace's spec already demands watching plus scan, and the
   notify crate plus inotify make it cheap.
   ([vscode-notes README](https://github.com/dionmunk/vscode-notes#readme),
   [notify-rs](https://github.com/notify-rs/notify#readme),
   [inotify(7)](https://man7.org/linux/man-pages/man7/inotify.7.html))
6. Wikilinks with a derived, rebuildable backlink index. [[links]] date back to
   NV itself (documented by Notational-Sublime), still work in nvALT 3.0,
   trigger searches in The Archive, autocomplete in nvEnvy, and the backlink
   pane is what nvALT users say they miss.
   ([Notational-Sublime README](https://github.com/dansheffler/Notational-Sublime#readme),
   [Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/),
   [Archive help](https://zettelkasten.de/the-archive/help/),
   [nvEnvy README](https://github.com/kenm47/nvEnvy#readme),
   [bicycleforyourmind](https://bicycleforyourmind.com/revival-of-nvalt))
7. Tags that survive the folder. nvALT 3.0 moved tags into Finder tags, a
   macOS filesystem mechanism with no Linux equivalent; The Archive refuses
   metadata tagging on principle and keeps #hashtags in the text; nvEnvy uses
   YAML frontmatter. The portable answer on Linux is inline tags or frontmatter,
   never xattrs.
   ([Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/),
   [Archive help](https://zettelkasten.de/the-archive/help/),
   [nvEnvy README](https://github.com/kenm47/nvEnvy#readme))
8. Stable identity conventions. The Archive's yyyyMMddHHmm note IDs give
   [[links]] something durable to point at while titles change; an NV port
   whose files are titled by first line needs an explicit rename-and-relink
   story.
   ([Archive help](https://zettelkasten.de/the-archive/help/))
9. Saved searches as a first-class object. The Archive ships "Saved searches";
   nvEnvy puts them on Cmd+1-9. Nobody in the Linux set does.
   ([Archive product page](https://zettelkasten.de/the-archive/),
   [nvEnvy README](https://github.com/kenm47/nvEnvy#readme))
10. An automation surface. The Archive's thearchive://matchOrCreate and
    nvEnvy's nvenvy://make make capture scriptable from anywhere; NV was
    "mouseless" by design and the Linux ecosystem expects a URL scheme plus a
    CLI.
    ([Archive help](https://zettelkasten.de/the-archive/help/),
    [nvEnvy README](https://github.com/kenm47/nvEnvy#readme),
    [notational.net](http://www.notational.net/))
11. Migration paths from every store shape. nvEnvy ships one-click nvALT
    import with OpenMeta tags; nvALT 3.0 ships rich-text-to-Markdown
    conversion; VS Code's extension reads any folder with any extensions. A
    Linux port should name its importers explicitly, including nvpy's
    Simplenote export.
    ([nvEnvy README](https://github.com/kenm47/nvEnvy#readme),
    [Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/))
12. Fill the actual Linux hole. The only GUI search-first clone is nvpy (Tk,
    Simplenote-centric, self-described ugly); everything else Linux is a plugin
    or a TUI. The gap is not "another notes app", it is the unibar over files,
    native.
    ([nvpy README](https://github.com/cpbotha/nvpy#readme),
    GitHub search 2026-11-17 as logged in section 1.7)

## Part 3: Modern baseline checklist (2026, Linux, this niche)

Each item states the requirement and its citation.

1. Unicode and UTF-8. All note text in UTF-8, which both halves of the stack
   already guarantee: "Glib uses UTF-8 for its strings, and GUI toolkits like
   GTK that use GLib do the same thing"
   ([GLib docs](https://docs.gtk.org/glib/character-set.html)), and Rust's
   string types are UTF-8 by definition
   ([std docs](https://doc.rust-lang.org/std/primitive.str.html)). Note the
   one mismatch the GLib doc itself flags: filenames on disk are not guaranteed
   UTF-8, so the index needs a lossless bytes-to-display policy.
2. File watching and external changes. Watch with inotify
   ([inotify(7)](https://man7.org/linux/man-pages/man7/inotify.7.html)) via
   the notify crate (cross-platform, debouncers, used by rust-analyzer and zed;
   [README](https://github.com/notify-rs/notify#readme)), and pair it with a
   rescan since watchers can drop events. On external edit, reload and never
   clobber: the ecosystem's conflict contract is Syncthing's, below.
3. Syncthing-friendliness. Syncthing detects a conflict when a file "has been
   modified on two devices simultaneously and the content actually differs",
   keeps both versions, and renames the loser to
   `<filename>.sync-conflict-<date>-<time>-<modifiedBy>.<ext>`; conflict files
   then propagate as normal files; the only folder marker is .stfolder, and
   the docs describe no lock files at all
   ([syncing docs](https://docs.syncthing.net/users/syncing.html),
   [FAQ](https://docs.syncthing.net/users/faq.html)). Baseline: atomic-rename
   writes, no lock or sidecar files (Zettlr's `.ztr-directory` sidecars are the
   counterexample, per its own
   [FAQ](https://docs.zettlr.com/en/reference/faq.html)), and an explicit
   policy for *.sync-conflict-* files (treat as notes or filter, but never
   delete).
4. Wayland display expectations. Support fractional scaling: the
   fractional-scale-v1 protocol ("This protocol allows a compositor to suggest
   for surfaces to render at fractional scales", interfaces
   wp_fractional_scale_manager_v1 and wp_fractional_scale_v1, currently staged)
   is how a GTK4 window renders crisply at 1.25x or 1.5x
   ([wayland.app](https://wayland.app/protocols/fractional-scale-v1)).
5. Accessibility. GTK4's accessibility bridge proxies to "AT-SPI on Linux/BSD"
   and "Standard GTK controls implement the GtkAccessible interface and are
   thus accessible to ATs by default"; keep to standard widgets and set
   labels/roles on the custom search field and note list
   ([GTK4 docs](https://docs.gtk.org/gtk4/section-accessibility.html)).
6. Editor features users now expect. Syntax highlighting: GtkSourceView
   extends GtkTextView with highlighting for programming and markup languages,
   a completion framework, and search-and-replace with regex
   ([project page](https://wiki.gnome.org/Projects/GtkSourceView)); Iotas
   shows it driving markdown highlighting with themes on GTK today
   ([README](https://gitlab.gnome.org/World/iotas/-/raw/main/README.md)).
   Spellcheck: libspelling, "a spellcheck library for GTK 4" wrapping
   enchant-2 with adapters for GtkSourceBuffer, GtkTextView, and GtkEntry
   ([README](https://gitlab.gnome.org/GNOME/libspelling/-/raw/main/README.md)).
   Word count: nvEnvy makes it a first-class toggle
   ([README](https://github.com/kenm47/nvEnvy#readme)).
7. Wikilinks and backlinks. The [[double bracket]] form is the field's
   convention, inherited from NV itself; resolution by note title with
   autocomplete (nvEnvy), link-target search (The Archive), or both. Backlinks
   and graph views are what Obsidian made mainstream
   ([site](https://obsidian.md/)) and what nvALT lacks
   ([review](https://bicycleforyourmind.com/revival-of-nvalt)); a derived,
   rebuildable backlink index honors the no-database invariant.
8. Tag conventions. Live conventions in the corpus: inline #hashtags (The
   Archive, deliberately the only place tags live) and YAML frontmatter tags
   (nvEnvy). The @tag form appears nowhere in this survey's sources. Extended
   attributes are a macOS-ism (nvALT 3.0's Finder tags) and must not carry
   tags on Linux.
   ([Archive help](https://zettelkasten.de/the-archive/help/),
   [nvEnvy README](https://github.com/kenm47/nvEnvy#readme))
9. Encryption at rest. The field encrypts transport, not the folder: Joplin's
   E2EE "will be used to encrypt all your notes" on the sync target
   ([E2EE docs](https://joplinapp.org/help/apps/sync/e2ee/)), Obsidian Sync
   sells E2EE as an add-on ([pricing](https://obsidian.md/pricing)), and
   nvALT keeps encrypted notes as an open-format feature
   ([Terpstra 2026](https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/)).
   The file-native options a Linux app should interoperate with are age (BSD-3,
   spec at [age-encryption.org/v1](https://age-encryption.org/v1), Rust
   implementation rage and YubiKey plugins per the
   [README](https://github.com/FiloSottile/age#readme)) and GnuPG; lockbook
   exists as a fully-encrypted FOSS notebook in Rust
   ([repo](https://github.com/lockbook/lockbook)).
10. Search performance at 10k+ notes. The bar is set by three references: NV's
    own claim to "search thousands of notes at once"
    ([notational.net](http://www.notational.net/)), ripgrep's recursive search
    over directories of plain text with its "Is it really faster than
    everything else?" benchmark section
    ([GUIDE](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md)), and
    Tantivy, "a full-text search engine library inspired by Apache Lucene and
    written in Rust" with published benchmarks
    ([README](https://github.com/quickwit-oss/tantivy#readme)). Design target:
    incremental as-you-type filtering over 10k notes must feel instant, with a
    derived in-memory index that survives deletion (the spec's index
    invariant). No published note-app benchmark fixes a millisecond number;
    Commonplace must measure its own (see Unknowns).
11. Packaging and system integration. Flatpak is the distribution channel the
    Linux GUI field actually uses: Flathub is "the home of Flatpak
    applications" ([docs.flathub.org](https://docs.flathub.org/)) and Rnote,
    Folio, and Iotas all lead their READMEs with Flathub badges. Settings and
    caches follow the XDG base directory specification (v0.8, 2021): XDG_CONFIG_HOME
    defaults to $HOME/.config and XDG_DATA_HOME to $HOME/.local/share
    ([spec](https://specifications.freedesktop.org/basedir-spec/latest/)).
    Register a URL scheme for capture and deep links: on freedesktop systems
    "URI scheme handling [is] handled through applications handling the
    x-scheme-handler/foo MIME type"
    ([Arch Wiki, XDG MIME Applications](https://wiki.archlinux.org/title/XDG_MIME_Applications)),
    with thearchive:// and nvenvy:// as working precedents in this exact niche
    ([Archive help](https://zettelkasten.de/the-archive/help/),
    [nvEnvy README](https://github.com/kenm47/nvEnvy#readme)).

## Sources

All fetched 2026-11-17. API rows (GitHub, GitLab, PyPI, HN Algolia) were read
as JSON via keyless GET.

| # | URL | Supported | As-of |
|---|---|---|---|
| 1 | http://www.notational.net/ | NV interaction model, search claims | 2026-11-17 |
| 2 | https://github.com/scrod/nv | NV license (GPL-3.0), activity | 2026-11-17 |
| 3 | https://github.com/ttscoff/nv | nvALT license (BSD-3), activity, README basics | 2026-11-17 |
| 4 | https://brettterpstra.com/2026/10/24/nvalt-back-from-the-dead/ | nvALT 3.0 changelog facts, nvUltra status, Simplenote demotion | 2026-11-17 |
| 5 | https://bicycleforyourmind.com/revival-of-nvalt | user sentiment, no-backlinks complaint, The Archive price, FSNotes pointer | 2026-11-17 |
| 6 | https://github.com/cpbotha/nvpy (readme API + PyPI) | nvpy model, license, activity, release dates | 2026-11-17 |
| 7 | https://github.com/alok/notational-fzf-vim | plugin-school model, nvALT lag complaint | 2026-11-17 |
| 8 | https://github.com/dansheffler/Notational-Sublime | NV [[wikilink]] behavior, search-and-create emulation, dormancy | 2026-11-17 |
| 9 | https://github.com/dionmunk/vscode-notes | folder model, watcher settings, create-on-no-match | 2026-11-17 |
| 10 | https://github.com/kenm47/nvEnvy | essential feature set, release 1.3.1 | 2026-11-17 |
| 11 | https://zettelkasten.de/the-archive/ | product surface, plain text storage | 2026-11-17 |
| 12 | https://zettelkasten.de/the-archive/help/ | search operators, wikilinks, IDs, tags, URL schemes | 2026-11-17 |
| 13 | https://github.com/Zettlr/Zettlr + https://docs.zettlr.com/en/reference/faq.html | license, activity, .ztr-directory sidecars | 2026-11-17 |
| 14 | https://docs.zettlr.com/en/file-manager/search.html | global search operators | 2026-11-17 |
| 15 | https://notable.app/ + https://github.com/notable/notable | file model, fuzzy search, activity stall, no license file | 2026-11-17 |
| 16 | https://zim-wiki.org/ | plain-text wiki storage | 2026-11-17 |
| 17 | https://github.com/laurent22/joplin + https://joplinapp.org/help/ | Markdown store, sync targets, FTS | 2026-11-17 |
| 18 | https://joplinapp.org/help/faq/ | "sync directory is basically just a database", external editor | 2026-11-17 |
| 19 | https://joplinapp.org/help/apps/sync/e2ee/ | E2EE scope | 2026-11-17 |
| 20 | https://obsidian.md/ + https://obsidian.md/pricing | local plain-text Markdown claim, links/backlinks/graph, pricing | 2026-11-17 |
| 21 | https://github.com/pbek/QOwnNotes | files model, Syncthing named, activity | 2026-11-17 |
| 22 | https://github.com/glushchenko/fsnotes | free alternative pointer, activity | 2026-11-17 |
| 23 | https://github.com/flxzt/rnote | Rust/GTK4 reference app | 2026-11-17 |
| 24 | https://github.com/toolstack/Folio | GNOME markdown notes, features | 2026-11-17 |
| 25 | https://gitlab.gnome.org/GNOME/gnote | gtkmm4 stack, Tomboy lineage, activity | 2026-11-17 |
| 26 | https://gitlab.gnome.org/World/iotas | SQLite/FTS model, partial loading, spellcheck | 2026-11-17 |
| 27 | https://github.com/SeaDve/Noteworthy, https://github.com/Linus-Mussmaecher/rucola, https://github.com/iwe-org/iwe, https://github.com/freetonik/textpod, https://github.com/lockbook/lockbook, https://github.com/dathinaios/symiosis, https://github.com/resure/gravity-notes, https://github.com/cwoac/nvvim, https://github.com/saccarosium/shell-velocity | Linux/Electron periphery census | 2026-11-17 |
| 28 | https://man7.org/linux/man-pages/man7/inotify.7.html | inotify reference | 2026-11-17 |
| 29 | https://github.com/notify-rs/notify | Rust watcher library | 2026-11-17 |
| 30 | https://wayland.app/protocols/fractional-scale-v1 | fractional scaling protocol | 2026-11-17 |
| 31 | https://docs.gtk.org/gtk4/section-accessibility.html | AT-SPI, default widget accessibility | 2026-11-17 |
| 32 | https://docs.gtk.org/glib/character-set.html | GLib/GTK UTF-8, filename caveat | 2026-11-17 |
| 33 | https://wiki.gnome.org/Projects/GtkSourceView | highlighting, completion, regex search | 2026-11-17 |
| 34 | https://gitlab.gnome.org/GNOME/libspelling | GTK4 spellcheck | 2026-11-17 |
| 35 | https://github.com/FiloSottile/age + https://age-encryption.org/v1 | age encryption, spec | 2026-11-17 |
| 36 | https://docs.syncthing.net/users/syncing.html + /users/faq.html | conflict naming, both-kept, .stfolder | 2026-11-17 |
| 37 | https://github.com/BurntSushi/ripgrep (GUIDE.md) | search performance bar | 2026-11-17 |
| 38 | https://github.com/quickwit-oss/tantivy | full-text engine, benchmarks | 2026-11-17 |
| 39 | https://docs.flathub.org/ | Flatpak distribution | 2026-11-17 |
| 40 | https://specifications.freedesktop.org/basedir-spec/latest/ | XDG paths | 2026-11-17 |
| 41 | https://wiki.archlinux.org/title/XDG_MIME_Applications | x-scheme-handler mechanism | 2026-11-17 |
| 42 | https://hn.algolia.com/api/v1/search?query=nvalt | HN stories and complaints | 2026-11-17 |

## Dead-end log

- WebSearch: absent from this harness's toolset at dispatch time; discovery ran
  on keyless GitHub/GitLab/PyPI/HN-Algolia endpoints plus direct fetches. The
  dispatch notes this as the DEGRADED path; the topic was fully answerable.
- scrod/nv README: no readme on the default path and the GitHub readme API
  returned 404; NV facts came from notational.net instead.
- zettelkasten.de/the-archive/manual/: one transient 500, then a real 404; the
  manual actually lives at /the-archive/help/ (found via sitemap).
- Obsidian help (help.obsidian.md redirects to obsidian.md/help/*): the new
  help site serves a JS shell with no article body to two different fetchers;
  the marketing site and pricing page carry the file-model facts instead.
- specifications.freedesktop.org desktop-entry spec: the single-page URL
  returns a 158-byte stub to plain curl and empty content to the fetchers, so
  the x-scheme-handler claim is cited to the Arch Wiki quote plus live app
  precedents, with the spec named as the normative reference.
- docs.zettlr.com sitemap.xml: returned nothing to curl; the doc pages were
  located through the docs index instead.
- Joplin repo readme/apps/*.md raw paths: 404 (tree layout differs); Joplin
  facts came from the top-level README and joplinapp.org pages.
- docs.flathub.org for-app-authors deep URL: guessed path 404; packaging claims
  kept to the landing page quote plus app README badges.
- notify-rs license: the GitHub API reports none; the README badge states
  CC0-1.0, so the license claim cites the README.
- apps.gnome.org: not attempted after the Obsidian shell failure; GNOME app
  coverage came from GNOME GitLab project APIs directly.

## Unknowns

- Notable's exact on-disk metadata convention (frontmatter vs sidecar JSON) is
  not stated on its site and its repo docs were not fetched; single-source
  unknown.
- GNote's on-disk note format (per-note XML vs otherwise) is unverified here;
  only its lineage and current stack are confirmed.
- GNOME Notes/Bijiben's storage model: docs too thin to characterize; only
  repo activity was captured.
- No published benchmark fixes a latency number for 10k-note incremental
  search in this app class; the performance item is a design target plus
  engine references, and Commonplace must measure its own corpus.
- The Archive's price (19 USD) is sourced from a third-party review, not the
  vendor page read today.
- nvALT issue-tracker mining was not performed (API budget spent on the survey
  rows); user complaints rest on the blog and HN record.
- The @tag tag convention: no source in this corpus uses it; its origin claim
  (if any) is unverified.

## Confidence

High on the prior-art survey and the platform checklist: nearly every
load-bearing claim quotes the app's own README, docs, or API metadata, all read
the same day. Medium on user-sentiment items (HN threads and two blog posts,
sampled not exhaustive). Performance expectations are directionally cited, not
benchmarked.
