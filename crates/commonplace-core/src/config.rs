//! The config surface (spec §6): `~/.config/commonplace/config.toml`
//! parsed and written by a hand-rolled TOML subset standing in for
//! upstream's NSKeyedArchiver/NSUserDefaults plumbing (crosswalk §11).
//! The grammar is deliberately tiny: `key = "quoted value"` lines, one
//! section header (`[saved_searches]`), `#` comments, and exactly
//! three escapes (`\\`, `\"`, `\n`). No serde; the engine stays
//! dependency-free.

use std::fs;
use std::path::{Path, PathBuf};

/// The default notes folder [CP; upstream's default was an
/// app-support dir]: `~` expands against `$HOME` at use time.
pub const DEFAULT_NOTES_DIR: &str = "~/Documents/Notes";

/// The saved-search hotkey slots (nvUltra's ⌘1-⌘9/⌘0, Phase 9; shared
/// with the app's Phase 17).
pub const SAVED_SLOTS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// The parsed config. `extra_keys` holds top-level keys this version
/// does not recognize; the writer re-emits them in read order so a
/// `--set` never drops foreign keys.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    pub notes_dir: Option<String>,
    /// `(slot, query)` pairs in read order; slots are `"0"`-`"9"`.
    pub saved_searches: Vec<(String, String)>,
    /// Launch restore (spec row 73, AC:2174-2190): the last search and
    /// the note it led to, written by the app.
    pub last_query: Option<String>,
    pub last_note: Option<String>,
    pub extra_keys: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum ConfigError {
    Parse { line: usize, reason: String },
    Io(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Parse { line, reason } => {
                write!(f, "config parse error at line {line}: {reason}")
            }
            ConfigError::Io(e) => write!(f, "config I/O error: {e}"),
        }
    }
}

/// Parse the subset. Unknown top-level keys land in `extra_keys`;
/// unknown sections, unquoted values, unknown escapes, duplicate keys,
/// and out-of-vocabulary saved-search slots are parse errors.
pub fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let mut config = Config::default();
    let mut section = Section::Top;
    let err = |line: usize, reason: &str| ConfigError::Parse {
        line,
        reason: reason.to_owned(),
    };

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[') {
            let Some(name) = inner.strip_suffix(']') else {
                return Err(err(line_no, "unterminated section header"));
            };
            if name != "saved_searches" {
                return Err(err(line_no, &format!("unknown section [{name}]")));
            }
            section = Section::Saved;
            continue;
        }
        let Some(eq) = line.find('=') else {
            return Err(err(line_no, "expected key = \"value\""));
        };
        let key = line[..eq].trim();
        let key = if let Some(q) = key.strip_prefix('"') {
            let Some(end) = q.strip_suffix('"') else {
                return Err(err(line_no, "unterminated quoted key"));
            };
            end.to_owned()
        } else {
            key.to_owned()
        };
        if key.is_empty() {
            return Err(err(line_no, "empty key"));
        }
        let value_src = line[eq + 1..].trim();
        let value = parse_quoted(value_src).map_err(|reason| err(line_no, &reason))?;
        match section {
            Section::Top => {
                if key == "notes_dir" {
                    if config.notes_dir.replace(value).is_some() {
                        return Err(err(line_no, "duplicate key notes_dir"));
                    }
                } else if key == "last_query" || key == "last_note" {
                    let slot = if key == "last_query" {
                        &mut config.last_query
                    } else {
                        &mut config.last_note
                    };
                    if slot.replace(value).is_some() {
                        return Err(err(line_no, &format!("duplicate key {key}")));
                    }
                } else {
                    config.extra_keys.push((key, value));
                }
            }
            Section::Saved => {
                if !SAVED_SLOTS.contains(&key.as_str()) {
                    return Err(err(line_no, "saved_searches keys must be the slots 0-9"));
                }
                if config.saved_searches.iter().any(|(k, _)| *k == key) {
                    return Err(err(line_no, &format!("duplicate key {key}")));
                }
                config.saved_searches.push((key, value));
            }
        }
    }
    Ok(config)
}

#[derive(PartialEq)]
enum Section {
    Top,
    Saved,
}

/// Parse one quoted value from the trimmed remainder of a line: the
/// value must open with `"`, honor the three escapes, close with `"`,
/// and carry nothing after it but whitespace and an optional comment.
fn parse_quoted(src: &str) -> Result<String, String> {
    let Some(rest) = src.strip_prefix('"') else {
        return Err("values must be quoted strings".to_owned());
    };
    let mut out = String::new();
    let mut chars = rest.chars();
    loop {
        let Some(c) = chars.next() else {
            return Err("unterminated quoted value".to_owned());
        };
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some(other) => {
                    return Err(format!("unknown escape \\{other}"));
                }
                None => return Err("unterminated quoted value".to_owned()),
            },
            _ => out.push(c),
        }
    }
    let tail = chars.as_str().trim();
    if !tail.is_empty() && !tail.starts_with('#') {
        return Err("unexpected text after the closing quote".to_owned());
    }
    Ok(out)
}

/// Serialize back to the subset: `notes_dir` first, then foreign keys
/// in read order, then the `saved_searches` section in slot order.
pub fn write_config(config: &Config) -> String {
    let mut out = String::new();
    let quote = |value: &str| -> String {
        let mut q = String::with_capacity(value.len() + 2);
        q.push('"');
        for c in value.chars() {
            match c {
                '\\' => q.push_str("\\\\"),
                '"' => q.push_str("\\\""),
                '\n' => q.push_str("\\n"),
                _ => q.push(c),
            }
        }
        q.push('"');
        q
    };
    if let Some(dir) = &config.notes_dir {
        out.push_str(&format!("notes_dir = {}\n", quote(dir)));
    }
    if let Some(query) = &config.last_query {
        out.push_str(&format!("last_query = {}\n", quote(query)));
    }
    if let Some(note) = &config.last_note {
        out.push_str(&format!("last_note = {}\n", quote(note)));
    }
    for (key, value) in &config.extra_keys {
        out.push_str(&format!("{key} = {}\n", quote(value)));
    }
    if !config.saved_searches.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("[saved_searches]\n");
        let mut slots: Vec<&(String, String)> = config.saved_searches.iter().collect();
        slots.sort_by(|a, b| a.0.cmp(&b.0));
        for (slot, query) in slots {
            out.push_str(&format!("{slot} = {}\n", quote(query)));
        }
    }
    out
}

impl Config {
    /// The config file's location: `$COMMONPLACE_CONFIG` overrides,
    /// else the XDG default `~/.config/commonplace/config.toml`
    /// (`$XDG_CONFIG_HOME` respected). The CLI and the app share it.
    pub fn xdg_path() -> PathBuf {
        if let Ok(override_path) = std::env::var("COMMONPLACE_CONFIG")
            && !override_path.is_empty()
        {
            return PathBuf::from(override_path);
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return Path::new(&xdg).join("commonplace/config.toml");
        }
        match std::env::var("HOME") {
            Ok(home) if !home.is_empty() => {
                Path::new(&home).join(".config/commonplace/config.toml")
            }
            _ => PathBuf::from(".config/commonplace/config.toml"),
        }
    }

    /// Load the config file; a missing file is not an error (spec §6:
    /// defaults apply). Parse and I/O failures are.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        match fs::read_to_string(path) {
            Ok(text) => parse_config(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    /// Write the config atomically-enough for a config file: create
    /// the parent, then write. Note files carry the atomicity
    /// invariant; this does not.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        fs::write(path, write_config(self)).map_err(ConfigError::Io)
    }

    /// The effective notes folder: the configured value or the
    /// default, with a leading `~` expanded against `$HOME` (an unset
    /// `$HOME` leaves the literal path, documented in spec §6).
    pub fn notes_dir(&self) -> PathBuf {
        expand_tilde(self.notes_dir.as_deref().unwrap_or(DEFAULT_NOTES_DIR))
    }

    /// The saved query in one hotkey slot, if set.
    pub fn saved_query(&self, slot: &str) -> Option<&str> {
        self.saved_searches
            .iter()
            .find(|(k, _)| k == slot)
            .map(|(_, v)| v.as_str())
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return home_or(Path::new(path));
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var("HOME").ok().filter(|home| !home.is_empty())
    {
        return Path::new(&home).join(rest);
    }
    PathBuf::from(path)
}

fn home_or(fallback: &Path) -> PathBuf {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => PathBuf::from(home),
        _ => fallback.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_pinned_shape() {
        let config = parse_config(
            "# comment\nnotes_dir = \"/home/me/Notes\"\n\n[saved_searches]\n\"1\" = \"query one\"\n\"0\" = \"#tagged\"\n",
        )
        .unwrap();
        assert_eq!(config.notes_dir.as_deref(), Some("/home/me/Notes"));
        assert_eq!(
            config.saved_searches,
            vec![
                ("1".to_owned(), "query one".to_owned()),
                ("0".to_owned(), "#tagged".to_owned()),
            ]
        );
    }

    #[test]
    fn round_trips_keys_and_values() {
        let config = parse_config(
            "notes_dir = \"a\\\\b \\\"q\\\"\"\n\n[saved_searches]\n\"3\" = \"line\\nbreak\"\n",
        )
        .unwrap();
        assert_eq!(
            config.notes_dir.as_deref(),
            Some("a\\b \"q\"".to_string().as_str())
        );
        assert_eq!(config.saved_searches[0].1, "line\nbreak");
        let written = write_config(&config);
        let reparsed = parse_config(&written).unwrap();
        assert_eq!(config, reparsed);
    }

    #[test]
    fn foreign_keys_survive_a_set_round_trip() {
        let config = parse_config("notes_dir = \"/n\"\ncustom_thing = \"kept\"\n").unwrap();
        assert_eq!(
            config.extra_keys,
            vec![("custom_thing".to_owned(), "kept".to_owned())]
        );
        let reparsed = parse_config(&write_config(&config)).unwrap();
        assert_eq!(config, reparsed);
    }

    #[test]
    fn missing_file_is_defaults_not_an_error() {
        let config = Config::load(Path::new("/nonexistent/commonplace/config.toml")).unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(
            config.notes_dir(),
            expand_tilde(DEFAULT_NOTES_DIR),
            "the default notes dir applies"
        );
    }

    #[test]
    fn malformed_configs_are_parse_errors_with_line_numbers() {
        let e = parse_config("notes_dir = unquoted\n").unwrap_err();
        assert!(matches!(e, ConfigError::Parse { line: 1, .. }));
        let e = parse_config("[other]\nx = \"1\"\n").unwrap_err();
        assert!(matches!(e, ConfigError::Parse { line: 1, .. }));
        let e = parse_config("[saved_searches]\n\"12\" = \"q\"\n").unwrap_err();
        assert!(matches!(e, ConfigError::Parse { line: 2, .. }));
        let e = parse_config("notes_dir = \"a\"\nnotes_dir = \"b\"\n").unwrap_err();
        assert!(matches!(e, ConfigError::Parse { line: 2, .. }));
        let e = parse_config("notes_dir = \"a\\x\"\n").unwrap_err();
        assert!(matches!(e, ConfigError::Parse { line: 1, .. }));
    }

    #[test]
    fn tilde_expands_against_home() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/test".to_owned());
        let config = parse_config("notes_dir = \"~/Documents/Notes\"\n").unwrap();
        assert_eq!(
            config.notes_dir(),
            PathBuf::from(home).join("Documents/Notes")
        );
    }
}
