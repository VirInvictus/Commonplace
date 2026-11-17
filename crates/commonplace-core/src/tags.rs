//! The tag store (spec row 64): a note's tags are its `Tags:`
//! metadata value (nvUltra's comma-separated convention) PLUS the
//! inline `#tags` found in the body text ([CP] superset), and the
//! indexes over them are derived structures, rebuilt from bodies
//! alone, never authoritative. No Finder-tag parity: the recorded
//! divergence (map §7 note 5).
//!
//! The inline grammar is pinned by Commonplace (nvUltra's exact
//! inline rule is undocumented): a `#` at a word boundary followed by
//! an alphanumeric, continuing with alphanumerics, hyphens, and
//! underscores, ending at the first other character. The scan covers
//! the whole body including any metadata block - a `#word` inside a
//! metadata value counts, the same as nvUltra's literal-`#xyz` search
//! rule (row 57).

use std::collections::HashMap;

use crate::metadata;
use crate::note::Note;

/// The inline `#tags` in a body, in first-appearance order, each
/// still in its original case (matching folds later).
pub fn inline_tags(body: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut chars = body.char_indices().peekable();
    let mut prev_alphanumeric = false;
    while let Some((i, c)) = chars.next() {
        if c == '#' && !prev_alphanumeric {
            // A tag candidate: at least one alphanumeric, then the
            // tag charset.
            let mut end = i + 1;
            let mut started = false;
            for (_, tc) in chars.by_ref() {
                if tc.is_alphanumeric() {
                    started = true;
                    end += tc.len_utf8();
                } else if started && (tc == '-' || tc == '_') {
                    end += tc.len_utf8();
                } else {
                    break;
                }
            }
            if started {
                tags.push(body[i + 1..end].to_owned());
                // The character that broke the run decides the next
                // boundary; recompute from it next loop.
                prev_alphanumeric = body[..end]
                    .chars()
                    .next_back()
                    .map(|c| c.is_alphanumeric())
                    .unwrap_or(false);
                continue;
            }
        }
        prev_alphanumeric = c.is_alphanumeric();
    }
    tags
}

/// A note's full tag set: metadata `Tags:` first, then inline `#tags`
/// not already present (first occurrence keeps its case; comparison
/// folds). The `Tags:` metadata and inline `#tags` never double-count
/// the same folded tag.
pub fn note_tags(note: &Note) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    let has = |candidate: &str, tags: &Vec<String>| {
        tags.iter()
            .any(|t| t.to_lowercase() == candidate.to_lowercase())
    };
    for tag in metadata::parse_metadata(note.body.as_str()).metadata.tags() {
        if !has(&tag, &tags) {
            tags.push(tag);
        }
    }
    for tag in inline_tags(note.body.as_str()) {
        if !has(&tag, &tags) {
            tags.push(tag);
        }
    }
    tags
}

/// The derived tag index: folded tag name -> the arena indexes of
/// every note carrying it (by metadata or inline). Rebuilt from
/// scratch by [`TagIndex::build`]; discard and rebuild after any
/// mutation.
#[derive(Debug, Default, Clone)]
pub struct TagIndex {
    index: HashMap<String, Vec<usize>>,
}

impl TagIndex {
    pub fn build(notes: &[Note]) -> Self {
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, note) in notes.iter().enumerate() {
            let mut seen: Vec<String> = Vec::new();
            for tag in note_tags(note) {
                let folded = tag.to_lowercase();
                if seen.contains(&folded) {
                    continue;
                }
                seen.push(folded.clone());
                index.entry(folded).or_default().push(i);
            }
        }
        TagIndex { index }
    }

    /// The arena indexes of notes carrying this tag (folded lookup;
    /// a leading `#` on the query is accepted and ignored, so search
    /// tokens work unmodified).
    pub fn notes_for(&self, tag: &str) -> &[usize] {
        let name = tag.strip_prefix('#').unwrap_or(tag).to_lowercase();
        self.index.get(&name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// The distinct folded tags in the index, sorted.
    pub fn tags(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.index.keys().map(|k| k.as_str()).collect();
        keys.sort();
        keys
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

impl crate::store::Store {
    /// The derived tag index over the loaded notes: rebuilt on every
    /// call (it is derived and safe to delete, invariant 1); rebuild
    /// after any mutation before relying on it.
    pub fn tag_index(&self) -> TagIndex {
        TagIndex::build(self.notes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{NoteBody, NoteTitle};
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn note(title: &str, body: &str) -> Note {
        let title = NoteTitle::from_body(title);
        Note {
            path: PathBuf::from(format!("{}.md", title.as_str())),
            title_override: None,
            folded_title: crate::fold::fold(title.as_str()).into_boxed_str(),
            title,
            body: NoteBody::of(body),
            created: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            size: 0,
        }
    }

    // The inline grammar (row 64's pinned rule).

    #[test]
    fn simple_inline_tags() {
        assert_eq!(inline_tags("a #tag in text"), ["tag"]);
        assert_eq!(inline_tags("#alpha and #beta"), ["alpha", "beta"]);
    }

    #[test]
    fn hash_needs_a_word_boundary() {
        // mid-word # is not a tag
        assert_eq!(inline_tags("abc#def is not"), Vec::<String>::new());
        // punctuation before is a boundary
        assert_eq!(inline_tags("(#tagged) here"), ["tagged"]);
        assert_eq!(inline_tags("start-of-line\n#next"), ["next"]);
    }

    #[test]
    fn trailing_punctuation_terminates() {
        assert_eq!(inline_tags("the #tag."), ["tag"]);
        assert_eq!(inline_tags("#tag, #other"), ["tag", "other"]);
    }

    #[test]
    fn hyphens_and_underscores_continue_but_must_not_start() {
        assert_eq!(inline_tags("#deep-space-nine"), ["deep-space-nine"]);
        assert_eq!(inline_tags("#my_tag"), ["my_tag"]);
        // a tag cannot START with a hyphen or underscore
        assert_eq!(inline_tags("#-not-a-tag"), Vec::<String>::new());
        assert_eq!(inline_tags("#_also-not"), Vec::<String>::new());
    }

    #[test]
    fn unicode_tags() {
        assert_eq!(inline_tags("café #café time"), ["café"]);
    }

    #[test]
    fn the_metadata_tags_key_is_comma_separated() {
        let n = note("t", "tags: startrek, tng\n\nbody");
        assert_eq!(note_tags(&n), ["startrek", "tng"]);
    }

    #[test]
    fn metadata_and_inline_dedup_folded() {
        let n = note("t", "tags: startrek\n\n#STARTREK appears again");
        assert_eq!(note_tags(&n), ["startrek"]);
    }

    #[test]
    fn inline_tags_from_the_metadata_block_count() {
        // The scan covers the whole body: a #word inside a metadata
        // value counts (documented pin in the module header).
        let n = note("t", "Title: my #weird note\n\nbody");
        assert_eq!(note_tags(&n), ["weird"]);
    }

    #[test]
    fn tag_index_maps_folded_tags_to_notes() {
        let notes = vec![
            note("First", "about #startrek"),
            note("Second", "tags: TNG\n\nbody"),
            note("Third", "#startrek and #tng"),
            note("Fourth", "no tags at all"),
        ];
        let index = TagIndex::build(&notes);
        assert_eq!(index.tags(), ["startrek", "tng"]);
        assert_eq!(index.notes_for("startrek"), [0, 2]);
        assert_eq!(index.notes_for("TNG"), [1, 2]);
        // Folded lookup: case-insensitive.
        assert_eq!(index.notes_for("#Startrek"), [0, 2]);
        assert_eq!(index.notes_for("nonexistent"), &[] as &[usize]);
    }

    #[test]
    fn store_builds_the_index() {
        let dir = std::env::temp_dir().join(format!(
            "commonplace-tags-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "First\nabout #startrek\n").unwrap();
        std::fs::write(dir.join("b.md"), "Second\n#startrek again\n").unwrap();
        let store = crate::store::load_folder(&dir).unwrap().store;
        let index = store.tag_index();
        assert_eq!(index.notes_for("STARTREK"), [0, 1]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
