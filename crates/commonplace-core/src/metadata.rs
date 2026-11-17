//! MultiMarkdown metadata blocks (spec row 64, grammar pinned by
//! `research/format-briefs-mmd-simplenote.md` §1 against the MMD 6
//! guide and the v6/v7 scanners). The block is a note's first lines:
//! it starts at the very top (no blank line before it), keys follow
//! the MMD charset with `:` terminating the key, values absorb
//! continuation lines (indented, colon-less, or URL-bearing), the
//! first blank line ends the block, and optional `---`/`...` fences
//! give YAML compatibility. Keys are case-insensitive with spaces
//! stripped for lookup; duplicate keys are first-come-first-served.

/// One parsed metadata block: entries in document order, keys
/// normalized (lowercased, spaces dropped), values cleaned (whitespace
/// runs collapsed to single spaces, one trailing space trimmed,
/// backslash-at-end-of-line kept as a literal newline).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    entries: Vec<(String, String)>,
}

impl Metadata {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Look a key up under the normalization (case-insensitive,
    /// spaces dropped: `Base Header Level`, `base headerlevel`, and
    /// `baseheaderlevel` are the same key, per the MMD 6 guide).
    pub fn get(&self, key: &str) -> Option<&str> {
        let wanted = normalize_key(key);
        self.entries
            .iter()
            .find(|(k, _)| *k == wanted)
            .map(|(_, v)| v.as_str())
    }

    /// The `Title:` value, the standard key row 53 reads for the
    /// display-title override.
    pub fn title(&self) -> Option<&str> {
        self.get("title")
    }

    /// The `Tags:` value split on commas, trimmed, empties dropped:
    /// nvUltra's convention (formats §1.3; not an MMD standard key).
    pub fn tags(&self) -> Vec<String> {
        match self.get("tags") {
            Some(value) => value
                .split(',')
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty())
                .collect(),
            None => Vec::new(),
        }
    }

    fn insert_first_wins(&mut self, key: String, value: String) {
        if !self.entries.iter().any(|(k, _)| *k == key) {
            self.entries.push((key, value));
        }
    }
}

pub(crate) fn normalize_key(key: &str) -> String {
    key.to_lowercase().replace(' ', "")
}

/// The parse result: the block (possibly empty) and the char offset
/// where the note's real content begins (after the terminating blank
/// line or fence). `end_offset == 0` means no block at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMetadata {
    pub metadata: Metadata,
    pub end_offset: usize,
}

/// Is this line blank for our purposes (ends a block)?
fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// Does the line look like a meta key line (`key : value`)? The key
/// starts with an ASCII letter or digit, continues with the scanner
/// charset (letters, digits, underscore, space, NBSP, tab, hyphen,
/// dot - the docs understate dot/tab/NBSP; formats §1.2), and ends at
/// a colon. Returns the (key, value) split with surrounding
/// whitespace removed from both.
pub(crate) fn meta_line(line: &str) -> Option<(&str, &str)> {
    let mut chars = line.char_indices();
    let (_, first) = chars.next()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }
    let mut colon = None;
    for (i, c) in chars {
        if c == ':' {
            colon = Some(i);
            break;
        }
        let ok = c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ' ' | '\t' | '\u{a0}');
        if !ok {
            return None;
        }
    }
    let colon = colon?;
    let key = &line[..colon];
    let value = line[colon + 1..].trim();
    Some((key.trim_end(), value))
}

/// Is the line a fence (`---` or longer, nothing else)? Opens YAML
/// compatibility mode or closes a block.
fn is_fence(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == '-')
}

/// Is the line a `...` closer? The MMD guides accept it after a
/// block; Commonplace pins acceptance (formats §1.6's ambiguity note).
fn is_dot_fence(line: &str) -> bool {
    line == "..."
}

/// Is the line ITSELF a URL or email (the bare-URL rule, formats
/// section 1.1: "a line that is a URL or email is never metadata")?
/// Anchored: the line must start with a `scheme://` prefix or
/// `mailto:`. The URL pattern outranks the meta pattern, so
/// `https://example.com` never becomes a key named `https`.
fn is_bare_url(line: &str) -> bool {
    let first_run = line.split_whitespace().next().unwrap_or("");
    if let Some(rest) = first_run.strip_prefix("mailto:") {
        return !rest.is_empty();
    }
    match first_run.find("://") {
        Some(i) if i > 0 => first_run[..i]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')),
        _ => false,
    }
}

/// Is every colon in the line inside a URL or email run? The pinned
/// continuation rule (formats section 1.3, "whose only colon is
/// inside a URL or email"): split the line into whitespace-delimited
/// runs; a colon is a URL colon when its run carries a `scheme://`
/// marker or starts with `mailto:`. `CSS: https://x.com/s.css` fails
/// (its first colon's run is bare `CSS:`) and so starts a new key,
/// while `see https://x.y/z:80` passes (both colons sit in the URL
/// run) and continues.
fn colons_all_in_url_runs(line: &str) -> bool {
    let mut colons = line.match_indices(':').map(|(i, _)| i).peekable();
    if colons.peek().is_none() {
        return false;
    }
    for colon in colons {
        let run_start = line[..colon]
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i + 1)
            .unwrap_or(0);
        let run_end = line[colon..]
            .find(char::is_whitespace)
            .map(|i| colon + i)
            .unwrap_or(line.len());
        let run = &line[run_start..run_end];
        let is_url_run = run.contains("://") || run.starts_with("mailto:");
        if !is_url_run {
            return false;
        }
    }
    true
}

/// Does the line continue the current value? Per the scanners
/// (formats section 1.3): indented, containing no colon, or with
/// every colon inside a URL/email run.
fn continues_value(line: &str) -> bool {
    if line.starts_with(' ') || line.starts_with('\t') {
        return true;
    }
    if !line.contains(':') {
        return true;
    }
    colons_all_in_url_runs(line)
}

/// Clean a raw value: backslash-at-end-of-line becomes a literal
/// newline; every other whitespace run (spaces, tabs, the newlines
/// between continuation lines) collapses to one space; one trailing
/// space is trimmed. Quotes are not special; values are verbatim.
fn clean_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut pending_space = false;
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'\n') {
            chars.next();
            pending_space = false;
            out.push('\n');
            continue;
        }
        if c == '\n' || c == ' ' || c == '\t' || c == '\u{a0}' {
            pending_space = true;
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
    }
    // One trailing space trimmed (a collapsed run leaves at most one).
    if out.ends_with(' ') {
        out.truncate(out.len() - 1);
    }
    out
}

/// Parse a note body's metadata block. Never fails: a body without a
/// block parses as empty with `end_offset == 0`, and a malformed
/// line simply ends the block. `end_offset` is a BYTE offset where
/// the note's real content begins (after the terminating blank line
/// or fence); `0` means no block at all. CRLF bodies are handled:
/// the `\r` of a `\r\n` terminator belongs to the line ending, never
/// to the key or value.
pub fn parse_metadata(body: &str) -> ParsedMetadata {
    let mut metadata = Metadata::default();
    let mut current_key: Option<String> = None;
    let mut raw_value = String::new();
    let mut end_offset = 0usize;
    let mut any_meta = false;
    let mut yaml_mode = false;
    let mut mode_decided = false;

    let mut pos = 0usize;
    while pos < body.len() {
        // One raw line: up to (not including) the next \n or EOF.
        let nl = body[pos..].find('\n').map(|r| pos + r);
        let (raw, after) = match nl {
            Some(n) => (&body[pos..n], n + 1),
            None => (&body[pos..], body.len()),
        };
        // The \r of a \r\n terminator is line-ending, not content.
        let line = raw.strip_suffix('\r').unwrap_or(raw);

        if !mode_decided {
            mode_decided = true;
            if is_fence(line) {
                yaml_mode = true;
                pos = after;
                continue;
            }
            if is_blank(line) {
                // A blank first line: no block (formats section 1.1).
                return ParsedMetadata {
                    metadata,
                    end_offset: 0,
                };
            }
            if is_bare_url(line) {
                // A line that IS a URL or email is never metadata
                // (formats section 1.1; the URL pattern outranks the
                // meta pattern).
                return ParsedMetadata {
                    metadata,
                    end_offset: 0,
                };
            }
            match meta_line(line) {
                Some((key, value)) => {
                    any_meta = true;
                    current_key = Some(normalize_key(key));
                    raw_value = value.to_owned();
                    pos = after;
                    continue;
                }
                None => {
                    return ParsedMetadata {
                        metadata,
                        end_offset: 0,
                    };
                }
            }
        }

        if yaml_mode {
            if is_fence(line) || is_dot_fence(line) {
                end_offset = after;
                break;
            }
            if is_blank(line) {
                end_offset = after;
                break;
            }
            match meta_line(line) {
                Some((key, value)) => {
                    if let Some(open) = current_key.take() {
                        metadata.insert_first_wins(open, clean_value(&raw_value));
                    }
                    current_key = Some(normalize_key(key));
                    raw_value = value.to_owned();
                    pos = after;
                }
                None => {
                    // Continuations in YAML mode behave the same.
                    if current_key.is_some() {
                        raw_value.push('\n');
                        raw_value.push_str(line);
                        pos = after;
                    } else {
                        // Should not happen (YAML mode always has a
                        // current key after its first key line);
                        // treat as body.
                        end_offset = pos;
                        break;
                    }
                }
            }
            continue;
        }

        // Plain MMD mode.
        if is_blank(line) {
            end_offset = after;
            break;
        }
        if continues_value(line) {
            raw_value.push('\n');
            raw_value.push_str(line);
            pos = after;
            continue;
        }
        match meta_line(line) {
            Some((key, value)) => {
                if let Some(open) = current_key.take() {
                    metadata.insert_first_wins(open, clean_value(&raw_value));
                }
                current_key = Some(normalize_key(key));
                raw_value = value.to_owned();
                pos = after;
            }
            None => {
                // A non-continuation, non-meta line ends the block;
                // the content boundary is before this line.
                end_offset = pos;
                break;
            }
        }
    }

    // Flush the last key.
    if let Some(open) = current_key.take() {
        metadata.insert_first_wins(open, clean_value(&raw_value));
    }

    // No terminating blank line and no fence: at EOF the whole
    // consumed prefix is the block.
    if end_offset == 0 && any_meta {
        end_offset = body.len();
    }

    ParsedMetadata {
        metadata,
        end_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden blocks, written from formats section 1's tables.

    #[test]
    fn simple_block_with_blank_terminator() {
        let p = parse_metadata("Title: Foo\nAuthor: Bar\n\nthe body starts here");
        assert_eq!(p.metadata.title(), Some("Foo"));
        assert_eq!(p.metadata.get("Author"), Some("Bar"));
        assert!(p.metadata.get("Transclude Base").is_none());
        // The block ends after the blank line.
        assert_eq!(
            &"Title: Foo\nAuthor: Bar\n\nthe body starts here"[p.end_offset..],
            "the body starts here"
        );
    }

    #[test]
    fn keys_are_case_insensitive_and_space_stripped() {
        let p = parse_metadata("Base Header Level: 2\n\nbody");
        assert_eq!(p.metadata.get("Base Header Level"), Some("2"));
        assert_eq!(p.metadata.get("base headerlevel"), Some("2"));
        assert_eq!(p.metadata.get("baseheaderlevel"), Some("2"));
    }

    #[test]
    fn empty_value_is_legal() {
        let p = parse_metadata("tags:\n\nbody");
        assert_eq!(p.metadata.get("tags"), Some(""));
        assert_eq!(p.metadata.tags(), Vec::<String>::new());
    }

    #[test]
    fn tags_split_on_commas() {
        let p = parse_metadata("tags: startrek, tng , deep space nine\n\nbody");
        assert_eq!(p.metadata.tags(), ["startrek", "tng", "deep space nine"]);
    }

    #[test]
    fn duplicate_keys_first_come_first_served() {
        let p = parse_metadata("Title: A\nTitle: B\n\nbody");
        assert_eq!(p.metadata.title(), Some("A"));
        assert_eq!(p.metadata.len(), 1);
    }

    #[test]
    fn no_block_when_first_line_is_prose() {
        let p = parse_metadata("Hello world, this is not metadata\nTags: x\n");
        assert!(p.metadata.is_empty());
        assert_eq!(p.end_offset, 0);
    }

    #[test]
    fn blank_first_line_means_no_block() {
        let p = parse_metadata("\nTitle: Foo\n\nbody");
        assert!(p.metadata.is_empty());
        assert_eq!(p.end_offset, 0);
    }

    #[test]
    fn mid_document_key_lines_are_body() {
        let p = parse_metadata("Title: Real\n\nTags: not metadata, just text");
        assert_eq!(p.metadata.title(), Some("Real"));
        assert!(p.metadata.get("tags").is_none());
    }

    #[test]
    fn continuation_indented_line() {
        let p = parse_metadata("Title: the long form\n    continues here\n\nbody");
        assert_eq!(p.metadata.title(), Some("the long form continues here"));
    }

    #[test]
    fn continuation_colon_less_line() {
        let p = parse_metadata("Title: chapter one\nthe reawakening\n\nbody");
        assert_eq!(p.metadata.title(), Some("chapter one the reawakening"));
    }

    #[test]
    fn url_colon_line_continues() {
        let p =
            parse_metadata("Title: sources\nsee https://example.com/page and mailto:x@y.z\n\nbody");
        assert_eq!(
            p.metadata.title(),
            Some("sources see https://example.com/page and mailto:x@y.z")
        );
    }

    #[test]
    fn whitespace_runs_collapse_and_trailing_space_trims() {
        let p = parse_metadata("Title: a\n  b\t\tc  \n\nbody");
        assert_eq!(p.metadata.title(), Some("a b c"));
    }

    #[test]
    fn backslash_newline_is_a_literal_newline() {
        let p = parse_metadata("Author: Smith\\\nNotes\n\nbody");
        assert_eq!(p.metadata.get("Author"), Some("Smith\nNotes"));
    }

    #[test]
    fn quoted_values_are_verbatim() {
        let p = parse_metadata("Title: \"The \\quoted\\ one\"\n\nbody");
        assert_eq!(p.metadata.title(), Some("\"The \\quoted\\ one\""));
    }

    #[test]
    fn yaml_fence_mode() {
        let p = parse_metadata("---\nTitle: Fenced\nTags: a, b\n---\n\nbody");
        assert_eq!(p.metadata.title(), Some("Fenced"));
        assert_eq!(p.metadata.tags(), ["a", "b"]);
        // The blank line after the closing fence is body content.
        assert_eq!(
            &"---\nTitle: Fenced\nTags: a, b\n---\n\nbody"[p.end_offset..],
            "\nbody"
        );
    }

    #[test]
    fn yaml_fence_mode_ends_at_a_blank_line() {
        let p = parse_metadata("---\nTitle: Fenced\n\nbody");
        assert_eq!(p.metadata.title(), Some("Fenced"));
    }

    #[test]
    fn dot_fence_closes_yaml_mode() {
        // The guides: "The line after the metadata can also be `...`."
        let p = parse_metadata("---\nTitle: Dots\n...\n\nbody");
        assert_eq!(p.metadata.title(), Some("Dots"));
        assert_eq!(&"---\nTitle: Dots\n...\n\nbody"[p.end_offset..], "\nbody");
    }

    #[test]
    fn long_fence_opens_yaml_mode() {
        let p = parse_metadata("----\nTitle: Dashes\n----\n\nbody");
        assert_eq!(p.metadata.title(), Some("Dashes"));
    }

    #[test]
    fn keys_with_dots_hyphens_and_underscores() {
        let p = parse_metadata("Base-Header.Level: 3\nmy_key: v\n\nbody");
        assert_eq!(p.metadata.get("Base-Header.Level"), Some("3"));
        assert_eq!(p.metadata.get("my_key"), Some("v"));
    }

    #[test]
    fn block_without_trailing_blank_runs_to_eof() {
        // BYTE offset, and no phantom newline for an unterminated
        // final line: end lands exactly at the body's end.
        let p = parse_metadata("Title: The End");
        assert_eq!(p.metadata.title(), Some("The End"));
        assert_eq!(p.end_offset, "Title: The End".len());
    }

    #[test]
    fn end_offset_is_a_byte_offset_across_unicode() {
        // Three two-byte chars in the key line: the byte offset must
        // clear the whole block, not a char count of it.
        let body = "Title: caf\u{e9}\u{e9}\u{e9}\n\nbody";
        let p = parse_metadata(body);
        assert_eq!(p.metadata.title(), Some("caf\u{e9}\u{e9}\u{e9}"));
        assert_eq!(&body[p.end_offset..], "body");
    }

    #[test]
    fn crlf_blocks_parse_and_terminate() {
        let body = "Title: Old\r\n\r\nbody\r\n";
        let p = parse_metadata(body);
        assert_eq!(p.metadata.title(), Some("Old"));
        // The blank CRLF line terminates; content starts after it.
        assert_eq!(&body[p.end_offset..], "body\r\n");
    }

    #[test]
    fn url_valued_keys_start_new_pairs_not_continuations() {
        // The CSS standard key carries a URL value: its line must not
        // be swallowed into the previous value (review finding).
        let p = parse_metadata("Title: A\nCSS: https://x.com/s.css\n\nbody");
        assert_eq!(p.metadata.title(), Some("A"));
        assert_eq!(p.metadata.get("css"), Some("https://x.com/s.css"));
    }

    #[test]
    fn prose_first_line_with_a_url_is_body_not_metadata() {
        // The bare-URL rule is anchored: a line that merely CONTAINS
        // a URL is ordinary (formats section 1.1: "a line that IS a
        // URL").
        let p = parse_metadata("Title: Notes https://example.com/x\n\nbody");
        assert_eq!(p.metadata.title(), Some("Notes https://example.com/x"));
    }

    #[test]
    fn bare_url_first_line_starts_no_block() {
        let p = parse_metadata("https://example.com/page\n\nbody");
        assert!(p.metadata.is_empty());
        assert_eq!(p.end_offset, 0);
    }

    #[test]
    fn prose_after_metadata_stays_in_the_body() {
        let p = parse_metadata("Title: T\nAuthor: A\n\nDear reader,\n\nTitle: not meta");
        assert_eq!(p.metadata.title(), Some("T"));
        assert_eq!(p.metadata.len(), 2);
    }
}
