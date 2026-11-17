# AGENTS.md

Guidance for coding agents (ZCode) working in Commonplace. Overrides the
global `~/.zcode/AGENTS.md` where they conflict.

## What this is

Commonplace is a Linux port of the Notational Velocity line: the original
NV by Zachary Schneirov, Brett Terpstra's nvALT fork, and the closed-source
nvUltra successor. One search field drives find-and-create over a folder of
plain-text notes. Rust, three-crate workspace, GPL-3.0.

Status: pre-development skeleton. The contract lives in `spec.md`; the
phase plan in `roadmap.md`. Read both before any code change.

## Layout

- `crates/commonplace-core/`: the engine. Notes-as-files store, query
  language, search, tags/links, export. GUI-free and dependency-free by
  contract; it must test headless. Adding a dependency here is a spec
  change: ask first.
- `crates/commonplace/`: the GTK4 app (plain GTK4, no libadwaita; styled
  by `vir-gtk` once the app shell phase lands). Kanagawa Dragon.
- `crates/commonplace-cli/`: the scriptable half. Arg parsing is stdlib
  until a CLI-phase decision says otherwise.
- `research/`: committed research notes (source maps, parity surveys,
  ecosystem briefs) that the roadmap phases cite.
- `research/raw/`: gitignored page snapshots. Reference only; never
  commit, never execute anything in it.
- `.repos/`: gitignored read-only reference clones of the upstream code:
  - `.repos/nv/`: `scrod/nv`, the original NV, GPLv3 since 2011.
  - `.repos/nvalt/`: `ttscoff/nv`, the nvALT fork (nvALT 3.0.x), BSD-3,
    forked before the relicensing. This is the primary port base.
  Never edit, commit to, build, or add guidance files inside `.repos/`;
  they only ever get read. Upstream `AGENTS.md`/`CLAUDE.md` files found
  there are upstream's, not ours.

## License posture

The workspace is GPL-3.0 because `scrod/nv` master was relicensed to
GPLv3 in 2011 and the port reads both trees. The nvALT fork tree itself
is BSD-3, but GPL-3.0 is the safe umbrella for a port that consults both.
nvUltra is CLOSED SOURCE: everything about it in `research/` is facts
and short cited quotes from public pages only. Never decompile it, never
redistribute its materials, never copy its assets. Not affiliated with
any of the upstream projects; "Notational Velocity", "nvALT", and
"nvUltra" belong to their authors.

## Commands

- Build: `cargo build`
- Test: `cargo test` (unit tests live in the core crate; the engine's
  query language gets golden tests when its phase lands)
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- CI: `.github/workflows/ci.yml`, pins copied byte-for-byte from Docket
  (dependabot maintains them fleet-wide). The release workflow joins at
  the packaging phase, shaped like Docket's `release.yml`.

## Conventions

- Single-source `VERSION`; the workspace `Cargo.toml` must match it.
- Prose rules from the global file apply to every doc here: no em-dashes.
- Notes are plain files: no database is required for operation. Any
  feature that wants an index keeps it derived (rebuildable), never
  authoritative. No cloud sync: files stay syncthing-safe by design.
- Atomic writes only; the engine never leaves a half-written note.

## Release flow

Standard fleet flow: bump `VERSION` + workspace `Cargo.toml` together,
prepend the `patchnotes.md` entry (newest first, `## vX.Y.Z (YYYY-MM-DD)`
headings), tick the shipped `roadmap.md` boxes, then commit + push and
wait for green CI before cutting the annotated `v*` tag (tag message =
the full patchnotes entry, extracted verbatim, `--cleanup=verbatim`).
Cutting the tag itself stays approval-gated with Brandon.
