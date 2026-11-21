# Commonplace

**Search-becomes-creation notes for Linux.** A Rust port of the
Notational Velocity line: one field finds your notes, and when nothing
matches, it becomes the new note. Plain text files in a folder. No
database required. No cloud.

This is an ode to the bloggers: to the ones who kept commonplace books
before weblogs had a name, and to the ones who turned them into feeds.
Notational Velocity (Zachary Schneirov) was the tool a generation of
bloggers kept open all day. nvALT (Brett Terpstra) kept it alive and
grew it. nvUltra (Brett Terpstra and Fletcher Penney) carries the line
forward on the Mac. Commonplace carries it to Linux.

## Status

The app is alive. The behavior contract lives
in `spec.md`; the phase-by-phase port plan (which upstream code
lands when) lives in `roadmap.md`. Research notes that feed those
phases live in `research/`. The engine loads, searches, sorts,
filters, writes, tags, links, renders, and exports a notes folder
headlessly (Phases 1-7), `commonplace-cli` drives all of it from the
shell (Phases 8-9, v0.5.0; see the CLI section below), and the GTK4
app runs: the two-pane shell, the live list over a watched folder,
and the one-field search machine (Phases 10-11, v0.6.0). The editor,
preview, and preferences phases are ahead (v0.7.0+).

## What it does

- One search field that finds notes as you type and creates them when
  nothing matches: the NV model, exactly (engine and CLI today; the
  field itself ships with the app phases).
- Notes are plain files (`.txt`, `.md`) in a folder you own. The index
  is derived and rebuildable; delete it and nothing is lost.
- A CLI (`commonplace-cli`) with a machine surface, because the engine
  is GUI-free and testable headless.
- A GTK4 app (plain GTK4, no libadwaita) in the house Kanagawa Dragon
  style, keyboard-first like the original: the shell runs today (the
  editor and preview phases are next).
- Preview and export for Markdown/MultiMarkdown, in the nvALT/nvUltra
  tradition (export is in the engine and the CLI today; the preview
  pane ships with the app phases).

Feature-level parity targets (NV core, nvALT, nvUltra) are pinned in
`spec.md` with the research citations behind them.

## The upstream line

- [Notational Velocity](https://github.com/scrod/nv): Zachary
  Schneirov's original (GPLv3 since 2011).
- [nvALT](https://github.com/ttscoff/nv): Brett Terpstra's fork
  (BSD-3; the primary port base).
- [nvUltra](https://nvultra.com): Terpstra + Fletcher Penney's
  closed-source successor; our parity target reads its public docs
  only.

Commonplace is not affiliated with or endorsed by any of them. The
names Notational Velocity, nvALT, and nvUltra belong to their authors.

## Building

```sh
cargo build --workspace
cargo test --workspace
```

The engine crate is dependency-free and tests headless; the CLI crate
carries exactly one dependency (comrak, the Markdown renderer); the
app crate needs GTK 4 development libraries.

## CLI

`commonplace-cli` is the scriptable half. The verb comes first, flags
after the verb name. Exit codes: 0 success (empty results included),
1 trouble (runtime failures, failed resolutions), 2 usage.

```sh
# One field, one query; the empty query lists everything.
commonplace-cli search "alpha beta"
commonplace-cli search --saved 1                  # a config hotkey slot
commonplace-cli list --sort title
commonplace-cli open "alpha"                      # absolute path of the match
commonplace-cli new "Fresh Note" --body "first line"
commonplace-cli append "fresh" --text "more"
commonplace-cli cat "alpha"
commonplace-cli render "alpha"                    # Markdown -> HTML
commonplace-cli tags
commonplace-cli export --format html --out ~/exports
commonplace-cli config --set notes_dir ~/Documents/Notes
commonplace-cli config --get notes_dir
commonplace-cli completions zsh                   # bash | zsh | fish

# A per-call notes-folder override rides the verb it belongs to:
commonplace-cli search --dir ~/other-notes "alpha"
```

Every verb takes `--json` and prints exactly one envelope
(`{"verb":...,"ok":true,...}`); shapes are additive from v0.5.0.

Config lives at `~/.config/commonplace/config.toml` (override the path
with `COMMONPLACE_CONFIG`; override the folder per call with `--dir`):

```toml
notes_dir = "~/Documents/Notes"

[saved_searches]
"1" = "project alpha"
"2" = "#reading"
```

The default notes folder is `~/Documents/Notes`. Saved searches use
the nvUltra hotkey slots (`0`-`9`) and are shared with the app.

## License

GPL-3.0. See `LICENSE`. The choice is forced by the upstream trees the
port consults: `scrod/nv` master is GPLv3 (relicensed 2011), and the
nvALT tree, which forked from it before the relicensing, is BSD-3.
GPL-3.0 is the safe umbrella for a port that consults both.
