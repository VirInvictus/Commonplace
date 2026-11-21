//! The verb surface over the Phase 1-7 engine (spec §6). Every verb
//! returns its exit class, stdout text, and optional `--json` envelope;
//! `main` prints and maps the exit code. Exit taxonomy: 0 success
//! (empty is a valid answer for report verbs), 1 trouble (runtime
//! failures and failed resolutions where a target was required), 2
//! usage.

use std::io::Read;
use std::path::{Path, PathBuf};

use commonplace_core::config::Config;
use commonplace_core::export::{ExportFormat, export_note_to_dir, export_store_to_dir};
use commonplace_core::render::{Renderer, expand_transclusions_rooted};
use commonplace_core::search::{FilterState, SortColumn, SortOptions};
use commonplace_core::store::{Store, load_folder};
use commonplace_core::tags::TagIndex;

use crate::args::{Invocation, SortArg, Verb};
use crate::json::{Json, envelope};
use crate::render::ComrakRenderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Ok,
    Trouble,
    Usage,
}

#[derive(Debug)]
pub struct VerbOutput {
    pub exit: Exit,
    pub stdout: String,
    pub stderr: Vec<String>,
    pub json: Option<Json>,
}

impl VerbOutput {
    fn ok(stdout: String) -> Self {
        VerbOutput {
            exit: Exit::Ok,
            stdout,
            stderr: Vec::new(),
            json: None,
        }
    }

    fn trouble(lines: Vec<String>) -> Self {
        VerbOutput {
            exit: Exit::Trouble,
            stdout: String::new(),
            stderr: lines,
            json: None,
        }
    }
}

/// The process environment the verbs consult, injected for testability
/// (the integration tests must not mutate process-global state).
#[derive(Debug, Clone)]
pub struct Env {
    pub config_path: PathBuf,
    pub home: PathBuf,
}

impl Env {
    /// Read the real environment: `COMMONPLACE_CONFIG` overrides the
    /// XDG config path; `HOME` backs `~` expansion.
    pub fn from_process() -> Env {
        let config_path = commonplace_core::config::Config::xdg_path();
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));
        Env { config_path, home }
    }
}

/// Run a parsed invocation against the real process environment.
pub fn run(invocation: &Invocation) -> VerbOutput {
    run_in(&Env::from_process(), invocation)
}

/// Run a parsed invocation against an injected environment.
pub fn run_in(env: &Env, invocation: &Invocation) -> VerbOutput {
    match invocation.verb {
        Verb::Search => search(env, invocation),
        Verb::List => list(env, invocation),
        Verb::Open => open(env, invocation),
        Verb::New => new(env, invocation),
        Verb::Append => append(env, invocation),
        Verb::Export => export(env, invocation),
        Verb::Tags => tags(env, invocation),
        Verb::Cat => cat(env, invocation),
        Verb::Render => render(env, invocation),
        Verb::Config => config(env, invocation),
        Verb::Completions => completions(invocation),
        Verb::Scheme => scheme(env, invocation),
    }
}

// -- shared plumbing -----------------------------------------------------------

struct Context {
    store: Store,
    config: Config,
    /// Scan warnings, pre-rendered for stderr; they never fail a verb.
    warnings: Vec<String>,
}

/// Open the store against the effective notes folder (the `--dir`
/// override wins over the config; both expand `~`).
fn context(env: &Env, invocation: &Invocation) -> Result<Context, VerbOutput> {
    let config =
        Config::load(&env.config_path).map_err(|e| VerbOutput::trouble(vec![format!("{e}")]))?;
    let folder = match &invocation.flags.dir {
        Some(dir) => expand_tilde(dir, &env.home),
        None => config.notes_dir(),
    };
    if !folder.is_dir() {
        return Err(VerbOutput::trouble(vec![format!(
            "notes folder is missing: {}",
            folder.display()
        )]));
    }
    let outcome = load_folder(&folder).map_err(|e| VerbOutput::trouble(vec![format!("{e}")]))?;
    let warnings = outcome
        .warnings
        .iter()
        .map(|w| format!("warning: {w}"))
        .collect();
    Ok(Context {
        store: outcome.store,
        config,
        warnings,
    })
}

fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(path)
}

fn sort_options(invocation: &Invocation) -> SortOptions {
    let mut options = SortOptions::default();
    if let Some(sort) = invocation.flags.sort {
        options.column = match sort {
            SortArg::Title => SortColumn::Title,
            SortArg::Modified => SortColumn::DateModified,
            SortArg::Created => SortColumn::DateCreated,
        };
        // An explicit column means ascending intent; the upstream
        // default (date modified, reversed) only applies when no
        // column was named. --reverse flips whichever column.
        options.reverse = false;
    }
    if invocation.flags.reverse {
        options.reverse = !options.reverse;
    }
    options
}

/// The filtered, sorted view: one `FilterState` fresh over the store
/// (the incremental resume machinery is the UI's concern; the CLI
/// always filters a whole query). An empty query filters nothing
/// (invariant 2), so bare `search` behaves like `list`.
fn filter_view(
    invocation: &Invocation,
    store: &Store,
) -> Result<Vec<usize>, commonplace_core::query::QueryError> {
    let query = invocation.positionals.join(" ");
    let mut state = FilterState::new(sort_options(invocation));
    state.update(&query, store.notes())
}

/// Resolve a query to one note's arena index: the Phase 3 selection
/// rule (first title-prefix match, prefix parent wins). `selection()`
/// answers with a position in the visible list, not an arena index,
/// so the visible list carries the mapping.
fn resolved(store: &Store, query: &str) -> Option<usize> {
    let mut state = FilterState::new(SortOptions::default());
    let visible = state.update(query, store.notes()).ok()?;
    let position = state.selection(store.notes())?;
    visible.get(position).copied()
}

fn note_record(store: &Store, index: usize) -> Vec<(&'static str, Json)> {
    let note = &store.notes()[index];
    let relative = note
        .path
        .strip_prefix(store.folder())
        .unwrap_or(&note.path)
        .to_string_lossy()
        .into_owned();
    vec![
        ("title", Json::Str(note.display_title().to_owned())),
        ("path", Json::Str(relative)),
    ]
}

// -- verbs ----------------------------------------------------------------------

fn search(env: &Env, invocation: &Invocation) -> VerbOutput {
    search_labeled(env, invocation, "search")
}

/// `list` shares search's machinery; only the envelope's verb tag
/// differs.
fn list(env: &Env, invocation: &Invocation) -> VerbOutput {
    let mut inv = invocation.clone();
    inv.positionals.clear();
    search_labeled(env, &inv, "list")
}

fn search_labeled(env: &Env, invocation: &Invocation, label: &'static str) -> VerbOutput {
    let ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let stderr = ctx.warnings.clone();
    // --saved resolves the query from the config's hotkey slot store;
    // an unset slot is a usage error, and it excludes a typed query.
    let query = if let Some(slot) = invocation.flags.saved {
        if !invocation.positionals.is_empty() {
            return VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec!["search: --saved excludes a typed query".to_owned()],
                json: None,
            };
        }
        let key = slot.to_string();
        match ctx.config.saved_query(&key) {
            Some(query) => query.to_owned(),
            None => {
                return VerbOutput {
                    exit: Exit::Usage,
                    stdout: String::new(),
                    stderr: vec![format!("search: saved search slot {key} is not set")],
                    json: None,
                };
            }
        }
    } else {
        invocation.positionals.join(" ")
    };
    let view = {
        let mut state = FilterState::new(sort_options(invocation));
        match state.update(&query, ctx.store.notes()) {
            Ok(view) => view,
            Err(e) => return VerbOutput::trouble(vec![format!("query error: {e}")]),
        }
    };
    if invocation.flags.json {
        let matches: Vec<Json> = view
            .iter()
            .map(|&i| Json::Obj(note_record(&ctx.store, i)))
            .collect();
        let json = envelope(
            label,
            vec![
                ("count", Json::Int(matches.len() as i64)),
                ("matches", Json::Arr(matches)),
            ],
        );
        VerbOutput {
            exit: Exit::Ok,
            stdout: json.render(),
            stderr,
            json: Some(json),
        }
    } else {
        let mut lines = Vec::new();
        for index in view {
            let note = &ctx.store.notes()[index];
            let relative = note
                .path
                .strip_prefix(ctx.store.folder())
                .unwrap_or(&note.path)
                .to_string_lossy()
                .into_owned();
            lines.push(format!("{}\t{}", note.display_title(), relative));
        }
        lines.extend(stderr);
        VerbOutput::ok(lines.join("\n") + if lines.is_empty() { "" } else { "\n" })
    }
}

fn open(env: &Env, invocation: &Invocation) -> VerbOutput {
    let ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let query = invocation.positionals.join(" ");
    match resolved(&ctx.store, &query) {
        Some(index) => VerbOutput::ok(ctx.store.notes()[index].path.display().to_string() + "\n"),
        None => VerbOutput::trouble(vec![format!("no note matches `{query}`")]),
    }
}

fn new(env: &Env, invocation: &Invocation) -> VerbOutput {
    let mut ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let title = invocation.positionals.join(" ");
    let body = match body_input(invocation, &invocation.flags.body) {
        Ok(body) => body,
        Err(output) => return output,
    };
    match ctx.store.create_note(&title, &body) {
        Ok(note) => VerbOutput::ok(note.path.display().to_string() + "\n"),
        Err(e) => match e {
            commonplace_core::store::WriteError::EmptyTitle => VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec!["new: the title is empty".to_owned()],
                json: None,
            },
            other => VerbOutput::trouble(vec![format!("create failed: {other}")]),
        },
    }
}

fn append(env: &Env, invocation: &Invocation) -> VerbOutput {
    let mut ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let query = invocation.positionals.join(" ");
    let text = match body_input(invocation, &invocation.flags.text) {
        Ok(text) => text,
        Err(output) => return output,
    };
    let Some(index) = resolved(&ctx.store, &query) else {
        return VerbOutput::trouble(vec![format!("no note matches `{query}`")]);
    };
    let path = ctx.store.notes()[index].path.clone();
    let existing = ctx.store.notes()[index].body.as_str().to_owned();
    let mut body = existing;
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str(&text);
    if !body.ends_with('\n') {
        body.push('\n');
    }
    match ctx.store.save_note(&path, &body) {
        Ok(_) => VerbOutput::ok(String::new()),
        Err(e) => VerbOutput::trouble(vec![format!("append failed: {e}")]),
    }
}

fn export(env: &Env, invocation: &Invocation) -> VerbOutput {
    let ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let format = match invocation.flags.format.as_deref() {
        None | Some("text") => ExportFormat::PlainText,
        Some("html") => ExportFormat::Html,
        Some(other) => {
            return VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec![format!("--format must be text or html, not `{other}`")],
                json: None,
            };
        }
    };
    let out_dir = match &invocation.flags.out {
        Some(out) => expand_tilde(out, &env.home),
        None => PathBuf::from("commonplace-export"),
    };
    let query = invocation.positionals.join(" ");
    let result = if query.is_empty() {
        export_store_to_dir(&ctx.store, &out_dir, format, &ComrakRenderer)
    } else {
        let view = match filter_view(invocation, &ctx.store) {
            Ok(view) => view,
            Err(e) => return VerbOutput::trouble(vec![format!("query error: {e}")]),
        };
        let mut written = Vec::new();
        let mut skipped = Vec::new();
        for index in view {
            let note = ctx.store.notes()[index].clone();
            match export_note_to_dir(&ctx.store, &note, &out_dir, format, &ComrakRenderer) {
                Ok(path) => written.push(path),
                Err(e) => skipped.push(commonplace_core::export::SkippedExport {
                    title: note.display_title().to_owned(),
                    reason: format!("{e:?}"),
                }),
            }
        }
        Ok(commonplace_core::export::ExportOutcome { written, skipped })
    };
    match result {
        Ok(outcome) => {
            let count = outcome.written.len();
            let skipped = outcome.skipped.len();
            if invocation.flags.json {
                let json = envelope(
                    "export",
                    vec![
                        ("written", Json::Int(count as i64)),
                        ("skipped", Json::Int(skipped as i64)),
                        ("out_dir", Json::Str(out_dir.display().to_string())),
                    ],
                );
                VerbOutput {
                    exit: Exit::Ok,
                    stdout: json.render(),
                    stderr: outcome
                        .skipped
                        .iter()
                        .map(|s| format!("warning: {}: {}", s.title, s.reason))
                        .collect(),
                    json: Some(json),
                }
            } else {
                let mut lines = vec![format!("wrote {count} notes to {}", out_dir.display())];
                if skipped > 0 {
                    lines.push(format!("skipped {skipped}"));
                }
                VerbOutput::ok(lines.join("\n") + "\n")
            }
        }
        Err(e) => VerbOutput::trouble(vec![format!("export failed: {e:?}")]),
    }
}

fn tags(env: &Env, invocation: &Invocation) -> VerbOutput {
    let ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let stderr = ctx.warnings.clone();
    let index = TagIndex::build(ctx.store.notes());
    let tags = index.tags();
    if invocation.flags.json {
        let payload: Vec<Json> = tags
            .iter()
            .map(|t| {
                Json::Obj(vec![
                    ("tag", Json::Str((*t).to_owned())),
                    ("notes", Json::Int(index.notes_for(t).len() as i64)),
                ])
            })
            .collect();
        let json = envelope(
            "tags",
            vec![
                ("count", Json::Int(payload.len() as i64)),
                ("tags", Json::Arr(payload)),
            ],
        );
        VerbOutput {
            exit: Exit::Ok,
            stdout: json.render(),
            stderr,
            json: Some(json),
        }
    } else {
        let mut lines: Vec<String> = tags
            .iter()
            .map(|t| format!("{}\t{}", t, index.notes_for(t).len()))
            .collect();
        lines.extend(stderr);
        VerbOutput::ok(lines.join("\n") + if lines.is_empty() { "" } else { "\n" })
    }
}

fn cat(env: &Env, invocation: &Invocation) -> VerbOutput {
    let ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let query = invocation.positionals.join(" ");
    match resolved(&ctx.store, &query) {
        Some(index) => {
            let body = ctx.store.notes()[index].body.as_str().to_owned();
            VerbOutput::ok(body)
        }
        None => VerbOutput::trouble(vec![format!("no note matches `{query}`")]),
    }
}

fn render(env: &Env, invocation: &Invocation) -> VerbOutput {
    let ctx = match context(env, invocation) {
        Ok(ctx) => ctx,
        Err(output) => return output,
    };
    let query = invocation.positionals.join(" ");
    let Some(index) = resolved(&ctx.store, &query) else {
        return VerbOutput::trouble(vec![format!("no note matches `{query}`")]);
    };
    let note = &ctx.store.notes()[index];
    let expanded = expand_transclusions_rooted(
        note.body.as_str(),
        Some(note.display_title()),
        &|interior| {
            let key = commonplace_core::fold::fold(interior);
            ctx.store
                .notes()
                .iter()
                .find(|n| commonplace_core::fold::fold(n.display_title()) == key)
                .map(|n| n.body.as_str().to_owned())
        },
    );
    VerbOutput::ok(ComrakRenderer.render_markdown(&expanded))
}

fn config(env: &Env, invocation: &Invocation) -> VerbOutput {
    // The config verb never opens the notes folder: `--set notes_dir`
    // is exactly how a missing folder gets configured, so only the
    // config file itself is read here.
    let flags = &invocation.flags;
    let mut config = match Config::load(&env.config_path) {
        Ok(config) => config,
        Err(e) => return VerbOutput::trouble(vec![format!("{e}")]),
    };
    if let Some(key) = &flags.get {
        // The key vocabulary is validated for reads too (spec §6:
        // unknown config keys are usage errors); a valid but unset
        // slot reads empty.
        if let Err(reason) = validate_key(key, None) {
            return VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec![reason],
                json: None,
            };
        }
        return match get_value(&config, key) {
            Some(value) => VerbOutput::ok(value + "\n"),
            None => VerbOutput::ok(String::new()),
        };
    }
    if let Some((key, value)) = &flags.set {
        if let Err(reason) = validate_key(key, Some(value)) {
            return VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec![reason],
                json: None,
            };
        }
        apply_key(&mut config, key, Some(value.clone()));
        let path = env.config_path.clone();
        return match config.save(&path) {
            Ok(()) => VerbOutput::ok(format!("wrote {}\n", path.display())),
            Err(e) => VerbOutput::trouble(vec![format!("{e}")]),
        };
    }
    if let Some(key) = &flags.unset {
        if let Err(reason) = validate_key(key, None) {
            return VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec![reason],
                json: None,
            };
        }
        apply_key(&mut config, key, None);
        let path = env.config_path.clone();
        return match config.save(&path) {
            Ok(()) => VerbOutput::ok(format!("wrote {}\n", path.display())),
            Err(e) => VerbOutput::trouble(vec![format!("{e}")]),
        };
    }
    // Bare `config`: the effective config as the writer emits it.
    VerbOutput::ok(commonplace_core::config::write_config(&config))
}

/// `notes_dir` and the saved slots `saved.0`-`saved.9` are the whole
/// key vocabulary; anything else is usage (spec §6).
fn validate_key(key: &str, value: Option<&String>) -> Result<(), String> {
    if key == "notes_dir" {
        return Ok(());
    }
    if let Some(slot) = key.strip_prefix("saved.")
        && commonplace_core::config::SAVED_SLOTS.contains(&slot)
    {
        if let Some(v) = value {
            if v.is_empty() {
                return Err("config: saved search queries cannot be empty (use --unset)".to_owned());
            }
            if v.chars().any(|c| c == '\n' || c == '\r') {
                return Err("config: saved search queries cannot contain newlines".to_owned());
            }
        }
        return Ok(());
    }
    Err(format!(
        "config: unknown key `{key}` (known: notes_dir, saved.0-9)"
    ))
}

fn apply_key(config: &mut Config, key: &str, value: Option<String>) {
    if key == "notes_dir" {
        config.notes_dir = value;
        return;
    }
    let slot = key.strip_prefix("saved.").unwrap().to_owned();
    match value {
        Some(query) => {
            if let Some(entry) = config.saved_searches.iter_mut().find(|(k, _)| *k == slot) {
                entry.1 = query;
            } else {
                config.saved_searches.push((slot, query));
            }
        }
        None => config.saved_searches.retain(|(k, _)| *k != slot),
    }
}

fn get_value(config: &Config, key: &str) -> Option<String> {
    if key == "notes_dir" {
        return Some(
            config
                .notes_dir
                .clone()
                .unwrap_or_else(|| commonplace_core::config::DEFAULT_NOTES_DIR.to_owned()),
        );
    }
    let slot = key.strip_prefix("saved.")?;
    config.saved_query(slot).map(str::to_owned)
}

fn completions(invocation: &Invocation) -> VerbOutput {
    let shell = invocation.positionals[0].as_str();
    match shell {
        "bash" | "zsh" | "fish" => VerbOutput::ok(crate::completions::script(shell)),
        other => VerbOutput {
            exit: Exit::Usage,
            stdout: String::new(),
            stderr: vec![format!(
                "completions: unknown shell `{other}` (bash, zsh, fish)"
            )],
            json: None,
        },
    }
}

fn scheme(env: &Env, invocation: &Invocation) -> VerbOutput {
    let action = invocation.positionals[0].as_str();
    match action {
        "find" => {
            let terms = invocation.positionals[1..].join(" ");
            let mut inv = invocation.clone();
            inv.positionals = vec![terms];
            open(env, &inv)
        }
        "make" => {
            let Some(title) = invocation.flags.title.clone() else {
                return VerbOutput {
                    exit: Exit::Usage,
                    stdout: String::new(),
                    stderr: vec!["scheme make: --title is required".to_owned()],
                    json: None,
                };
            };
            if invocation.flags.html.is_some() || invocation.flags.url.is_some() || invocation.flags.tags.is_some() {
                return VerbOutput::trouble(vec![
                    "scheme make: --html/--url/--tags land with the import phase (roadmap Phase 21); only --txt is functional".to_owned(),
                ]);
            }
            let mut inv = invocation.clone();
            inv.positionals = vec![title];
            inv.flags.body = invocation.flags.txt.clone();
            new(env, &inv)
        }
        "markdownify" => VerbOutput::trouble(vec![
            "scheme markdownify: the HTML-to-Markdown converter lands with the import phase (roadmap Phase 21)".to_owned(),
        ]),
        other => VerbOutput {
            exit: Exit::Usage,
            stdout: String::new(),
            stderr: vec![format!(
                "scheme: unknown action `{other}` (find, make, markdownify)"
            )],
            json: None,
        },
    }
}

/// Body text from a flag or stdin; exactly one source, and `--stdin`
/// with no flag source reads the stream.
fn body_input(invocation: &Invocation, flag_value: &Option<String>) -> Result<String, VerbOutput> {
    if invocation.flags.stdin {
        if flag_value.is_some() {
            return Err(VerbOutput {
                exit: Exit::Usage,
                stdout: String::new(),
                stderr: vec!["--stdin excludes --body/--text".to_owned()],
                json: None,
            });
        }
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| VerbOutput::trouble(vec![format!("stdin read failed: {e}")]))?;
        return Ok(buffer);
    }
    Ok(flag_value.clone().unwrap_or_default())
}
