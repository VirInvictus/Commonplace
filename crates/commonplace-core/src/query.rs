//! The query language: one parser covering the three documented
//! grammars of the Notational Velocity line, superset-ordered from
//! weakest to strongest (spec §5 rows 8-11 and the Phase 2 decision
//! rows 57-59).
//!
//! - [NV]: bare words, implicit AND, `:` as a token separator, and the
//!   quote character that switches the whole query to phrase mode
//!   (`NotationController.m:1396`, map §2).
//! - [ALT]: `#tag` words that mix with plain words (`TagSearch.m`,
//!   delta §1; spec row 57).
//! - [ULTRA]: boolean keywords `AND`/`OR`/`NOT` with `&&`/`&`,
//!   `||`/`|`, `!` aliases, parentheses, the full-name `tag:`
//!   operator, and quoted exact ordered phrases (ultra search-syntax).
//!
//! Precedence is parens > NOT > AND > OR with implicit AND at parse
//! level. Needles are folded (case + diacritics, `crate::fold`) at
//! parse time; the matcher compares folded needles against folded
//! haystacks. Matching runs over title and body; the labels field and
//! the tag-store half of tag-word matching join at Phase 5 (the
//! literal `#xyz` rule carries tag words until then).

use std::fmt;

use crate::fold::fold;
use crate::note::Note;

/// The parsed query. `Term` and `Phrase` differ only in how the lexer
/// grouped them (phrases keep their spaces as one exact ordered
/// needle); both match as folded substrings of title or body, exactly
/// like upstream's per-token strstr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A bare word: a folded substring needle.
    Term(String),
    /// A quoted run: one folded needle matched in order.
    Phrase(String),
    /// A tag word: `#xyz` (`prefix: true`, nvALT prefix matching per
    /// row 57) or `tag:xyz` (`prefix: false`, the full-name operator).
    /// Matches the literal `#xyz` in title/body until the Phase 5 tag
    /// store adds the tag-title side.
    TagWord { name: String, prefix: bool },
    /// Every operand must match (implicit AND per token, map §2).
    And(Vec<Expr>),
    /// Any operand may match.
    Or(Vec<Expr>),
    /// The operand must not match.
    Not(Box<Expr>),
}

/// A grammar failure. Error values, never app errors: the search
/// field stays a search field (invariant 2; spec row 59, point 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// A group was opened but never closed (`"(foo"`).
    UnclosedParen,
    /// A close paren with no open group (`"foo)"`).
    UnbalancedCloseParen,
    /// `"()"`.
    EmptyGroup,
    /// An operator with nothing to operate on (`"a AND"`, `"OR foo"`,
    /// a bare `"NOT"`).
    MissingOperand,
    /// `"tag:"` with no name after it.
    EmptyTagName,
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::UnclosedParen => write!(f, "query has an unclosed '('"),
            QueryError::UnbalancedCloseParen => write!(f, "query has a ')' with no matching '('"),
            QueryError::EmptyGroup => write!(f, "query has an empty group '()'"),
            QueryError::MissingOperand => write!(f, "query has an operator with no operand"),
            QueryError::EmptyTagName => write!(f, "query has 'tag:' with no tag name"),
        }
    }
}

impl std::error::Error for QueryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    Phrase(String),
    TagWord(String),
    TagOp(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
}

fn is_word_break(c: char) -> bool {
    // The separator set is exactly upstream's ASCII set plus the
    // structural characters (row 59): other Unicode whitespace is
    // needle content, matching NV's byte-wise strsep, and keeping the
    // lexer identical to the incremental scan's tokenizer.
    matches!(
        c,
        ' ' | '\t' | '\r' | '\n' | '(' | ')' | '"' | '&' | '|' | ':'
    )
}

/// Trim the separator whitespace exactly, never Unicode whitespace:
/// an NBSP inside a needle is content (row 59's pinned set).
fn trim_separators(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
}

/// Lex the raw query into tokens with their char offsets. The
/// superset resolutions of spec row 59 live here: operators are the
/// uppercase keywords and symbols; `:` ends a word and is skipped;
/// `#` starts a tag word; a `"` anywhere switches to phrase mode
/// (upstream re-tokenizes the whole query on `"` when it contains
/// one, `NotationController.m:1396`), with an unterminated quote
/// extending to the end. Whitespace is exactly the ASCII separator
/// set, matching the scan side (row 59, point 3).
fn lex_with_offsets(raw: &str) -> Vec<(usize, Tok)> {
    let chars: Vec<char> = raw.chars().collect();
    let mut toks: Vec<(usize, Tok)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            _ if matches!(c, ' ' | '\t' | '\r' | '\n') => i += 1,
            '(' => {
                toks.push((i, Tok::LParen));
                i += 1;
            }
            ')' => {
                toks.push((i, Tok::RParen));
                i += 1;
            }
            '&' => {
                toks.push((i, Tok::And));
                i += if i + 1 < chars.len() && chars[i + 1] == '&' {
                    2
                } else {
                    1
                };
            }
            '|' => {
                toks.push((i, Tok::Or));
                i += if i + 1 < chars.len() && chars[i + 1] == '|' {
                    2
                } else {
                    1
                };
            }
            ':' => i += 1,
            '"' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                let phrase: String = chars[start..i].iter().collect();
                let trimmed = trim_separators(&phrase);
                if !trimmed.is_empty() {
                    toks.push((start, Tok::Phrase(trimmed.to_owned())));
                }
                i += 1; // past the closing quote (or the end)
            }
            '!' => {
                toks.push((i, Tok::Not));
                i += 1;
            }
            '#' => {
                let start = i;
                while i < chars.len() && !is_word_break(chars[i]) {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if word.chars().count() > 1 {
                    toks.push((start, Tok::TagWord(word[1..].to_owned())));
                } else {
                    // A bare "#" is an ordinary term (upstream requires
                    // token length >= 2 for a tag word, TagSearch.m:27-40).
                    toks.push((start, Tok::Word(word)));
                }
            }
            _ => {
                // The full-name tag operator: "tag:" at a word start,
                // case-insensitively (the manual shows tag:X and TAG:X).
                // Take chars, not bytes: a multibyte char at the
                // boundary must not panic the slice.
                let next4: String = chars[i..].iter().take(4).collect();
                if next4.len() == 4 && next4.eq_ignore_ascii_case("tag:") {
                    i += 4;
                    let name_start = i;
                    while i < chars.len() && !is_word_break(chars[i]) {
                        i += 1;
                    }
                    let name: String = chars[name_start..i].iter().collect();
                    toks.push((i, Tok::TagOp(name)));
                } else {
                    let start = i;
                    while i < chars.len() && !is_word_break(chars[i]) {
                        i += 1;
                    }
                    let word: String = chars[start..i].iter().collect();
                    // The uppercase keyword forms are operators; any
                    // other case is an ordinary term (row 59: NV
                    // lowercased queries, so lowercase keywords never
                    // meant operators upstream).
                    let tok = match word.as_str() {
                        "AND" => Tok::And,
                        "OR" => Tok::Or,
                        "NOT" => Tok::Not,
                        _ => Tok::Word(word),
                    };
                    toks.push((start, tok));
                }
            }
        }
    }
    toks
}

fn lex(raw: &str) -> Vec<Tok> {
    lex_with_offsets(raw)
        .into_iter()
        .map(|(_, tok)| tok)
        .collect()
}

/// One scanner-visible token of a resume-eligible query: exactly the
/// segmentation the parser sees, so the incremental filter's needles
/// and the oracle's can never disagree. Only these three kinds can
/// appear in a resume-eligible stream (operators, groups, and
/// `tag:` operators make `resume_eligible` false).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlainToken {
    /// A bare word; the scan needle is the folded text.
    Word(String),
    /// A quoted run; the scan needle is the folded text.
    Phrase(String),
    /// A `#xyz` word; the scan needle is the folded `#xyz` literal.
    TagWord(String),
}

/// Tokenize a query for the incremental scan. Call only for
/// resume-eligible queries ([`resume_eligible`]); for anything else
/// the parser's AST governs via the naive re-filter path. Offsets are
/// char offsets into `raw`, pointing at the needle's first character.
pub fn plain_tokens(raw: &str) -> Vec<(usize, PlainToken)> {
    lex_with_offsets(raw)
        .into_iter()
        .filter_map(|(start, tok)| match tok {
            Tok::Word(w) => Some((start, PlainToken::Word(w))),
            Tok::Phrase(p) => Some((start, PlainToken::Phrase(p))),
            Tok::TagWord(n) => Some((start, PlainToken::TagWord(n))),
            _ => None,
        })
        .collect()
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn parse_query(&mut self) -> Result<Option<Expr>, QueryError> {
        if self.toks.is_empty() {
            return Ok(None);
        }
        let expr = self.parse_or()?;
        if self.pos < self.toks.len() {
            // The only token parse_or can leave over is a stray ')'.
            return Err(QueryError::UnbalancedCloseParen);
        }
        Ok(Some(expr))
    }

    fn parse_or(&mut self) -> Result<Expr, QueryError> {
        let mut items = vec![self.parse_and()?];
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            // An OR operand that turns up empty is a missing operand
            // ("a OR", "(a OR)"), not an empty group: only "()" with
            // no OR before it means EmptyGroup.
            let operand = self.parse_and().map_err(|err| {
                if err == QueryError::EmptyGroup {
                    QueryError::MissingOperand
                } else {
                    err
                }
            })?;
            items.push(operand);
        }
        if items.len() == 1 {
            Ok(items.pop().unwrap())
        } else {
            Ok(Expr::Or(items))
        }
    }

    fn parse_and(&mut self) -> Result<Expr, QueryError> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None | Some(Tok::Or) | Some(Tok::RParen) => break,
                Some(Tok::And) => {
                    // An explicit AND must sit between two factors.
                    return Err(QueryError::MissingOperand);
                }
                _ => items.push(self.parse_not()?),
            }
            if self.peek() == Some(&Tok::And) {
                self.pos += 1;
                // An explicit AND promises another factor: "a AND"
                // must error, not silently drop the operator.
                match self.peek() {
                    Some(
                        Tok::Word(_)
                        | Tok::Phrase(_)
                        | Tok::TagWord(_)
                        | Tok::TagOp(_)
                        | Tok::LParen
                        | Tok::Not,
                    ) => {}
                    _ => return Err(QueryError::MissingOperand),
                }
            }
        }
        match items.len() {
            0 => {
                if self.peek() == Some(&Tok::Or) {
                    // An OR arrived with no left operand ("OR foo").
                    Err(QueryError::MissingOperand)
                } else {
                    // Only "()" reaches here.
                    Err(QueryError::EmptyGroup)
                }
            }
            1 => Ok(items.pop().unwrap()),
            _ => Ok(Expr::And(items)),
        }
    }

    fn parse_not(&mut self) -> Result<Expr, QueryError> {
        if self.peek() == Some(&Tok::Not) {
            self.pos += 1;
            Ok(Expr::Not(Box::new(self.parse_not()?)))
        } else {
            self.parse_atom()
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, QueryError> {
        match self.toks.get(self.pos) {
            None => Err(QueryError::MissingOperand),
            Some(Tok::LParen) => {
                self.pos += 1;
                let inner = self.parse_or()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err(QueryError::UnclosedParen);
                }
                self.pos += 1;
                Ok(inner)
            }
            Some(Tok::Word(w)) => {
                self.pos += 1;
                Ok(Expr::Term(fold(w)))
            }
            Some(Tok::Phrase(p)) => {
                self.pos += 1;
                Ok(Expr::Phrase(fold(p)))
            }
            Some(Tok::TagWord(n)) => {
                self.pos += 1;
                let name = fold(n);
                Ok(Expr::TagWord { name, prefix: true })
            }
            Some(Tok::TagOp(n)) => {
                if n.is_empty() {
                    return Err(QueryError::EmptyTagName);
                }
                self.pos += 1;
                let name = fold(n);
                Ok(Expr::TagWord {
                    name,
                    prefix: false,
                })
            }
            Some(Tok::And) | Some(Tok::Or) => {
                // Reached with the operator in factor position: a
                // leading operator or a doubled one.
                Err(QueryError::MissingOperand)
            }
            Some(Tok::Not) => unreachable!("parse_not consumes every Not"),
            Some(Tok::RParen) => unreachable!("parse_and breaks before atoms see RParen"),
        }
    }
}

/// Parse a query. `Ok(None)` for an empty (or whitespace-only) query:
/// it filters nothing. Needles in the returned `Expr` are already
/// folded.
pub fn parse(raw: &str) -> Result<Option<Expr>, QueryError> {
    Parser {
        toks: lex(raw),
        pos: 0,
    }
    .parse_query()
}

/// How many `#xyz` tag words the query carries. Upstream's
/// `NVTagWordCount` (`TagSearch.m:52-60`): a count increase between
/// the previous and current query forces a full re-filter from all
/// notes, because tag words match against data the token resume does
/// not cover. `#unix:admin` counts once (the `#unix` word).
pub fn tag_word_count(raw: &str) -> usize {
    lex(raw)
        .iter()
        .filter(|tok| matches!(tok, Tok::TagWord(_)))
        .count()
}

/// Whether the query's token stream is free of operators, groups, and
/// the full-name tag operator: only then do its words, phrases, and
/// `#tag` words mean the same thing to the NV strsep tokenizer the
/// incremental resume scans with as to the parser. Two static
/// counterexamples pin the rule: `"O& ut"` parses to a plain chain
/// but the token scan would hunt the raw needle `o&`, and `tag:x`
/// parses to a full-name tag word but the strsep scan (where `:` is a
/// separator) would hunt `tag` AND `x` instead of the literal `#x`.
/// Anything else takes the naive re-filter path.
pub fn resume_eligible(raw: &str) -> bool {
    lex(raw).iter().all(|tok| {
        !matches!(
            tok,
            Tok::And | Tok::Or | Tok::Not | Tok::LParen | Tok::RParen | Tok::TagOp(_)
        )
    })
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Term(needle) => f.write_str(needle),
            Expr::Phrase(needle) => write!(f, "\"{needle}\""),
            Expr::TagWord { name, prefix } => {
                if *prefix {
                    write!(f, "#{name}")
                } else {
                    write!(f, "tag:{name}")
                }
            }
            Expr::And(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" ")?;
                    }
                    match item {
                        // An OR inside an AND needs its parens back.
                        Expr::Or(_) => write!(f, "({item})")?,
                        other => write!(f, "{other}")?,
                    }
                }
                Ok(())
            }
            Expr::Or(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" OR ")?;
                    }
                    write!(f, "{item}")?;
                }
                Ok(())
            }
            Expr::Not(inner) => match inner.as_ref() {
                Expr::And(_) | Expr::Or(_) => write!(f, "NOT ({inner})"),
                other => write!(f, "NOT {other}"),
            },
        }
    }
}

/// Does one folded needle hit the folded title or the folded body?
/// (Upstream matches title OR body OR labels per note,
/// `NoteObject.m:1850-1871`; labels join at Phase 5.)
fn needle_matches(needle: &str, folded_title: &str, folded_body: &str) -> bool {
    folded_title.contains(needle) || folded_body.contains(needle)
}

/// Match a parsed expression against already-folded fields plus the
/// note's folded tag titles (row 64's store feeding row 57's tag
/// words). Pure and naive: the oracle the incremental filter in
/// `crate::search` is property-tested against.
pub fn matches(expr: &Expr, folded_title: &str, folded_body: &str, folded_tags: &[String]) -> bool {
    match expr {
        Expr::Term(needle) | Expr::Phrase(needle) => {
            needle_matches(needle, folded_title, folded_body)
        }
        Expr::TagWord { name, prefix } => {
            // The 3.0.19 literal rule (TagSearch.m:120-124): the
            // hashtag written in the note matches. Row 57 adds the
            // tag-title side now that the store exists: `#word` is a
            // folded PREFIX match over tag titles
            // (`NVTagTitlesMatchPrefix`, TagSearch.m:126-139); the
            // full-name `tag:` operator requires the whole name. The
            // strict full-name toggle arrives with preferences
            // (Phase 19).
            let literal = format!("#{name}");
            let literal_hit = needle_matches(&literal, folded_title, folded_body);
            let folded_name = fold(name);
            let title_hit = if *prefix {
                folded_tags.iter().any(|tag| tag.starts_with(&folded_name))
            } else {
                folded_tags.contains(&folded_name)
            };
            literal_hit || title_hit
        }
        Expr::And(items) => items
            .iter()
            .all(|item| matches(item, folded_title, folded_body, folded_tags)),
        Expr::Or(items) => items
            .iter()
            .any(|item| matches(item, folded_title, folded_body, folded_tags)),
        Expr::Not(inner) => !matches(inner, folded_title, folded_body, folded_tags),
    }
}

/// Convenience over a `Note`: folds the title, body, and tag titles,
/// then matches.
pub fn matches_note(expr: &Expr, note: &Note) -> bool {
    let folded_tags: Vec<String> = crate::tags::note_tags(note)
        .iter()
        .map(|t| fold(t))
        .collect();
    matches(
        expr,
        &fold(note.title.as_str()),
        &fold(note.body.as_str()),
        &folded_tags,
    )
}

/// The naive reference filter: arena indexes of every note matching
/// `expr`, in the given order. The oracle for `crate::search`.
pub fn find_matches(expr: &Expr, notes: &[Note]) -> Vec<usize> {
    notes
        .iter()
        .enumerate()
        .filter(|(_, note)| matches_note(expr, note))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_unwrap(raw: &str) -> Expr {
        parse(raw).unwrap().expect("query parsed to None")
    }

    // NV vocabulary (map §2): bare words, implicit AND, colon
    // separator, quote-flip phrase mode.

    #[test]
    fn nv_bare_words_implicitly_and() {
        assert_eq!(
            parse_unwrap("foo bar"),
            Expr::And(vec![Expr::Term("foo".into()), Expr::Term("bar".into())])
        );
    }

    #[test]
    fn nv_colon_is_a_token_separator() {
        assert_eq!(
            parse_unwrap("foo:bar"),
            Expr::And(vec![Expr::Term("foo".into()), Expr::Term("bar".into())])
        );
        assert_eq!(
            parse_unwrap("#unix:admin"),
            Expr::And(vec![
                Expr::TagWord {
                    name: "unix".into(),
                    prefix: true
                },
                Expr::Term("admin".into())
            ])
        );
    }

    #[test]
    fn nv_needles_are_folded_at_parse() {
        assert_eq!(parse_unwrap("BÜCHER"), Expr::Term("bucher".into()));
        assert_eq!(parse_unwrap("Café"), Expr::Term("cafe".into()));
    }

    #[test]
    fn nv_quote_flips_the_whole_query_to_phrase_mode() {
        assert_eq!(
            parse_unwrap("foo \"bar baz\" qux"),
            Expr::And(vec![
                Expr::Term("foo".into()),
                Expr::Phrase("bar baz".into()),
                Expr::Term("qux".into())
            ])
        );
        // Unterminated quote extends to the end (upstream splits on
        // '"' with no error path).
        assert_eq!(
            parse_unwrap("foo \"bar baz"),
            Expr::And(vec![
                Expr::Term("foo".into()),
                Expr::Phrase("bar baz".into())
            ])
        );
    }

    #[test]
    fn empty_and_whitespace_queries_filter_nothing() {
        assert_eq!(parse(""), Ok(None));
        assert_eq!(parse("   \t\n"), Ok(None));
        // A lone empty pair of quotes is also nothing to search for.
        assert_eq!(parse("\"\""), Ok(None));
    }

    #[test]
    fn bare_hash_is_an_ordinary_term() {
        // Token length >= 2 required for a tag word (TagSearch.m:31-33)
        assert_eq!(parse_unwrap("#"), Expr::Term("#".into()));
    }

    // ALT vocabulary (TagSearch.m, delta §1): tag words.

    #[test]
    fn hash_words_are_prefix_tag_words() {
        assert_eq!(
            parse_unwrap("#unix"),
            Expr::TagWord {
                name: "unix".into(),
                prefix: true
            }
        );
        assert_eq!(
            parse_unwrap("#a #b"),
            Expr::And(vec![
                Expr::TagWord {
                    name: "a".into(),
                    prefix: true
                },
                Expr::TagWord {
                    name: "b".into(),
                    prefix: true
                }
            ])
        );
    }

    // ULTRA vocabulary (the manual's own examples).

    #[test]
    fn manual_tag_and() {
        assert_eq!(
            parse_unwrap("#startrek AND #TNG"),
            Expr::And(vec![
                Expr::TagWord {
                    name: "startrek".into(),
                    prefix: true
                },
                Expr::TagWord {
                    name: "tng".into(),
                    prefix: true
                }
            ])
        );
    }

    #[test]
    fn manual_paren_expansion() {
        assert_eq!(
            parse_unwrap("#startrek AND (worf OR geordi)"),
            Expr::And(vec![
                Expr::TagWord {
                    name: "startrek".into(),
                    prefix: true
                },
                Expr::Or(vec![Expr::Term("worf".into()), Expr::Term("geordi".into())])
            ])
        );
    }

    #[test]
    fn manual_full_example_with_not() {
        assert_eq!(
            parse_unwrap("#startrek AND (worf OR geordi) NOT picard"),
            Expr::And(vec![
                Expr::TagWord {
                    name: "startrek".into(),
                    prefix: true
                },
                Expr::Or(vec![Expr::Term("worf".into()), Expr::Term("geordi".into())]),
                Expr::Not(Box::new(Expr::Term("picard".into()))),
            ])
        );
    }

    #[test]
    fn symbol_aliases() {
        assert_eq!(parse_unwrap("a && b"), parse_unwrap("a & b"));
        assert_eq!(parse_unwrap("a && b"), parse_unwrap("a AND b"));
        assert_eq!(parse_unwrap("a || b"), parse_unwrap("a | b"));
        assert_eq!(parse_unwrap("a || b"), parse_unwrap("a OR b"));
        assert_eq!(parse_unwrap("NOT a"), parse_unwrap("!a"));
        assert_eq!(
            parse_unwrap("!a AND b"),
            Expr::And(vec![
                Expr::Not(Box::new(Expr::Term("a".into()))),
                Expr::Term("b".into())
            ])
        );
    }

    #[test]
    fn lowercase_keywords_are_terms() {
        // NV lowercased queries before tokenizing, so the operator
        // keywords must be uppercase to read as operators (row 59).
        assert_eq!(
            parse_unwrap("and or not"),
            Expr::And(vec![
                Expr::Term("and".into()),
                Expr::Term("or".into()),
                Expr::Term("not".into())
            ])
        );
    }

    #[test]
    fn precedence_parens_gt_not_gt_and_gt_or() {
        assert_eq!(
            parse_unwrap("a OR b AND c"),
            Expr::Or(vec![
                Expr::Term("a".into()),
                Expr::And(vec![Expr::Term("b".into()), Expr::Term("c".into())])
            ])
        );
        assert_eq!(
            parse_unwrap("a AND NOT b OR c"),
            Expr::Or(vec![
                Expr::And(vec![
                    Expr::Term("a".into()),
                    Expr::Not(Box::new(Expr::Term("b".into())))
                ]),
                Expr::Term("c".into())
            ])
        );
        assert_eq!(
            parse_unwrap("(a OR b) AND c"),
            Expr::And(vec![
                Expr::Or(vec![Expr::Term("a".into()), Expr::Term("b".into())]),
                Expr::Term("c".into())
            ])
        );
        assert_eq!(
            parse_unwrap("NOT (a OR b)"),
            Expr::Not(Box::new(Expr::Or(vec![
                Expr::Term("a".into()),
                Expr::Term("b".into())
            ])))
        );
    }

    #[test]
    fn tag_operator_is_full_name_form() {
        assert_eq!(
            parse_unwrap("TAG:startrek AND android"),
            Expr::And(vec![
                Expr::TagWord {
                    name: "startrek".into(),
                    prefix: false
                },
                Expr::Term("android".into())
            ])
        );
    }

    #[test]
    fn display_round_trips_the_shapes() {
        for raw in [
            "foo bar",
            "foo:bar",
            "foo \"bar baz\" qux",
            "#startrek AND (worf OR geordi) NOT picard",
            "NOT (a OR b)",
            "a b OR c d",
            "TAG:full AND #prefix",
        ] {
            let expr = parse_unwrap(raw);
            assert_eq!(
                parse_unwrap(&expr.to_string()),
                expr,
                "round trip of {raw:?}"
            );
        }
    }

    // Grammar failures are values (row 59, point 5).

    #[test]
    fn grammar_failures_are_error_values() {
        assert_eq!(parse("(foo"), Err(QueryError::UnclosedParen));
        assert_eq!(parse("foo)"), Err(QueryError::UnbalancedCloseParen));
        assert_eq!(parse("()"), Err(QueryError::EmptyGroup));
        assert_eq!(parse("foo AND"), Err(QueryError::MissingOperand));
        assert_eq!(parse("OR foo"), Err(QueryError::MissingOperand));
        assert_eq!(parse("a AND AND b"), Err(QueryError::MissingOperand));
        assert_eq!(parse("NOT"), Err(QueryError::MissingOperand));
        assert_eq!(parse("tag:"), Err(QueryError::EmptyTagName));
        assert_eq!(
            parse("no (such:query").unwrap_err().to_string(),
            "query has an unclosed '('"
        );
    }

    // The naive matcher, against hand-built folded haystacks.

    fn m(expr: &Expr, title: &str, body: &str) -> bool {
        matches(expr, &fold(title), &fold(body), &[])
    }

    fn m_tagged(expr: &Expr, title: &str, body: &str, tags: &[&str]) -> bool {
        let folded: Vec<String> = tags.iter().map(|t| fold(t)).collect();
        matches(expr, &fold(title), &fold(body), &folded)
    }

    #[test]
    fn term_matches_title_or_body_case_insensitively() {
        let e = parse_unwrap("velocity");
        assert!(m(&e, "Notational Velocity", ""));
        assert!(m(&e, "other", "the velocity of notes"));
        assert!(!m(&e, "unrelated", "nothing here"));
    }

    #[test]
    fn manual_diacritic_example() {
        // "bucher" matches files that contain "Bücher" (ultra
        // search-syntax; spec row 58)
        let e = parse_unwrap("bucher");
        assert!(m(&e, "Meine Bücher", "shelf"));
        let e = parse_unwrap("bücher");
        assert!(m(&e, "bucher list", "shelf"));
    }

    #[test]
    fn phrase_is_an_exact_ordered_needle() {
        let e = parse_unwrap("\"exact phrase\"");
        assert!(m(&e, "", "found the exact phrase here"));
        assert!(!m(&e, "", "phrase exact order scrambled"));
    }

    #[test]
    fn tag_word_literal_rule() {
        // 3.0.19: a #tag written in the title/body/labels matches
        let e = parse_unwrap("#unix");
        assert!(m(&e, "shell notes", "learning #unix basics"));
        assert!(m(&e, "#unix tips", ""));
        assert!(!m(&e, "unix without hash", "plain unix mention"));
    }

    #[test]
    fn implicit_and_spans_fields() {
        let e = parse_unwrap("warp engine");
        assert!(m(&e, "warp drive", "the engine hums"));
        assert!(!m(&e, "warp drive", "no second word"));
    }

    #[test]
    fn tag_words_consult_the_tag_store() {
        // Row 57 completed: metadata-only tags (no #hash in the text)
        // match #words by folded prefix and tag: by full name.
        let e = parse_unwrap("#startrek");
        assert!(m_tagged(&e, "log entry", "plain text", &["startrek"]));
        assert!(m_tagged(&e, "log entry", "plain text", &["startrek-tng"]));
        assert!(!m_tagged(&e, "log entry", "plain text", &["wars"]));
        let full = parse_unwrap("tag:startrek");
        assert!(m_tagged(&full, "log", "plain", &["startrek"]));
        // Full-name operator does not prefix-match.
        assert!(!m_tagged(&full, "log", "plain", &["startrek-tng"]));
    }

    #[test]
    fn not_excludes_matches() {
        let e = parse_unwrap("startrek NOT picard");
        assert!(m(&e, "startrek", "kirk and spock"));
        assert!(!m(&e, "startrek", "captain picard logs"));
    }
}
