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

use crate::note::Note;

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

/// Order `indexes` (arena indexes into `notes`) as the visible list.
///
/// Upstream runs two stable mergesorts: the title comparator first,
/// then the column's (`sortAndRedisplayNotes`,
/// `NotationController.m:1592-1602`); ties in the second pass keep
/// the title order the first pass left. The reverse preference uses
/// the fully reversed comparators, which flip the tiebreaks too
/// (`compareTitleStringReverse`, `NoteObject.m:229-240`), equivalent
/// to reversing each pass's composite ordering.
pub fn sort_visible(indexes: &mut [usize], notes: &[Note], opts: SortOptions) {
    let note = |i: &usize| &notes[*i];
    if opts.reverse {
        indexes.sort_by(|x, y| compare_title(note(x), note(y)).reverse());
    } else {
        indexes.sort_by(|x, y| compare_title(note(x), note(y)));
    }
    if opts.column != SortColumn::Title {
        if opts.reverse {
            indexes.sort_by(|x, y| column_compare(note(x), note(y), opts.column).reverse());
        } else {
            indexes.sort_by(|x, y| column_compare(note(x), note(y), opts.column));
        }
    }
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

    #[test]
    fn default_sort_options_match_upstreams_registered_defaults() {
        let opts = SortOptions::default();
        assert_eq!(opts.column, SortColumn::DateModified);
        assert!(opts.reverse);
    }
}
