//! End-to-end CLI verb tests (spec §6): every verb through the real
//! parser and the real engine over a temp notes folder, pinning the
//! exit-code table and the `--json` envelope shapes.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use commonplace_cli::args;
use commonplace_cli::verbs::{Env, Exit, run_in};

/// A scratch notes folder plus a scratch config, both removed on drop.
/// The config path is forced through `COMMONPLACE_CONFIG` so the tests
/// never touch the developer's real config.
struct Sandbox {
    notes: PathBuf,
    config: PathBuf,
    env: Env,
}

impl Sandbox {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "commonplace-cli-golden-{}-{}-{}",
            tag,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let notes = std::env::temp_dir().join(format!("{unique}-notes"));
        fs::create_dir_all(&notes).unwrap();
        let config = std::env::temp_dir().join(format!("{unique}-config.toml"));
        let env = Env {
            config_path: config.clone(),
            home: notes.parent().unwrap().to_path_buf(),
        };
        // The sandbox config points at the sandbox notes folder.
        fs::write(&config, format!("notes_dir = \"{}\"\n", notes.display())).unwrap();
        Sandbox { notes, config, env }
    }

    fn write_note(&self, name: &str, contents: &str) {
        fs::write(self.notes.join(name), contents).unwrap();
    }

    fn run(&self, argv: &[&str]) -> (Exit, String, Vec<String>) {
        let owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
        let invocation = match args::parse(&owned) {
            Ok(inv) => inv,
            Err(e) => {
                // The binary maps parse errors to usage exit 2.
                return (Exit::Usage, String::new(), vec![format!("{e}")]);
            }
        };
        let output = run_in(&self.env.clone(), &invocation);
        (output.exit, output.stdout.clone(), output.stderr.clone())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.notes);
        let _ = fs::remove_file(&self.config);
        let stem = self
            .notes
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        for suffix in ["export-out", "export-html", "export-json"] {
            let _ = fs::remove_dir_all(std::env::temp_dir().join(format!("{stem}-{suffix}")));
        }
    }
}

impl Sandbox {
    /// A uniquely-named scratch dir outside the notes folder, cleaned
    /// with the sandbox (the temp parent is shared by every sandbox).
    fn scratch(&self, suffix: &str) -> PathBuf {
        let unique = self
            .notes
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let path = std::env::temp_dir().join(format!("{unique}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}

fn seeded(tag: &str) -> Sandbox {
    let sandbox = Sandbox::new(tag);
    sandbox.write_note("Alpha.md", "Alpha\n\nalpha body\n");
    sandbox.write_note("Beta.md", "Beta\n\nbeta body #project\n");
    sandbox.write_note("Gamma.md", "Gamma\n\ngamma body\n");
    sandbox
}

// -- the exit-code table (spec §6) ---------------------------------------------

#[test]
fn exit_code_table() {
    let sandbox = seeded("exit-codes");

    // 0: report verbs succeed, empty or not.
    let (exit, stdout, _) = sandbox.run(&["search", "alpha"]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("Alpha"), "{stdout}");
    let (exit, _, _) = sandbox.run(&["search", "nomatch-xyz"]);
    assert_eq!(exit, Exit::Ok, "empty is a valid answer for search");
    let (exit, _, _) = sandbox.run(&["list"]);
    assert_eq!(exit, Exit::Ok);
    let (exit, _, _) = sandbox.run(&["tags"]);
    assert_eq!(exit, Exit::Ok);
    let (exit, stdout, _) = sandbox.run(&["cat", "alpha"]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("alpha body"), "{stdout}");

    // 1: trouble - failed resolutions where a target was required.
    let (exit, _, stderr) = sandbox.run(&["open", "nomatch-xyz"]);
    assert_eq!(exit, Exit::Trouble);
    assert_eq!(stderr.len(), 1, "{stderr:?}");
    let (exit, _, _) = sandbox.run(&["cat", "nomatch-xyz"]);
    assert_eq!(exit, Exit::Trouble);
    let (exit, _, _) = sandbox.run(&["render", "nomatch-xyz"]);
    assert_eq!(exit, Exit::Trouble);
    let (exit, _, _) = sandbox.run(&["append", "nomatch-xyz", "--text", "x"]);
    assert_eq!(exit, Exit::Trouble);
    // ...and a missing notes folder is trouble for every verb.
    let (exit, _, _) = sandbox.run(&["list", "--dir", "/nonexistent-folder-xyz"]);
    assert_eq!(exit, Exit::Trouble);

    // 2: usage - unknown verbs, flags, and argument shapes.
    let (exit, _, stderr) = sandbox.run(&["frobnicate"]);
    assert_eq!(exit, Exit::Usage);
    assert_eq!(stderr.len(), 1, "{stderr:?}");
    let (exit, _, _) = sandbox.run(&["list", "--wat"]);
    assert_eq!(exit, Exit::Usage);
    let (exit, _, _) = sandbox.run(&["new", "   "]);
    assert_eq!(exit, Exit::Usage, "an empty title is usage");
    let (exit, _, _) = sandbox.run(&["completions", "tcsh"]);
    assert_eq!(exit, Exit::Usage);
    let (exit, _, _) = sandbox.run(&["config", "--get", "nope"]);
    assert_eq!(exit, Exit::Usage);
    let (exit, _, _) = sandbox.run(&["open"]);
    assert_eq!(exit, Exit::Usage);
}

#[test]
fn new_creates_and_cat_reads_back() {
    let sandbox = Sandbox::new("new-cat");
    let (exit, stdout, _) = sandbox.run(&["new", "Fresh Note", "--body", "first line\n\nbody\n"]);
    assert_eq!(exit, Exit::Ok);
    let created = PathBuf::from(stdout.trim());
    assert_eq!(created.parent().unwrap(), sandbox.notes);
    assert_eq!(
        fs::read_to_string(&created).unwrap(),
        "Fresh Note\nfirst line\n\nbody\n"
    );

    let (exit, stdout, _) = sandbox.run(&["cat", "fresh"]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("first line"), "{stdout}");
}

#[test]
fn search_json_envelope_is_pinned() {
    let sandbox = seeded("search-json");
    let (exit, stdout, _) = sandbox.run(&["search", "alpha", "--json"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(
        stdout.trim_end(),
        "{\"verb\":\"search\",\"ok\":true,\"count\":1,\"matches\":[{\"title\":\"Alpha\",\"path\":\"Alpha.md\"}]}"
    );
}

#[test]
fn tags_json_envelope_is_pinned() {
    let sandbox = seeded("tags-json");
    let (exit, stdout, _) = sandbox.run(&["tags", "--json"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(
        stdout.trim_end(),
        "{\"verb\":\"tags\",\"ok\":true,\"count\":1,\"tags\":[{\"tag\":\"project\",\"notes\":1}]}"
    );
}

#[test]
fn list_json_envelope_is_pinned() {
    let sandbox = seeded("list-json");
    let (exit, stdout, _) = sandbox.run(&["list", "--json"]);
    assert_eq!(exit, Exit::Ok);
    // Every note appears as a title/path record; the count pins.
    assert!(stdout.contains("\"verb\":\"list\""), "{stdout}");
    assert!(stdout.contains("\"ok\":true"), "{stdout}");
    assert!(stdout.contains("\"count\":3"), "{stdout}");
}

#[test]
fn config_verb_round_trip_and_defaults() {
    let sandbox = Sandbox::new("config-verb");

    // Bare config prints the effective file (the sandbox seeds
    // notes_dir, so the writer re-emits it).
    let (exit, stdout, _) = sandbox.run(&["config"]);
    assert_eq!(exit, Exit::Ok);
    assert!(
        stdout.contains(&format!("notes_dir = \"{}\"", sandbox.notes.display())),
        "{stdout}"
    );

    // --set notes_dir writes the file; --get reads it back.
    let dir_arg = format!("{}", sandbox.notes.display());
    let (exit, _, _) = sandbox.run(&["config", "--set", "notes_dir", &dir_arg]);
    assert_eq!(exit, Exit::Ok);
    assert!(sandbox.config.exists(), "the config file was written");
    let (exit, stdout, _) = sandbox.run(&["config", "--get", "notes_dir"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(stdout.trim_end(), sandbox.notes.display().to_string());

    // Foreign keys survive a set.
    let (exit, _, _) = sandbox.run(&["config", "--set", "saved.1", "#project"]);
    assert_eq!(exit, Exit::Ok);
    let (exit, stdout, _) = sandbox.run(&["config", "--get", "saved.1"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(stdout.trim_end(), "#project");
    let written = fs::read_to_string(&sandbox.config).unwrap();
    assert!(written.contains("notes_dir"), "{written}");
    assert!(written.contains("[saved_searches]"), "{written}");

    // --unset removes the slot.
    let (exit, _, _) = sandbox.run(&["config", "--unset", "saved.1"]);
    assert_eq!(exit, Exit::Ok);
    let (exit, stdout, _) = sandbox.run(&["config", "--get", "saved.1"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(stdout, "", "an unset slot reads empty");
}

#[test]
fn search_saved_slot_uses_the_config_store() {
    let sandbox = seeded("saved");
    let (exit, _, _) = sandbox.run(&["config", "--set", "saved.2", "gamma"]);
    assert_eq!(exit, Exit::Ok);
    let (exit, stdout, _) = sandbox.run(&["search", "--saved", "2"]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("Gamma"), "{stdout}");
    let (exit, _, stderr) = sandbox.run(&["search", "--saved", "5"]);
    assert_eq!(exit, Exit::Usage, "an unset slot is a usage error");
    assert!(stderr[0].contains("slot 5"), "{stderr:?}");
}

#[test]
fn export_verb_writes_and_reports() {
    let sandbox = seeded("export");
    let out = sandbox.scratch("export-out");
    let out_arg = format!("{}", out.display());
    let (exit, stdout, _) = sandbox.run(&["export", "--out", &out_arg]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("wrote 3 notes"), "{stdout}");
    assert_eq!(fs::read_dir(&out).unwrap().count(), 3);

    let out_html = sandbox.scratch("export-html");
    let html_arg = format!("{}", out_html.display());
    let (exit, stdout, _) = sandbox.run(&["export", "--format", "html", "--out", &html_arg]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("wrote 3 notes"), "{stdout}");
    let html = fs::read_to_string(out_html.join("Alpha.html")).unwrap();
    assert!(html.contains("<title>Alpha</title>"), "{html}");

    // The JSON envelope pins (fresh dir: collisions accumulate).
    let out_json = sandbox.scratch("export-json");
    let json_arg = format!("{}", out_json.display());
    let (exit, stdout, _) = sandbox.run(&["export", "--out", &json_arg, "--json"]);
    assert_eq!(exit, Exit::Ok);
    assert!(
        stdout.contains("\"verb\":\"export\"")
            && stdout.contains("\"ok\":true")
            && stdout.contains("\"written\":3"),
        "{stdout}"
    );
}

#[test]
fn render_verb_outputs_html_through_comrak() {
    let sandbox = seeded("render");
    let (exit, stdout, _) = sandbox.run(&["render", "alpha"]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("<p>alpha body</p>"), "{stdout}");
    assert!(!stdout.contains("{{"), "{stdout}");
}

#[test]
fn scheme_routes_resolve_and_create() {
    let sandbox = seeded("scheme");

    // find resolves like open.
    let (exit, stdout, _) = sandbox.run(&["scheme", "find", "alpha"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(
        stdout.trim_end(),
        sandbox.notes.join("Alpha.md").display().to_string()
    );

    // make creates with --title/--txt.
    let (exit, stdout, _) = sandbox.run(&["scheme", "make", "--title", "Made", "--txt", "content"]);
    assert_eq!(exit, Exit::Ok);
    assert!(PathBuf::from(stdout.trim()).exists(), "{stdout}");

    // make with html/url/tags defers to the import phase (exit 1, clean line).
    let (exit, _, stderr) = sandbox.run(&["scheme", "make", "--title", "X", "--html", "<p>y</p>"]);
    assert_eq!(exit, Exit::Trouble);
    assert!(stderr[0].contains("Phase 21"), "{stderr:?}");

    // markdownify defers the same way.
    let (exit, _, stderr) = sandbox.run(&["scheme", "markdownify"]);
    assert_eq!(exit, Exit::Trouble);
    assert!(stderr[0].contains("Phase 21"), "{stderr:?}");

    // unknown action is usage.
    let (exit, _, _) = sandbox.run(&["scheme", "frob"]);
    assert_eq!(exit, Exit::Usage);
}

#[test]
fn completions_verb_emits_scripts() {
    let sandbox = Sandbox::new("completions");
    for shell in ["bash", "zsh", "fish"] {
        let (exit, stdout, _) = sandbox.run(&["completions", shell]);
        assert_eq!(exit, Exit::Ok);
        assert!(stdout.contains("search"), "{shell}: {stdout}");
    }
}

#[test]
fn resolution_hits_the_matching_note_not_the_first_note() {
    // Regression: selection() answers with a visible-list position;
    // resolving it as an arena index made every verb but `search`
    // silently target the first note in path order.
    let sandbox = seeded("resolution");
    let (exit, stdout, _) = sandbox.run(&["open", "beta"]);
    assert_eq!(exit, Exit::Ok);
    assert_eq!(
        stdout.trim_end(),
        sandbox.notes.join("Beta.md").display().to_string()
    );
    let (exit, stdout, _) = sandbox.run(&["cat", "beta"]);
    assert_eq!(exit, Exit::Ok);
    assert!(stdout.contains("beta body"), "{stdout}");
    let (exit, _, _) = sandbox.run(&["append", "beta", "--text", "extra line"]);
    assert_eq!(exit, Exit::Ok);
    let beta = fs::read_to_string(sandbox.notes.join("Beta.md")).unwrap();
    assert!(beta.ends_with("extra line\n"), "{beta}");
    let alpha = fs::read_to_string(sandbox.notes.join("Alpha.md")).unwrap();
    assert!(!alpha.contains("extra line"), "alpha untouched: {alpha}");
}

#[test]
fn sort_flags_change_list_order() {
    let sandbox = seeded("sort");
    let (exit, stdout, _) = sandbox.run(&["list", "--sort", "title"]);
    assert_eq!(exit, Exit::Ok);
    let titles: Vec<&str> = stdout
        .lines()
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(
        titles,
        vec!["Alpha", "Beta", "Gamma"],
        "title sort is folded byte order"
    );
    let (exit, stdout, _) = sandbox.run(&["list", "--sort", "title", "--reverse"]);
    assert_eq!(exit, Exit::Ok);
    let titles: Vec<&str> = stdout
        .lines()
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(titles, vec!["Gamma", "Beta", "Alpha"]);
}
