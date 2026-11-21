//! The preview-prep pass (spec row 68): a dependency-free port of
//! nvALT's `NVPreviewHTMLPreparingContent` and
//! `NVPreviewHTMLNeedsCodeHighlight` (`PreviewRender.m`, map §5).
//! Upstream drove NSRegularExpression over the rendered HTML; the
//! port's renderer output is comrak's, but the needles and rewrites
//! are the same contracts, so the scanners below hand-roll exactly
//! those patterns over bytes.

/// What the prepared HTML needs the preview page to have loaded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreviewNeeds {
    pub math: bool,
    pub mermaid: bool,
    pub wiki: bool,
}

/// The exact upstream task CSS, prepended as a `<style>` block when
/// any checkbox exists in the HTML (`NVPreviewTaskListCSS`).
const TASK_CSS: &str = "li input[type=\"checkbox\"]{margin-right:0.4em;vertical-align:text-bottom;}\
                        li.nv-task-done{text-decoration:line-through;}\
                        li.nv-task-done input{text-decoration:none;}";

/// Prepare rendered HTML for the preview page and report which
/// renderers it needs (row 68): mermaid fences rewritten into
/// `<pre class="mermaid">` blocks, completed task items marked
/// `nv-task-done` with the task CSS prepended, and the math/mermaid/
/// wiki detection flags computed over the prepared result.
pub fn preview_prep(html: &str) -> (String, PreviewNeeds) {
    let mut prepared = rewrite_mermaid_fences(html);

    let needs = PreviewNeeds {
        // Upstream's exact needles: `class="math "` (the open-value
        // prefix, matching `class="math display"`) or the bare
        // `class="math"`.
        math: find_ci(prepared.as_bytes(), b"class=\"math ", 0).is_some()
            || find_ci(prepared.as_bytes(), b"class=\"math\"", 0).is_some(),
        mermaid: find_ci(prepared.as_bytes(), b"class=\"mermaid\"", 0).is_some(),
        wiki: prepared.contains("[["),
    };

    mark_completed_tasks(&mut prepared);
    (prepared, needs)
}

/// True when the (prepared) HTML has a code block: a `<pre>` whose
/// first element is `<code>`. Mermaid diagrams no longer match because
/// [`preview_prep`] rewrote them first; inline code has no `<pre>`.
pub fn needs_code_highlight(html: &str) -> bool {
    let bytes = html.as_bytes();
    let mut scan = 0usize;
    while let Some(rel) = find_ci(&bytes[scan..], b"<pre", 0) {
        let pre = scan + rel;
        if !tag_boundary(bytes, pre + 4) {
            scan = pre + 4;
            continue;
        }
        let Some(gt) = memchr(bytes, b'>', pre + 4) else {
            return false;
        };
        let code = skip_ws(bytes, gt + 1);
        if starts_with_ci(&bytes[code..], b"<code") && tag_boundary(bytes, code + 5) {
            return true;
        }
        scan = gt + 1;
    }
    false
}

// -- mermaid fence rewrite ---------------------------------------------------

/// Rewrite mermaid fences (both upstream shapes, case-insensitive) in
/// one forward pass: `<pre ...><code class="language-{?mermaid?}">
/// source</code></pre>` and `<pre lang="{?mermaid?}" ...><code>
/// source</code></pre>` become `<pre class="mermaid">source</pre>`.
/// The out buffer always receives the skipped spans, so nothing is
/// dropped between matches.
fn rewrite_mermaid_fences(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut scan = 0usize;
    while let Some(rel) = find_ci(&bytes[scan..], b"<pre", 0) {
        let pre = scan + rel;
        if !tag_boundary(bytes, pre + 4) {
            out.push_str(&html[scan..pre + 4]);
            scan = pre + 4;
            continue;
        }
        let Some(pre_gt) = memchr(bytes, b'>', pre + 4) else {
            out.push_str(&html[scan..pre + 4]);
            scan = pre + 4;
            continue;
        };
        let pre_attrs = &html[pre + 4..pre_gt];
        let code = skip_ws(bytes, pre_gt + 1);
        if !starts_with_ci(&bytes[code..], b"<code") || !tag_boundary(bytes, code + 5) {
            out.push_str(&html[scan..pre_gt + 1]);
            scan = pre_gt + 1;
            continue;
        }
        let Some(code_gt) = memchr(bytes, b'>', code + 5) else {
            out.push_str(&html[scan..pre_gt + 1]);
            scan = pre_gt + 1;
            continue;
        };
        let code_attrs = &html[code + 5..code_gt];

        let is_mermaid = attrs_match_shape1(pre_attrs, code_attrs) || attrs_match_shape2(pre_attrs);
        let body_start = code_gt + 1;
        let end = if is_mermaid {
            // Lazy terminator: the first `</code></pre>` after the code tag.
            find_ci(&bytes[body_start..], b"</code></pre>", 0).map(|rel| body_start + rel)
        } else {
            None
        };
        match end {
            Some(end) => {
                out.push_str(&html[scan..pre]);
                out.push_str("<pre class=\"mermaid\">");
                out.push_str(&html[body_start..end]);
                out.push_str("</pre>");
                scan = end + b"</code></pre>".len();
            }
            None => {
                out.push_str(&html[scan..pre_gt + 1]);
                scan = pre_gt + 1;
            }
        }
    }
    out.push_str(&html[scan..]);
    out
}

/// Upstream pattern 1: `<pre\b[^>]*>\s*<code\b[^>]*\bclass="language-\{?mermaid\}?"[^>]*>`.
fn attrs_match_shape1(_pre_attrs: &str, code_attrs: &str) -> bool {
    contains_word_ci(code_attrs, b"class=", Some(b"language-mermaid".as_slice()))
        || contains_word_ci(
            code_attrs,
            b"class=",
            Some(b"language-{mermaid}".as_slice()),
        )
}

/// Upstream pattern 2: `<pre\b[^>]*\blang="\{?mermaid\}?"[^>]*>`.
fn attrs_match_shape2(pre_attrs: &str) -> bool {
    contains_word_ci(pre_attrs, b"lang=", Some(b"mermaid".as_slice()))
        || contains_word_ci(pre_attrs, b"lang=", Some(b"{mermaid}".as_slice()))
}

/// A case-insensitive attribute check where `attr` must start at a word
/// boundary inside the attribute string (upstream's `\battr=`). With
/// `Some(value)` the exact quoted `attr="value"` form must follow; with
/// `None` the attribute literal alone suffices (upstream's `checked=`
/// lookahead accepts any value form).
fn contains_word_ci(attrs: &str, attr: &[u8], value: Option<&[u8]>) -> bool {
    let bytes = attrs.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = find_ci(&bytes[from..], attr, 0) {
        let at = from + rel;
        if at == 0 || !is_word_byte(bytes[at - 1]) {
            let after = at + attr.len();
            let ok = match value {
                Some(v) => {
                    let mut quoted = Vec::with_capacity(v.len() + 2);
                    quoted.push(b'"');
                    quoted.extend_from_slice(v);
                    quoted.push(b'"');
                    starts_with_ci(&bytes[after..], &quoted)
                }
                None => true,
            };
            if ok {
                return true;
            }
        }
        from = at + 1;
    }
    false
}

// -- completed-task marking ---------------------------------------------------

/// Upstream `NVMarkCompletedPreviewTasks`: `<li ...>` whose first
/// element is `<input ...>` carrying both `type="checkbox"` and
/// `checked=` gains `class="nv-task-done"`; the task CSS style block
/// is prepended when any checkbox exists at all. The out buffer always
/// receives the skipped spans, so nothing is dropped between matches.
fn mark_completed_tasks(html: &mut String) {
    if find_ci(html.as_bytes(), b"type=\"checkbox\"", 0).is_none() {
        return;
    }
    let mut marked = String::with_capacity(html.len() + TASK_CSS.len() + 8);
    marked.push_str("<style>");
    marked.push_str(TASK_CSS);
    marked.push_str("</style>");

    let bytes = html.as_bytes();
    let mut scan = 0usize;
    while let Some(rel) = find_ci(&bytes[scan..], b"<li", 0) {
        let li = scan + rel;
        if !tag_boundary(bytes, li + 3) {
            marked.push_str(&html[scan..li + 3]);
            scan = li + 3;
            continue;
        }
        let Some(li_gt) = memchr(bytes, b'>', li + 3) else {
            marked.push_str(&html[scan..li + 3]);
            scan = li + 3;
            continue;
        };
        let li_attrs = &html[li + 3..li_gt];
        let input = skip_ws(bytes, li_gt + 1);
        let checked_task =
            if starts_with_ci(&bytes[input..], b"<input") && tag_boundary(bytes, input + 6) {
                memchr(bytes, b'>', input + 6).and_then(|in_gt| {
                    let attrs = &html[input + 6..in_gt];
                    (contains_word_ci(attrs, b"type=", Some(b"checkbox".as_slice()))
                        && contains_word_ci(attrs, b"checked=", None))
                    .then_some(in_gt)
                })
            } else {
                None
            };
        match checked_task {
            Some(in_gt) => {
                marked.push_str(&html[scan..li]);
                marked.push_str("<li class=\"nv-task-done\"");
                marked.push_str(li_attrs);
                marked.push('>');
                marked.push_str(&html[li_gt + 1..in_gt + 1]);
                scan = in_gt + 1;
            }
            None => {
                marked.push_str(&html[scan..li_gt + 1]);
                scan = li_gt + 1;
            }
        }
    }
    marked.push_str(&html[scan..]);
    *html = marked;
}

// -- byte scanning helpers -----------------------------------------------------
//
// All needles are ASCII and UTF-8 is self-synchronizing, so every byte
// offset these helpers return is a char boundary in the source string.

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Upstream's `\b` after a tag name: the byte at `at` must not
/// continue a word.
fn tag_boundary(bytes: &[u8], at: usize) -> bool {
    at >= bytes.len() || !is_word_byte(bytes[at])
}

fn starts_with_ci(hay: &[u8], prefix: &[u8]) -> bool {
    hay.len() >= prefix.len()
        && hay[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn memchr(bytes: &[u8], needle: u8, from: usize) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|&b| b == needle)
        .map(|rel| from + rel)
}

fn skip_ws(bytes: &[u8], at: usize) -> usize {
    let mut i = at;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Case-insensitive ASCII search; byte offsets only, safe on UTF-8
/// because the needles are ASCII (no multi-byte sequence can complete
/// a false ASCII match).
fn find_ci(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    debug_assert!(!needle.is_empty());
    if needle.len() > hay.len() {
        return None;
    }
    for start in from..=hay.len() - needle.len() {
        if hay[start..start + needle.len()]
            .iter()
            .zip(needle.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
        {
            return Some(start);
        }
    }
    None
}
