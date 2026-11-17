//! The note record and the two intrinsic text rules of the model:
//! first-line title extraction and body decoding. These are the
//! Commonplace-owned equivalents of upstream's foreign-class categories
//! (the crosswalk's rule: title extraction and body sanitization are
//! model behavior, so they live as inherent methods, not free
//! functions).
//!
//! Index units: every offset, cap, and window here counts `char`s
//! (Unicode scalar values), never bytes and never UTF-16 units. This
//! is the recorded Phase 1 confirmation of the index-units contract
//! (`research/rust-model-notes.md`); upstream counted UTF-16 units, a
//! difference that only widens caps for astral-plane text.

use std::fmt;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::time::SystemTime;

/// The UTF-8 byte-order mark, stripped from file bytes on read.
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// The title cap upstream passes to the title extraction (60 chars;
/// the `maxTitleLen:60` call in `syntheticTitleAndTrimmedBody:`,
/// `NSString_NV.m:323-328`).
pub const TITLE_CAP: usize = 60;

/// A single note, backed by one plain-text file on disk.
///
/// The path is the note's identity (invariant 1: notes are files); the
/// title is derived display text, never authoritative. `mtime` and
/// `size` are reconciliation keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The file backing this note, as given to the loader (folder
    /// joined with the file name).
    pub path: PathBuf,
    /// Derived fallback title: the content's first line under the
    /// row-3 rule, the raw file text the filename source is.
    pub title: NoteTitle,
    /// A `Title:` metadata override (spec row 53): when present it
    /// wins the display title; the first line stays the filename
    /// source.
    pub title_override: Option<String>,
    /// The folded DISPLAY-title key (`crate::fold` over
    /// [`Self::display_title`]): what matching, the alpha sorts, and
    /// the autocomplete-prefix rule all compare. Upstream kept the
    /// same idea as the lowercased `cTitle` C-string cache on the
    /// note itself (map §2).
    pub folded_title: Box<str>,
    /// Full file text, decoded, BOM stripped, line endings as read.
    pub body: NoteBody,
    /// File creation time where the filesystem provides a birth time,
    /// else the modification time (spec row 60).
    pub created: SystemTime,
    /// File modification time at load.
    pub mtime: SystemTime,
    /// File size in bytes at load: the reconcile key beside `mtime`
    /// (spec row 63; upstream's `logicalSize`, map section 4).
    pub size: u64,
}

impl Note {
    /// The title the UI shows: the `Title:` metadata override when
    /// present (row 53), else the derived first-line title.
    pub fn display_title(&self) -> &str {
        match &self.title_override {
            Some(override_title) => override_title,
            None => self.title.as_str(),
        }
    }
}

/// A derived note title: the first line of a note's body under the
/// upstream rule (`research/nv-source-map.md` §4, `NSString_NV.m:265-328`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTitle(String);

impl NoteTitle {
    /// Derive the display title from a note body.
    ///
    /// The rule, verified against `NSString_NV.m:265-328`: skip
    /// leading whitespace and newlines; take text up to the first of
    /// `\n`, `\r`, `\t`, U+2028, U+2029 (other whitespace does not
    /// break the line); if longer than [`TITLE_CAP`] chars, cut at the
    /// last literal space within the final 10 chars of the capped
    /// window, else at the cap; an empty result becomes
    /// "Untitled Note".
    pub fn from_body(body: &str) -> Self {
        // Upstream skips `whitespaceAndNewlineCharacterSet`
        // (NSString_NV.m:281-283), which is exactly the Unicode
        // White_Space set that `char::is_whitespace` implements.
        let rest = body.trim_start();
        let first_line = match rest.find(['\n', '\r', '\t', '\u{2028}', '\u{2029}']) {
            Some(end) => &rest[..end],
            None => rest,
        };
        match Self::cap_first_line(first_line) {
            Some(title) => NoteTitle(title),
            None => Self::untitled(),
        }
    }

    /// The upstream name for a note with no derivable title
    /// (`NSString_NV.m:320`; upstream localized the string, the port
    /// fixes the English value).
    pub fn untitled() -> Self {
        NoteTitle("Untitled Note".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 60-char cap with break-at-last-space-in-the-final-10
    /// (`NSString_NV.m:305-313`). `None` for an empty line, which the
    /// caller maps to "Untitled Note" (m:320).
    fn cap_first_line(line: &str) -> Option<String> {
        let len = line.chars().count();
        if len == 0 {
            return None;
        }
        if len <= TITLE_CAP {
            return Some(line.to_owned());
        }
        // Backwards search for a literal U+0020 in the char window
        // [CAP - 10, CAP): a hit cuts there (dropping the space and
        // everything after), a miss cuts at the cap. Upstream searched
        // UTF-16 units (`rangeOfString:` over `NSMakeRange(50, 10)`);
        // here the window is chars per the index-units contract.
        let starts: Vec<usize> = line.char_indices().map(|(i, _)| i).collect();
        let cut_char = (TITLE_CAP - 10..TITLE_CAP)
            .rev()
            .find(|&k| &line[starts[k]..starts[k + 1]] == " ")
            .unwrap_or(TITLE_CAP);
        Some(line[..starts[cut_char]].to_owned())
    }
}

impl fmt::Display for NoteTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A note body: the file's full text, UTF-8, BOM stripped, never
/// lossily replaced. Line endings are kept exactly as read (upstream
/// does no normalization on read: map §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteBody(String);

impl NoteBody {
    /// Decode file bytes as UTF-8, stripping a leading BOM. Invalid
    /// UTF-8 is an error value, never a silent replacement: the
    /// recorded [CP] encoding divergence (spec §5; upstream's
    /// encoding-guess and xattr bookkeeping has no Linux analog).
    pub fn from_utf8(bytes: &[u8]) -> Result<Self, Utf8Error> {
        let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
        std::str::from_utf8(bytes).map(|s| NoteBody(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The body of a note that could not be read or decoded (the
    /// store's warning paths).
    pub fn empty() -> Self {
        NoteBody(String::new())
    }

    /// Wrap text that is already valid (a `&str` is UTF-8 by
    /// construction): the create/save paths.
    pub fn of(text: &str) -> Self {
        NoteBody(text.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden table for the title rule, from the verified upstream
    // contract (map §4, NSString_NV.m:265-328), not from the code.

    #[test]
    fn first_line_becomes_title() {
        let t = NoteTitle::from_body("Hello world\nsecond line\n");
        assert_eq!(t.as_str(), "Hello world");
    }

    #[test]
    fn body_without_trailing_newline() {
        let t = NoteTitle::from_body("Only a line");
        assert_eq!(t.as_str(), "Only a line");
    }

    #[test]
    fn leading_whitespace_and_blank_lines_are_skipped() {
        // whitespaceAndNewline skip (NSString_NV.m:281-283)
        assert_eq!(NoteTitle::from_body("\n\n  Hello\nrest").as_str(), "Hello");
        assert_eq!(NoteTitle::from_body(" \t\t Hello").as_str(), "Hello");
    }

    #[test]
    fn line_breaks_at_carriage_return() {
        assert_eq!(NoteTitle::from_body("Hello\r\nworld").as_str(), "Hello");
        assert_eq!(NoteTitle::from_body("Hello\rworld").as_str(), "Hello");
    }

    #[test]
    fn tab_breaks_the_title_mid_line() {
        // \t is a title delimiter upstream (NSString_NV.m:275-276)
        assert_eq!(NoteTitle::from_body("Hello\tworld").as_str(), "Hello");
    }

    #[test]
    fn unicode_line_and_paragraph_separators_break_the_title() {
        assert_eq!(NoteTitle::from_body("Hello\u{2028}world").as_str(), "Hello");
        assert_eq!(NoteTitle::from_body("Hello\u{2029}world").as_str(), "Hello");
    }

    #[test]
    fn spaces_do_not_break_the_title() {
        assert_eq!(
            NoteTitle::from_body("two  spaces\nx").as_str(),
            "two  spaces"
        );
    }

    fn line_of(n: usize) -> String {
        "a".repeat(n)
    }

    #[test]
    fn cap_59_is_kept_whole() {
        let t = NoteTitle::from_body(&line_of(59));
        assert_eq!(t.as_str().chars().count(), 59);
    }

    #[test]
    fn cap_exactly_60_is_kept_whole() {
        let line = line_of(60);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), line);
    }

    #[test]
    fn cap_61_breaks_at_space_in_the_final_10() {
        // 55 chars, space, 5 chars: the space sits at char 55, inside
        // the [50, 60) window; the cut drops the space and the tail.
        let line = format!("{} {}", line_of(55), line_of(5));
        assert_eq!(line.chars().count(), 61);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), line_of(55));
    }

    #[test]
    fn cap_break_takes_the_last_space_in_the_window() {
        // spaces at chars 52 and 56; backwards search finds 56, so the
        // title is chars [0, 56): everything before the cut, space at
        // 52 included
        let line = format!("{} {} {}", line_of(52), line_of(3), line_of(4));
        assert_eq!(line.chars().count(), 61);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), format!("{} {}", line_of(52), line_of(3)));
    }

    #[test]
    fn cap_ignores_space_just_outside_the_window() {
        // last space at char 49, one below the [50, 60) window: cut at
        // 60 chars, space at 49 included
        let line = format!("{} {}", line_of(49), line_of(11));
        assert_eq!(line.chars().count(), 61);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), format!("{} {}", line_of(49), line_of(10)));
    }

    #[test]
    fn cap_without_space_in_window_cuts_at_60() {
        // a space at 40 only: the cut at 60 lands past it, space included
        let line = format!("{} {}", line_of(40), line_of(20));
        assert_eq!(line.chars().count(), 61);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), format!("{} {}", line_of(40), line_of(19)));
    }

    #[test]
    fn cap_long_line_no_space_at_all() {
        let line = line_of(80);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), line_of(60));
    }

    #[test]
    fn title_cuts_before_a_tab_past_the_cap() {
        // the delimiter rules still apply first: line is cut at \t,
        // then the (short) remainder is kept whole
        let line = format!("{}\t{}", line_of(61), "tail");
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), line_of(60));
    }

    #[test]
    fn exactly_60_with_trailing_space_keeps_the_space() {
        // no truncation branch runs at exactly the cap, and upstream
        // does not trim the extracted title afterwards
        let line = format!("{} ", line_of(59));
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), line);
    }

    #[test]
    fn empty_body_is_untitled() {
        assert_eq!(NoteTitle::from_body("").as_str(), "Untitled Note");
        assert_eq!(NoteTitle::from_body("\n\n").as_str(), "Untitled Note");
        assert_eq!(NoteTitle::from_body("   \t  ").as_str(), "Untitled Note");
    }

    #[test]
    fn astral_chars_count_as_one_char_each() {
        // 61 emoji: cut at 60 CHARS, not 60 bytes (an emoji is 4
        // bytes) and not 60 UTF-16 units (an emoji is 2 upstream
        // units). The index-units contract in action.
        let line = "\u{1F642}".repeat(61);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), "\u{1F642}".repeat(60));
    }

    #[test]
    fn astral_break_space_in_window_counts_chars() {
        // 50 emoji, space, 10 chars: space at char 50, in window; cut
        // at char 50 (the byte position is 200, which must not be
        // confused with a char index)
        let line = format!("{} {}", "\u{1F642}".repeat(50), line_of(10));
        assert_eq!(line.chars().count(), 61);
        let t = NoteTitle::from_body(&line);
        assert_eq!(t.as_str(), "\u{1F642}".repeat(50));
    }

    #[test]
    fn untitled_constructor_matches_the_rule() {
        assert_eq!(NoteTitle::untitled().as_str(), "Untitled Note");
    }

    // Body decoding goldens: UTF-8 always, BOM stripped, never lossy.

    #[test]
    fn body_strips_leading_bom() {
        let b = NoteBody::from_utf8(b"\xEF\xBB\xBF# Title\nbody").unwrap();
        assert_eq!(b.as_str(), "# Title\nbody");
    }

    #[test]
    fn body_keeps_line_endings_exactly() {
        let b = NoteBody::from_utf8(b"a\r\nb\rc\n").unwrap();
        assert_eq!(b.as_str(), "a\r\nb\rc\n");
    }

    #[test]
    fn body_without_bom_is_unchanged() {
        let b = NoteBody::from_utf8(b"plain text").unwrap();
        assert_eq!(b.as_str(), "plain text");
    }

    #[test]
    fn bom_only_body_is_empty() {
        let b = NoteBody::from_utf8(b"\xEF\xBB\xBF").unwrap();
        assert_eq!(b.as_str(), "");
        assert_eq!(NoteTitle::from_body(b.as_str()), NoteTitle::untitled());
    }

    #[test]
    fn invalid_utf8_is_an_error_never_lossy() {
        // Latin-1 "café" bytes: must not become "caf<replacement>"
        let err = NoteBody::from_utf8(b"caf\xE9 notes\nbody").unwrap_err();
        assert_eq!(err.valid_up_to(), 3);
    }

    #[test]
    fn midfile_invalid_utf8_is_an_error() {
        // "ok" is valid, the 0xFF byte is not
        let err = NoteBody::from_utf8(&[b'o', b'k', 0xFF]).unwrap_err();
        assert_eq!(err.valid_up_to(), 2);
    }

    #[test]
    fn bom_after_other_bytes_is_not_stripped() {
        // a BOM sequence not at the start is ordinary text
        let b = NoteBody::from_utf8("x\u{FEFF}y".as_bytes()).unwrap();
        assert_eq!(b.as_str(), "x\u{FEFF}y");
    }
}
