# AGENTS.md

Guidance for coding agents (ZCode) working in Commonplace. Overrides the
global `~/.zcode/AGENTS.md` where they conflict.

## What this is

Commonplace is a Linux port of the Notational Velocity line: the original
NV by Zachary Schneirov, Brett Terpstra's nvALT fork, and the closed-source
nvUltra successor. One search field drives find-and-create over a folder of
plain-text notes. Rust, three-crate workspace, GPL-3.0.

Status: the engine, the CLI, and the app shell are complete (roadmap
Phases 1-11, tested and benchmarked; v0.4.0 shipped the render
boundary and export core, v0.5.0 the CLI arc and M1, v0.6.0 the app
shell and the search-field state machine); the editor and preview
arcs are ahead (Phases 12+). The
contract lives in `spec.md`; the phase plan in `roadmap.md`. Read
both before any code change.

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

## Executing a phase

Every roadmap phase runs the same loop, in this order:

1. Re-read the phase and every research section it cites; the
   citations are the specification of the work.
2. Re-diff the moving target: `git -C .repos/nvalt fetch` and compare
   against the tip the phase assumes (6a4c000 at Phase 0); any
   upstream movement gets noted in the phase's checklist box before
   code starts.
3. Write the golden tests FIRST, from the cited contract tables and
   grammar rules, not from the implementation.
4. Implement to green: `cargo test`, then
   `cargo clippy --workspace --all-targets -- -D warnings`, then
   `cargo fmt --all`.
5. Docs carried in the same commits: spec rows, patchnotes when the
   phase ships a version, README if user-visible, this file if a
   decision landed.
6. Tick the phase's checkboxes in the shipping commit and update the
   decision ledger rows the phase answered.
7. Land it released per the release flow below; there is no
   Unreleased holding pen.

## Commands

- Build: `cargo build`
- Test: `cargo test` (all suites live in the core crate: store
  fixture folders, write-path and tag/link goldens, query goldens
  over the three upstream vocabularies, the search oracle property
  suite, and two opt-in benchmarks)
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- CI: `.github/workflows/ci.yml`, shape from Docket's hardened
  workflow with action pins copied byte-for-byte from Quire's `ci.yml`
  (the same Rust + GTK4 stack; dependabot maintains them fleet-wide). The release workflow joins at
  the packaging phase, shaped like Docket's `release.yml`.

## Conventions

- Single-source `VERSION`; the workspace `Cargo.toml` must match it.
- Prose rules from the global file apply to every doc here: no em-dashes.
- Notes are plain files: no database is required for operation. Any
  feature that wants an index keeps it derived (rebuildable), never
  authoritative. No cloud sync: files stay syncthing-safe by design.
- Atomic writes only; the engine never leaves a half-written note.

## Workflow: straight on main

Work lands directly on `main`. No feature branches and no worktrees:
this is a private solo repo and the history is the review. The
discipline that replaces branching: one verified piece at a time
(its tests, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo fmt --all` all green before the piece is committed), a
push at each phase's end so CI validates the phase whole, and never
a red or unverified commit. This overrides the global
worktree-isolation rule for approved agents: they work directly on
main here, exactly as Brandon directed on 2026-11-17.

## Documentation surfaces

Five files, one job each. Update them as the coding happens, in the
same commits as the changes they describe, never in a later sweep;
the roadmap's documentation catch-up milestones (M1, M2, and Phase
24) exist to catch what slipped, not to carry planned work.

- `README.md`: all user-facing information, organized like the rest
  of the fleet (tagline, status, what it does, upstream
  acknowledgment, build, license). Present-tense claims only for
  shipped behavior; future features stay future-tensed.
- `AGENTS.md`: all programmer- and agent-facing information: build
  and test commands, conventions, decisions and their rationale,
  port rules, license posture. A decision is not landed until it is
  recorded here or in the roadmap's decision ledger, in the same
  commit as the change it governs.
- `spec.md`: structural and semantic changes; the contract of record
  and the heart of the system. Spec first, then code: code that
  disagrees with spec is always a code bug.
- `patchnotes.md`: everything changed on each version, newest first.
  There is no Unreleased section, ever: work either sits unlanded or
  lands released, and every entry corresponds to a tag.
- `roadmap.md`: the phase plan with checkboxes. Tick the box in the
  commit that ships the work, never in a batch after the fact.

## Release flow

Standard fleet flow: bump `VERSION` + workspace `Cargo.toml` together,
prepend the `patchnotes.md` entry (newest first, `## vX.Y.Z (YYYY-MM-DD)`
headings), tick the shipped `roadmap.md` boxes, then commit + push and
wait for green CI before cutting the annotated `v*` tag (tag message =
the full patchnotes entry, extracted verbatim, `--cleanup=verbatim`).
**Every tag is followed, in the same session, by a GitHub Release**
(`gh release create vX.Y.Z --title <the entry's heading line>
--notes-file <the same scratch file>`): a tag without its Release is
a defect, not a done item; the 2026-11-11 fleet backfill exists
because 184 tags had no releases. Cutting the tag itself stays
approval-gated with Brandon.
