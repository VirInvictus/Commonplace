//! Wiki links, transclusions, and the link index (spec row 65; the
//! scanner ports `AttributedPlainText.m:250-304` rule for rule).
//!
//! Extraction: scan `[[` ... `]]` pairs left to right. The interior
//! may not START with a guard character (the scan resumes right after
//! the `[[`, so inner invalid `[[`s are still found); a newline inside
//! kills the block (the scan resumes after the newline); the interior
//! may not END with a guard character; and the ObjC heuristic rejects
//! blocks whose inner brackets look unbalanced (a `]` at or before the
//! first `[`). The guard set is `[`, `]`, whitespace, and control or
//! illegal characters (`AttributedPlainText.m:240-246`).
//!
//! Upstream stored a link as a `nvalt://find/<title>` SEARCH url: a
//! link resolves through the search engine and never creates a note.
//! Commonplace keeps that search semantics and adds the recorded [CP]
//! improvement (rows 25/65): a rename rewrites inbound links whose
//! text matches the old title (folded). `{{transclusion}}` is
//! recognized with cycle detection; expansion is Phase 6/15.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::fold::fold;
use crate::note::Note;

/// One extracted `[[link]]`: the interior text plus its char span in
/// the body (start inclusive, end exclusive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// The interior guard set: brackets, whitespace, control and illegal
/// characters (`antiInteriorSet`, `AttributedPlainText.m:240-246`;
/// `is_whitespace` covers the Unicode whitespace set the guard
/// rejects).
fn is_guard(c: char) -> bool {
    matches!(c, '[' | ']') || c.is_whitespace() || c.is_control()
}

/// The ObjC heuristic (`_StringWithRangeIsProbablyObjC`,
/// `AttributedPlainText.m:291-304`): assuming the range is
/// bookended with matching double brackets, does it contain an
/// unbalanced inner bracket? A `]` strictly after the first `[`
/// looks balanced (accept); otherwise reject.
fn looks_like_objc(interior: &str) -> bool {
    let first_right = interior.find(']');
    let first_left = interior.find('[');
    match (first_right, first_left) {
        // No brackets of either variety: not ObjC (upstream returns
        // NO here first).
        (None, None) => false,
        // Both a left and a right, with the right after the left:
        // balanced-looking, not ObjC.
        (Some(r), Some(l)) => r <= l,
        // A dangling bracket of either kind: ObjC.
        _ => true,
    }
}

/// Extract the wiki links of a body, in document order.
pub fn wiki_links(body: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    let mut scan = 0usize;
    while let Some(open_rel) = body[scan..].find("[[") {
        let open = scan + open_rel;
        let interior_start = open + 2;
        let Some(end_rel) = body[interior_start..].find("]]") else {
            break;
        };
        let interior_end = interior_start + end_rel;
        let interior = &body[interior_start..interior_end];

        // First-char guard: resume right after the `[[` so inner
        // invalid `[[`s are still scanned.
        if interior.starts_with(is_guard) {
            scan = interior_start;
            continue;
        }
        // Newline trap: resume after the newline, not after the
        // `]]`. Upstream's newlineCharacterSet covers CR, NEL, and
        // the Unicode line/paragraph separators too.
        if let Some((nl_rel, nl_char)) = interior
            .char_indices()
            .find(|(_, c)| matches!(*c, '\n' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}'))
        {
            scan = interior_start + nl_rel + nl_char.len_utf8();
            continue;
        }
        // Last-char guard or ObjC shape: no link here, but the scan
        // continues after the `]]`.
        let last_ok = interior
            .chars()
            .next_back()
            .map(|c| !is_guard(c))
            .unwrap_or(false);
        if last_ok && !looks_like_objc(interior) {
            links.push(WikiLink {
                start: interior_start,
                end: interior_end,
                text: interior.to_owned(),
            });
        }
        scan = interior_end + 2;
    }
    links
}

/// The `{{transclusion}}` targets in a body, in document order.
/// Grammar pinned simply (row 65): a non-empty interior with no
/// braces and no newline.
pub fn transclusions(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut scan = 0usize;
    while let Some(open_rel) = body[scan..].find("{{") {
        let open = scan + open_rel;
        let interior_start = open + 2;
        let Some(end_rel) = body[interior_start..].find("}}") else {
            break;
        };
        let interior = &body[interior_start..interior_start + end_rel];
        let trimmed = interior.trim();
        if !trimmed.is_empty()
            && !trimmed.contains('{')
            && !trimmed.contains('}')
            && !trimmed.contains('\n')
        {
            out.push(trimmed.to_owned());
        }
        scan = interior_start + end_rel + 2;
    }
    out
}

/// Rewrite inbound links for a rename (row 65's [CP] improvement):
/// every link whose folded text matches `old_folded` gets its
/// interior replaced with the new title. `None` when the body carries
/// no matching link (callers skip the write).
pub fn rewrite_links_for_rename(body: &str, old_folded: &str, new_title: &str) -> Option<String> {
    let links = wiki_links(body);
    let mut changed = false;
    let mut out = String::with_capacity(body.len());
    let mut cursor = 0usize;
    for link in links {
        if fold(&link.text) == old_folded {
            out.push_str(&body[cursor..link.start]);
            out.push_str(new_title);
            cursor = link.end;
            changed = true;
        }
    }
    if !changed {
        return None;
    }
    out.push_str(&body[cursor..]);
    Some(out)
}

/// The derived link index: folded link text -> the arena indexes of
/// notes whose bodies link to it. Rebuildable from bodies alone.
#[derive(Debug, Default, Clone)]
pub struct LinkIndex {
    backlinks: HashMap<String, Vec<usize>>,
}

impl LinkIndex {
    pub fn build(notes: &[Note]) -> Self {
        let mut backlinks: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, note) in notes.iter().enumerate() {
            for link in wiki_links(note.body.as_str()) {
                backlinks.entry(fold(&link.text)).or_default().push(i);
            }
        }
        LinkIndex { backlinks }
    }

    /// The arena indexes of notes whose bodies link to the given
    /// display title (folded). An unlinked title answers empty.
    pub fn backlinks_to(&self, display_title: &str) -> &[usize] {
        self.backlinks
            .get(&fold(display_title))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

impl crate::store::Store {
    /// The derived link index over the loaded notes; rebuilt per call
    /// and after any mutation (derived and safe to delete).
    pub fn link_index(&self) -> LinkIndex {
        LinkIndex::build(self.notes())
    }

    /// Follow `{{transclusions}}` from a note and detect a cycle.
    /// `Some(cycle)` returns the walk including the repeated path;
    /// `None` means the reachable graph is acyclic. Targets resolve by
    /// folded display title first, then by file stem (extension
    /// tolerance); an unresolvable target ends that branch.
    pub fn transclusion_cycle_from(&self, path: &Path) -> Option<Vec<PathBuf>> {
        let start = self.find_by_path(path)?;
        let mut stack: Vec<PathBuf> = vec![start.path.clone()];
        let mut visited: Vec<PathBuf> = vec![start.path.clone()];
        self.walk_transclusions(&mut stack, &mut visited)
    }

    fn walk_transclusions(
        &self,
        stack: &mut Vec<PathBuf>,
        visited: &mut Vec<PathBuf>,
    ) -> Option<Vec<PathBuf>> {
        let current = stack.last()?;
        let note = self.find_by_path(current)?;
        for target in transclusions(note.body.as_str()) {
            // An unresolvable target ends ITS BRANCH only, never the
            // walk: a missing sibling must not hide a real cycle
            // behind it.
            let Some(target_path) = self.resolve_target(&target) else {
                continue;
            };
            if stack.contains(&target_path) {
                // A cycle: the stack plus the repeated path.
                let mut cycle = stack.clone();
                cycle.push(target_path);
                return Some(cycle);
            }
            if visited.contains(&target_path) {
                // Already fully walked without a cycle.
                continue;
            }
            visited.push(target_path.clone());
            stack.push(target_path);
            if let Some(cycle) = self.walk_transclusions(stack, visited) {
                return Some(cycle);
            }
            stack.pop();
        }
        None
    }

    /// Resolve a transclusion/link target to a loaded note path:
    /// folded display title first, then file stem.
    fn resolve_target(&self, target: &str) -> Option<PathBuf> {
        let folded = fold(target);
        for note in self.notes() {
            if fold(note.display_title()) == folded {
                return Some(note.path.clone());
            }
        }
        for note in self.notes() {
            if let Some(stem) = note.path.file_stem()
                && fold(&stem.to_string_lossy()) == folded
            {
                return Some(note.path.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scanner goldens: contract section 3B, rule for rule.

    #[test]
    fn simple_pairs_extract() {
        let links = wiki_links("see [[Target Note]] here");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "Target Note");
    }

    #[test]
    fn two_links_extract_in_order() {
        let links = wiki_links("[[a]] then [[b]]");
        assert_eq!(
            links.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn first_char_guard_rejects_and_rescans_after_the_brackets() {
        // The interior starts with whitespace: rejected; the inner
        // "[[real]]" (after the whitespace) is still found because the
        // scan resumes after the "[[".
        let links = wiki_links("[[ real]] and [[real]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "real");
    }

    #[test]
    fn last_char_guard_rejects_but_the_scan_passes_the_brackets() {
        let links = wiki_links("[[real ]] plus [[ok]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "ok");
    }

    #[test]
    fn interior_brackets_are_allowed_when_balanced() {
        let links = wiki_links("[[a [b] c]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "a [b] c");
    }

    #[test]
    fn newline_kills_the_link() {
        let links = wiki_links("[[multi\nline]]");
        assert_eq!(links, &[]);
    }

    #[test]
    fn objc_blocks_are_excluded() {
        // The heuristic's shape: an unbalanced inner bracket rejects
        // the whole span (the upstream comment admits the compound
        // case still slips through; that limitation ports too).
        let links = wiki_links("[[Class alloc] init] and [[real link]]");
        assert_eq!(links, &[]);
    }

    #[test]
    fn unterminated_pairs_extract_nothing() {
        assert_eq!(wiki_links("[[no closer here"), &[]);
    }

    #[test]
    fn char_spans_are_exact() {
        let body = "before [[mid]] after";
        let links = wiki_links(body);
        assert_eq!(links[0].text, "mid");
        assert_eq!(&body[links[0].start..links[0].end], "mid");
    }

    // Transclusion grammar.

    #[test]
    fn transclusions_extract() {
        assert_eq!(
            transclusions("lead {{Embedded}} and {{Other}} tail"),
            ["Embedded", "Other"]
        );
    }

    #[test]
    fn transclusion_guards() {
        // empty, nested-brace, and newline interiors are not targets
        assert_eq!(
            transclusions("{{}} {{a{{b}} {{a\nb}}"),
            Vec::<String>::new()
        );
    }

    // The rename rewrite (row 65's [CP] improvement).

    #[test]
    fn rewrite_matches_folded_and_preserves_the_rest() {
        let body = "see [[Old Title]] and [[old title]] but [[Unrelated]] stays";
        let rewritten = rewrite_links_for_rename(body, &fold("Old Title"), "New Title").unwrap();
        assert_eq!(
            rewritten,
            "see [[New Title]] and [[New Title]] but [[Unrelated]] stays"
        );
    }

    #[test]
    fn no_matching_links_means_no_rewrite() {
        assert_eq!(
            rewrite_links_for_rename("[[Other]] only", &fold("Old Title"), "New Title"),
            None
        );
    }

    #[test]
    fn folded_matching_across_accents() {
        let body = "link [[Café Notes]] here";
        let rewritten = rewrite_links_for_rename(body, &fold("Cafe Notes"), "New Name").unwrap();
        assert_eq!(rewritten, "link [[New Name]] here");
    }
}
