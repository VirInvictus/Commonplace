//! The search engine: column sorts, the incremental filter state
//! machine, and the selection rule. This is the famous as-you-type
//! feel, ported from `NotationController.m`'s filter and sort
//! machinery (map §2) with no inverted index by design: per-note
//! folded caches plus prefix-resume filtering (spec §5 rows 14-17).
//!
//! Matching semantics live in `crate::query` (the naive oracle); the
//! incremental machinery here must agree with it at every step, which
//! the property tests pin.

use std::cmp::Ordering;

use crate::fold::fold;
use crate::note::Note;
use crate::query::{self, PlainToken, QueryError};

/// The sortable columns, upstream's four (`NotesTableView.m:101-105`:
/// title, labels, dateModified, dateCreated). The labels column
/// arrives with Phase 5's label data and is deliberately absent until
/// then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Title,
    DateModified,
    DateCreated,
}

/// Sort configuration. The defaults mirror upstream's registered
/// preferences: the date-modified column, reversed
/// (`GlobalPrefs.m:193-194`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortOptions {
    pub column: SortColumn,
    pub reverse: bool,
}

impl Default for SortOptions {
    fn default() -> Self {
        SortOptions {
            column: SortColumn::DateModified,
            reverse: true,
        }
    }
}

/// The title comparator: folded key byte order (locale-free; row 60
/// notes the divergence from upstream's case-insensitive
/// CFStringCompare), ties by creation date, then by path where
/// upstream compared UUID bytes (`compareTitleString`,
/// `NoteObject.m:199-212`; a files-first engine has no per-note
/// UUIDs).
fn compare_title(a: &Note, b: &Note) -> Ordering {
    a.folded_title
        .cmp(&b.folded_title)
        .then(a.created.cmp(&b.created))
        .then(a.path.cmp(&b.path))
}

/// The date comparators are bare field compares with no tiebreaks
/// (`compareDateModified`/`compareDateCreated`,
/// `NoteObject.m:191-197`): ties keep whatever order the title pass
/// left, which is the whole point of the two stable passes.
fn compare_date_modified(a: &Note, b: &Note) -> Ordering {
    a.mtime.cmp(&b.mtime)
}

fn compare_date_created(a: &Note, b: &Note) -> Ordering {
    a.created.cmp(&b.created)
}

fn column_compare(a: &Note, b: &Note, column: SortColumn) -> Ordering {
    match column {
        SortColumn::Title => compare_title(a, b),
        SortColumn::DateModified => compare_date_modified(a, b),
        SortColumn::DateCreated => compare_date_created(a, b),
    }
}

/// The two stable passes of `sortAndRedisplayNotes`
/// (`NotationController.m:1592-1602`): the title comparator first,
/// then the column's; ties in the second pass keep the title order
/// the first pass left. The reverse preference uses the fully
/// reversed comparators, which flip the tiebreaks too
/// (`compareTitleStringReverse`, `NoteObject.m:229-240`), equivalent
/// to reversing each pass's composite ordering.
fn two_pass_stable<T>(
    items: &mut [T],
    title: impl Fn(&T, &T) -> Ordering,
    column: impl Fn(&T, &T) -> Ordering,
    use_column: bool,
    reverse: bool,
) {
    if reverse {
        items.sort_by(|x, y| title(x, y).reverse());
    } else {
        items.sort_by(title);
    }
    if use_column {
        if reverse {
            items.sort_by(|x, y| column(x, y).reverse());
        } else {
            items.sort_by(column);
        }
    }
}

/// Order `indexes` (arena indexes into `notes`) as the visible list.
pub fn sort_visible(indexes: &mut [usize], notes: &[Note], opts: SortOptions) {
    let note = |i: &usize| &notes[*i];
    two_pass_stable(
        indexes,
        |x, y| compare_title(note(x), note(y)),
        |x, y| column_compare(note(x), note(y), opts.column),
        opts.column != SortColumn::Title,
        opts.reverse,
    );
}

/// One visible candidate: the arena index plus the per-field resume
/// positions, the port of upstream's `cTitleFoundPtr` /
/// `cContentsFoundPtr` pair (`NoteObject.m:1836-1840`). `None` means
/// the field failed the current token and stays dead until the next
/// fresh scan resets it, exactly like a NULL found pointer.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    index: usize,
    title_pos: Option<usize>,
    body_pos: Option<usize>,
}

/// The incremental filter state machine: the port of
/// `filterNotesFromUTF8String:` (`NotationController.m:1358-1499`),
/// updated per keystroke.
///
/// The resume trick, and when it applies (spec row 60's closing
/// note): for plain AND-chains of words, phrases, and tag words (the
/// grammar upstream's resume was built for), a query that only
/// extends the previous one keeps its candidate set and re-scans only
/// the tokens from the previous last token onward; the first of those
/// resumes each field's search from the previous token's match
/// position (sound, because `strstr` found the first occurrence, so
/// the longer needle can only match at or after it). Operator and
/// group queries re-filter naively from all notes: upstream has no
/// resume semantics for them to preserve. Any query deletion, tag
/// -word count increase, or grammar change restarts from all notes.
///
/// Lifetime contract: the state is keyed by arena indexes of ONE
/// loaded collection. Every `update`/`resort`/`selection` call must
/// hand over the same notes slice (same store, unchanged contents):
/// the engine's notes are immutable between loads in these phases,
/// and Phase 4's reconcile pass will clear the state on any disk
/// change rather than re-key it.
#[derive(Debug)]
pub struct FilterState {
    /// The folded working query (upstream's lowercased
    /// `currentFilterStr`).
    query: String,
    /// The visible candidates, kept in sorted-view order.
    candidates: Vec<Candidate>,
    /// Char offset in `query` where the last scanned token starts
    /// (upstream's `lastWordInFilterStr`).
    last_word_start: usize,
    /// Whether the current query is a plain AND-chain (resume-
    /// eligible).
    plain: bool,
    /// Tag-word count of the current raw query
    /// (`NVQueryNewlyHasTagWord` compares these).
    tag_words: usize,
    /// Folded body cache keyed by arena index: bodies are the large
    /// part, folded on first touch and kept for the state's lifetime.
    /// Derived and rebuildable by contract; Phase 4's reconcile pass
    /// clears it when a note changes on disk.
    body_folds: std::collections::HashMap<usize, Box<str>>,
    sort: SortOptions,
}

impl FilterState {
    pub fn new(sort: SortOptions) -> Self {
        FilterState {
            query: String::new(),
            candidates: Vec::new(),
            last_word_start: 0,
            plain: true,
            tag_words: 0,
            body_folds: std::collections::HashMap::new(),
            sort,
        }
    }

    /// The visible candidate set as arena indexes, in sorted order.
    pub fn visible(&self) -> Vec<usize> {
        self.candidates.iter().map(|c| c.index).collect()
    }

    /// Apply a keystroke's query. Parse errors leave the state
    /// untouched and return the error value; the caller (the field,
    /// Phase 11) decides to keep showing the previous results.
    pub fn update(&mut self, raw_query: &str, notes: &[Note]) -> Result<Vec<usize>, QueryError> {
        let expr = query::parse(raw_query)?;
        let new_query = fold(raw_query);
        // Resume eligibility is a property of the token stream, not
        // just the AST: an operator hidden in the raw query (the `&`
        // in "O& ut") parses to a plain chain but must not take the
        // token-resume path.
        let new_plain = query::resume_eligible(raw_query);
        let new_tag_words = query::tag_word_count(raw_query);

        // PHASE 1: resume or restart (m:1374-1388).
        let resume_ok = self.plain
            && new_plain
            && !self.query.is_empty()
            && new_query.starts_with(&self.query)
            && new_tag_words <= self.tag_words;
        if !resume_ok {
            self.candidates = (0..notes.len())
                .map(|index| Candidate {
                    index,
                    title_pos: None,
                    body_pos: None,
                })
                .collect();
            self.last_word_start = 0;
        }

        // PHASE 2: filter the candidates (m:1396-1454).
        match expr.as_ref() {
            None => {
                // The empty query filters nothing; the candidate set
                // is all notes either way.
            }
            Some(_) if new_plain => {
                // Plain chains are scanned token by token; the parsed
                // shape only decided that this is the resume path.
                self.scan_plain_tokens(raw_query, resume_ok, notes);
            }
            Some(expr) => {
                let old = std::mem::take(&mut self.candidates);
                let mut kept = Vec::with_capacity(old.len());
                for cand in old {
                    let body = self.folded_body(cand.index, notes);
                    if query::matches(expr, &notes[cand.index].folded_title, body) {
                        kept.push(cand);
                    }
                }
                self.candidates = kept;
            }
        }

        // PHASE 3: the sorted view.
        sort_candidates(&mut self.candidates, notes, self.sort);

        self.query = new_query;
        self.plain = new_plain;
        self.tag_words = new_tag_words;
        Ok(self.visible())
    }

    /// Re-sort the current candidate set for a column change, without
    /// refiltering (upstream's `sortAndRedisplayNotes`).
    pub fn resort(&mut self, notes: &[Note], sort: SortOptions) {
        self.sort = sort;
        sort_candidates(&mut self.candidates, notes, sort);
    }

    /// The autocomplete selection: the first visible note whose
    /// folded title has the whole query as a prefix, unless a
    /// "prefix parent" wins: a note whose title is itself a prefix of
    /// that title, also matches the query, and is visible. Parents
    /// are considered shortest-first (alpha order puts prefixes
    /// first), mirroring `prefixParentsOfNote` and the PHASE 4 block
    /// (`NotationController.m:1460-1491`). No selection for an empty
    /// query or a query with tag words, like upstream; the
    /// autocomplete preference itself is Phase 11's to consult.
    pub fn selection(&self, notes: &[Note]) -> Option<usize> {
        if self.query.is_empty() || self.tag_words > 0 {
            return None;
        }
        for (vis_pos, cand) in self.candidates.iter().enumerate() {
            let title = &notes[cand.index].folded_title;
            if !title.starts_with(&self.query) {
                continue;
            }
            // Prefix parents of this title, shortest-first via the
            // folded-key alpha order (any title that prefixes another
            // sorts before it).
            let mut parents: Vec<usize> = notes
                .iter()
                .enumerate()
                .filter(|(j, other)| {
                    *j != cand.index
                        && title.starts_with(&other.folded_title[..])
                        && !other.folded_title.is_empty()
                })
                .map(|(j, _)| j)
                .collect();
            parents.sort_by(|x, y| notes[*x].folded_title.cmp(&notes[*y].folded_title));
            for parent in parents {
                if !notes[parent].folded_title.starts_with(&self.query) {
                    continue;
                }
                if let Some(pos) = self.candidates.iter().position(|c| c.index == parent) {
                    return Some(pos);
                }
            }
            return Some(vis_pos);
        }
        None
    }

    /// The folded body of one note, folded on first touch and cached.
    fn folded_body(&mut self, index: usize, notes: &[Note]) -> &str {
        self.body_folds
            .entry(index)
            .or_insert_with(|| fold(notes[index].body.as_str()).into_boxed_str())
    }

    /// Scan the tokens of `raw_query` from `last_word_start` over the
    /// candidate set: the port of the strsep resume loop (m:1405-1454),
    /// tokenized by the same lexer the parser uses
    /// ([`query::plain_tokens`]) so the needle sets can never
    /// disagree.
    fn scan_plain_tokens(&mut self, raw_query: &str, resume_ok: bool, notes: &[Note]) {
        let resume_from = self.last_word_start;
        let tokens = query::plain_tokens(raw_query);
        // Tokens before the resume offset already matched in a
        // previous run; scanning starts at the previous last token
        // (upstream resumes strsep at lastWordInFilterStr, m:1407).
        for (start, token) in tokens.into_iter().skip_while(|(s, _)| *s < resume_from) {
            // Only the first scanned token (the previous last token,
            // still at its old offset in a prefix extension) resumes
            // from the stored positions; every later token scans
            // fresh, resetting them (m:1427: useCachedPositions).
            let resumed = resume_ok && start == self.last_word_start;
            // The needle is the token text, folded exactly as the
            // parser folds it; a tag word's needle is the whole
            // "#xyz" literal (upstream trims the word,
            // TagSearch.m:36-40).
            let needle: String = match &token {
                PlainToken::Word(text) | PlainToken::Phrase(text) => fold(text),
                PlainToken::TagWord(name) => fold(&format!("#{name}")),
            };
            let is_tag = matches!(token, PlainToken::TagWord(_));
            let needle = needle.as_str();

            let old = std::mem::take(&mut self.candidates);
            let mut kept: Vec<Candidate> = Vec::with_capacity(old.len());
            for mut cand in old {
                if is_tag {
                    // Tag words scan the whole buffers fresh and leave
                    // the resume pointers untouched
                    // (noteMatchesTagWord, NoteObject.m:1842-1848).
                    let title = &notes[cand.index].folded_title;
                    let body = self.folded_body(cand.index, notes);
                    if title.contains(needle) || body.contains(needle) {
                        kept.push(cand);
                    }
                    continue;
                }
                let title = notes[cand.index].folded_title.as_ref();
                let body = self.folded_body(cand.index, notes);
                let (title_pos, body_pos) = if resumed {
                    (
                        cand.title_pos.and_then(|off| find_from(title, needle, off)),
                        cand.body_pos.and_then(|off| find_from(body, needle, off)),
                    )
                } else {
                    // Fresh scan: reset the pointers to the field
                    // starts first (resetFoundPtrsForNote).
                    (find_from(title, needle, 0), find_from(body, needle, 0))
                };
                cand.title_pos = title_pos;
                cand.body_pos = body_pos;
                if title_pos.is_some() || body_pos.is_some() {
                    kept.push(cand);
                }
            }
            self.candidates = kept;
            // Every non-empty scanned token moves the resume offset
            // (m:1452).
            self.last_word_start = start;
        }
    }
}

/// Substring search from a char offset, returning the absolute match
/// start (upstream stores the found position in the field pointer).
fn find_from(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let byte_from = haystack
        .char_indices()
        .nth(from)
        .map(|(b, _)| b)
        .unwrap_or(haystack.len());
    haystack[byte_from..]
        .find(needle)
        .map(|byte_pos| haystack[..byte_from + byte_pos].chars().count())
}

/// Sort the candidate list into the visible order: the same two
/// stable passes over the candidates' notes.
fn sort_candidates(candidates: &mut [Candidate], notes: &[Note], opts: SortOptions) {
    let cand = |c: &Candidate| &notes[c.index];
    two_pass_stable(
        candidates,
        |x, y| compare_title(cand(x), cand(y)),
        |x, y| column_compare(cand(x), cand(y), opts.column),
        opts.column != SortColumn::Title,
        opts.reverse,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{NoteBody, NoteTitle};
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn note(path: &str, title: &str, created: u64, mtime: u64) -> Note {
        let title = NoteTitle::from_body(title);
        Note {
            path: PathBuf::from(path),
            folded_title: crate::fold::fold(title.as_str()).into_boxed_str(),
            title,
            body: NoteBody::empty(),
            created: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(created),
            mtime: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime),
            size: 0,
        }
    }

    fn titles(indexes: &[usize], notes: &[Note]) -> Vec<String> {
        indexes
            .iter()
            .map(|i| notes[*i].title.as_str().to_owned())
            .collect()
    }

    #[test]
    fn title_pass_orders_by_the_folded_key() {
        // Folded key byte order: accented titles sort beside their
        // unaccented selves, not scattered by codepoint.
        let notes = vec![
            note("z.md", "zebra", 1, 1),
            note("e.md", "Éclair", 1, 1),
            note("a.md", "apple", 1, 1),
        ];
        let mut idx = vec![0, 1, 2];
        sort_visible(
            &mut idx,
            &notes,
            SortOptions {
                column: SortColumn::Title,
                reverse: false,
            },
        );
        assert_eq!(titles(&idx, &notes), ["apple", "Éclair", "zebra"]);
    }

    #[test]
    fn title_ties_break_by_created_then_path() {
        let notes = vec![
            note("b.md", "same", 20, 1),
            note("a.md", "same", 10, 1),
            note("c.md", "same", 10, 1),
        ];
        let mut idx = vec![0, 1, 2];
        sort_visible(
            &mut idx,
            &notes,
            SortOptions {
                column: SortColumn::Title,
                reverse: false,
            },
        );
        assert_eq!(titles(&idx, &notes), ["same", "same", "same"]);
        assert_eq!(idx, vec![1, 2, 0]); // created 10, 10, 20; path a < c
    }

    #[test]
    fn date_column_ties_keep_title_order() {
        // Equal mtimes: pass 2 is stable over pass 1, so ties come out
        // in title order. That IS the two-pass contract (map §2).
        let notes = vec![
            note("1.md", "mango", 1, 500),
            note("2.md", "apple", 1, 500),
            note("3.md", "zebra", 1, 100),
        ];
        let mut idx = vec![0, 1, 2];
        sort_visible(
            &mut idx,
            &notes,
            SortOptions {
                column: SortColumn::DateModified,
                reverse: false,
            },
        );
        assert_eq!(titles(&idx, &notes), ["zebra", "apple", "mango"]);
    }

    #[test]
    fn reverse_flips_the_column_and_the_tiebreak_order() {
        let notes = vec![
            note("1.md", "mango", 1, 500),
            note("2.md", "apple", 1, 500),
            note("3.md", "zebra", 1, 100),
        ];
        let mut idx = vec![0, 1, 2];
        sort_visible(
            &mut idx,
            &notes,
            SortOptions {
                column: SortColumn::DateModified,
                reverse: true,
            },
        );
        // Newest first (mango and apple at 500, zebra at 100); the
        // 500-tie keeps pass 1's REVERSED title order (zebra, mango,
        // apple), which among the ties reads mango, apple.
        assert_eq!(titles(&idx, &notes), ["mango", "apple", "zebra"]);
    }

    #[test]
    fn title_column_runs_one_pass_only() {
        // Title is pass 1; reverse flips title, created, and path
        // together (upstream's compareTitleStringReverse).
        let notes = vec![note("a.md", "same", 10, 1), note("b.md", "same", 20, 1)];
        let mut idx = vec![0, 1];
        sort_visible(
            &mut idx,
            &notes,
            SortOptions {
                column: SortColumn::Title,
                reverse: true,
            },
        );
        assert_eq!(idx, vec![1, 0]); // title equal, created 20 before 10
    }

    #[test]
    fn created_column_sorts() {
        let notes = vec![
            note("1.md", "new", 900, 1),
            note("2.md", "old", 100, 1),
            note("3.md", "mid", 500, 1),
        ];
        let mut idx = vec![0, 1, 2];
        sort_visible(
            &mut idx,
            &notes,
            SortOptions {
                column: SortColumn::DateCreated,
                reverse: false,
            },
        );
        assert_eq!(titles(&idx, &notes), ["old", "mid", "new"]);
    }

    // --- The incremental filter and its oracle ---

    fn filter_note(path: &str, title: &str, body: &str, mtime: u64) -> Note {
        let title = NoteTitle::from_body(title);
        Note {
            path: PathBuf::from(path),
            folded_title: crate::fold::fold(title.as_str()).into_boxed_str(),
            title,
            body: NoteBody::from_utf8(body.as_bytes()).unwrap(),
            created: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100),
            mtime: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime),
            size: 0,
        }
    }

    /// A corpus with prefix-parent traps, accents, hashtags, and
    /// phrase targets: the shapes the resume logic can get wrong.
    fn fixture() -> Vec<Note> {
        vec![
            filter_note("w.md", "worf and klingons", "peace in the empire", 500),
            filter_note("w2.md", "worferrari", "fast machines", 400),
            filter_note("w3.md", "worf", "the sole title", 300),
            filter_note("p.md", "Picard's log", "captain picard drinks tea", 200),
            filter_note("b.md", "Bücher list", "deutsche Bücher sortiert", 150),
            filter_note("c.md", "cafe notes", "café au lait and croissants", 120),
            filter_note("s.md", "#startrek logs", "#startrek #tng worf appears", 110),
            filter_note(
                "e.md",
                "exact phrase demo",
                "contains an exact phrase here",
                105,
            ),
            filter_note("z.md", "zebra", "plain body text", 100),
            // Discriminators for the review-found divergence classes:
            // each matches under one tokenization and not the other,
            // so the property tests go red if the scanner's needle set
            // ever drifts from the parser's again.
            filter_note("q.md", "quote target", "x a b spaces", 95),
            // NBSP is needle content, not a separator (row 59)
            filter_note("n.md", "nb sp", "foo\u{a0}bar lives", 92),
            // tag:x must reach the literal #unix, not needles tag+unix
            filter_note("u.md", "unix tips", "#unix basics", 91),
            // İ folds through its table entry, dotless
            filter_note("i.md", "İstanbul notes", "turkish city", 90),
        ]
    }

    /// The naive reference: parse, match every note, sort. `Err` for
    /// queries that fail to parse, like the incremental path.
    fn naive(raw: &str, notes: &[Note], sort: SortOptions) -> Result<Vec<usize>, ()> {
        match crate::query::parse(raw) {
            Err(_) => Err(()),
            Ok(None) => {
                let mut all: Vec<usize> = (0..notes.len()).collect();
                sort_visible(&mut all, notes, sort);
                Ok(all)
            }
            Ok(Some(expr)) => {
                let mut idx = crate::query::find_matches(&expr, notes);
                sort_visible(&mut idx, notes, sort);
                Ok(idx)
            }
        }
    }

    /// The selection rule written against an arbitrary visible list,
    /// independent of FilterState's candidate bookkeeping.
    fn naive_selection(visible: &[usize], raw: &str, notes: &[Note]) -> Option<usize> {
        let folded_query = crate::fold::fold(raw);
        if folded_query.is_empty() || crate::query::tag_word_count(raw) > 0 {
            return None;
        }
        for (pos, &i) in visible.iter().enumerate() {
            let title = &notes[i].folded_title;
            if !title.starts_with(&folded_query) {
                continue;
            }
            let mut parents: Vec<usize> = notes
                .iter()
                .enumerate()
                .filter(|(j, other)| {
                    *j != i
                        && title.starts_with(&other.folded_title[..])
                        && !other.folded_title.is_empty()
                })
                .map(|(j, _)| j)
                .collect();
            parents.sort_by(|x, y| notes[*x].folded_title.cmp(&notes[*y].folded_title));
            for parent in parents {
                if notes[parent].folded_title.starts_with(&folded_query)
                    && let Some(pp) = visible.iter().position(|&v| v == parent)
                {
                    return Some(pp);
                }
            }
            return Some(pos);
        }
        None
    }

    fn step_matches_naive(state: &mut FilterState, raw: &str, notes: &[Note], sort: SortOptions) {
        let incremental = state.update(raw, notes);
        let oracle = naive(raw, notes, sort);
        match oracle {
            Err(_) => assert!(incremental.is_err(), "parse error not surfaced for {raw:?}"),
            Ok(expected) => {
                let got = incremental.unwrap();
                assert_eq!(got, expected, "visible mismatch for {raw:?}");
                assert_eq!(
                    state.selection(notes),
                    naive_selection(&expected, raw, notes),
                    "selection mismatch for {raw:?}"
                );
            }
        }
    }

    #[test]
    fn typing_forward_matches_the_oracle_at_every_char() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        for end in 1..="worf picard".len() {
            let raw = &"worf picard"[..end];
            step_matches_naive(&mut state, raw, &notes, sort);
        }
    }

    #[test]
    fn deleting_back_matches_the_oracle_at_every_char() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        let full = "worferrari picard";
        for end in (1..=full.len()).rev() {
            let raw = &full[..end];
            step_matches_naive(&mut state, raw, &notes, sort);
        }
    }

    #[test]
    fn mid_word_inserts_and_quote_flips_match_the_oracle() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        for raw in [
            "worf",
            "worf an",
            "worf am", // mid-word edit: not a prefix extension
            "worf am \"exact",
            "worf am \"exact phrase\"",
            "worf OR picard",
            "worf OR picard NOT caf",
            "NOT worf",
            "(worf OR zebra) AND picard",
            "#startrek",
            "worf #startrek",
            "worf #startrek logs",
            "tag:startrek AND android",
            "",
            " ",
            "Bücher",
            "bucher",
            // Review-found divergence classes, each with a fixture
            // note that one tokenization admits and the other refuses:
            "tag:unix",     // full-name operator: naive path, literal #unix
            "\"x\" a",      // quote segments; unquoted tail splits per word
            "\"x\" a b",    // the resumed tail must stay two words, not one
            "foo\u{a0}bar", // NBSP is needle content, not a separator
            "istanbul",     // İ folds dotless through its table entry
        ] {
            step_matches_naive(&mut state, raw, &notes, sort);
        }
    }

    #[test]
    fn random_edit_walk_never_drifts_from_the_oracle() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        let alphabet: Vec<char> = "worf picard #st tag:u \"ex AND OR NOT !()&|:éBüİx"
            .chars()
            .collect();
        let mut raw = String::new();
        let mut seed: u64 = 0x5EED_C0DE;
        for step in 0..600 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let r = seed;
            match r % 5 {
                0 => raw.push(alphabet[(r >> 8) as usize % alphabet.len()]),
                1 => {
                    raw.pop();
                }
                2 => {
                    if !raw.is_empty() {
                        let mut cut = (r >> 8) as usize % raw.len();
                        while !raw.is_char_boundary(cut) {
                            cut -= 1;
                        }
                        raw.truncate(cut);
                    }
                }
                3 => {
                    // insert at a char boundary
                    let at = (r >> 16) as usize % (raw.chars().count() + 1);
                    let byte_at = raw
                        .char_indices()
                        .nth(at)
                        .map(|(b, _)| b)
                        .unwrap_or(raw.len());
                    raw.insert(byte_at, alphabet[(r >> 24) as usize % alphabet.len()]);
                }
                _ => raw.clear(),
            }
            let incremental = state.update(&raw, &notes);
            let oracle = naive(&raw, &notes, sort);
            match oracle {
                Err(_) => assert!(
                    incremental.is_err(),
                    "step {step}: parse error missing for {raw:?}"
                ),
                Ok(expected) => {
                    assert_eq!(
                        incremental.unwrap(),
                        expected,
                        "step {step}: drift at {raw:?}"
                    );
                    assert_eq!(
                        state.selection(&notes),
                        naive_selection(&expected, &raw, &notes),
                        "step {step}: selection drift at {raw:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn parse_error_leaves_the_previous_results_showing() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        state.update("worf", &notes).unwrap();
        let before = state.visible();
        assert!(state.update("worf AND", &notes).is_err());
        assert_eq!(state.visible(), before);
        // And the state still works afterwards.
        state.update("worf picard", &notes).unwrap();
        assert_eq!(state.visible(), naive("worf picard", &notes, sort).unwrap());
    }

    #[test]
    fn the_prefix_parent_wins_selection() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        // mtimes: "worf and klingons" (500) newest, "worferrari" (400),
        // "worf" (300): the visible order for "worf" is
        // [worf and klingons, worferrari, worf]. The first title-prefix
        // match is "worf and klingons", but "worf" is a prefix parent
        // of it and is visible, so the parent wins (PHASE 4,
        // NotationController.m:1460-1491).
        let visible = state.update("worf", &notes).unwrap();
        assert_eq!(
            visible,
            naive("worf", &notes, sort).unwrap(),
            "oracle sanity"
        );
        assert_eq!(state.selection(&notes), Some(2));
    }

    #[test]
    fn selection_falls_back_to_the_first_prefix_match() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        // "worf an" only title-prefixes "worf and klingons"; its
        // parent "worf" is not visible ("an" does not match the sole
        // title), so the match itself is selected.
        let visible = state.update("worf an", &notes).unwrap();
        assert_eq!(state.selection(&notes).map(|p| visible[p]), Some(0));
    }

    #[test]
    fn no_selection_for_empty_or_tag_queries() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        state.update("", &notes).unwrap();
        assert_eq!(state.selection(&notes), None);
        state.update("worf #st", &notes).unwrap();
        assert_eq!(state.selection(&notes), None);
    }

    #[test]
    fn resort_changes_order_without_refiltering() {
        let notes = fixture();
        let sort = SortOptions::default();
        let mut state = FilterState::new(sort);
        state.update("worf", &notes).unwrap();
        let before = state.visible();
        state.resort(
            &notes,
            SortOptions {
                column: SortColumn::Title,
                reverse: false,
            },
        );
        let after = state.visible();
        assert_eq!(before.len(), after.len());
        // Same set, title order now.
        let mut sorted_view = after.clone();
        sorted_view.sort();
        let mut before_sorted = before.clone();
        before_sorted.sort();
        assert_eq!(sorted_view, before_sorted);
        // And the order is the naive title order.
        let mut expected: Vec<usize> =
            crate::query::find_matches(&crate::query::parse("worf").unwrap().unwrap(), &notes);
        sort_visible(
            &mut expected,
            &notes,
            SortOptions {
                column: SortColumn::Title,
                reverse: false,
            },
        );
        assert_eq!(after, expected);
    }

    #[test]
    fn default_sort_options_match_upstreams_registered_defaults() {
        let opts = SortOptions::default();
        assert_eq!(opts.column, SortColumn::DateModified);
        assert!(opts.reverse);
    }
}
