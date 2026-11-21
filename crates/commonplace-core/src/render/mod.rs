//! The render boundary (spec row 66): Markdown-to-HTML is a trait here,
//! dependency-free, and the frontend crates wire comrak behind it. The
//! pre-render [CP] pipeline also lives here: transclusion expansion
//! (row 67) before any renderer sees the body, and the preview-prep
//! detection pass in [`preview`] (row 68) over the rendered HTML.

pub mod preview;

/// Maximum transclusion expansion depth (row 67's runaway guard): a
/// chain longer than this keeps its deeper `{{...}}` interiors literal.
pub const MAX_EXPANSION_DEPTH: usize = 16;

/// Markdown to HTML. Implementations own their option surface; core
/// only pins the boundary so the store, export, and both frontends
/// share one pipeline without dragging a renderer dependency into the
/// engine.
pub trait Renderer {
    fn render_markdown(&self, markdown: &str) -> String;
}

/// Dependency-free test stub (row 66): renders plain text, not
/// Markdown. Each blank-line-separated paragraph becomes one escaped
/// `<p>`; deterministic and well-formed by construction, which makes it
/// the fixed oracle for export and pipeline tests.
pub struct StubRenderer;

impl Renderer for StubRenderer {
    fn render_markdown(&self, markdown: &str) -> String {
        let mut out = String::new();
        let mut para = String::new();
        let mut para_open = false;
        for line in markdown.lines() {
            if line.trim().is_empty() {
                if para_open {
                    push_paragraph(&mut out, &para);
                    para.clear();
                    para_open = false;
                }
            } else {
                if para_open {
                    para.push('\n');
                }
                para.push_str(line);
                para_open = true;
            }
        }
        if para_open {
            push_paragraph(&mut out, &para);
        }
        out
    }
}

fn push_paragraph(out: &mut String, text: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str("<p>");
    out.push_str(&escape_html(text));
    out.push_str("</p>");
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Expand `{{title}}` transclusions depth-first against `resolve`
/// (row 67). The resolver receives the trimmed interior and answers
/// with the target body; the engine's store lookup is a thin adapter
/// over this closure shape. Contracts: folded exact-title equality is
/// the resolver's job (it keys its own matching); an unresolvable
/// target stays literal and never stops the walk; a repeat of an
/// in-flight folded interior rejects that cycle (the offending literal
/// `{{...}}` stays, the branch ends); depth beyond
/// [`MAX_EXPANSION_DEPTH`] stays literal.
///
/// This convenience form runs without a root key, so it cannot see a
/// cycle that resolves back to the root body itself; every real caller
/// (export, the preview pipeline) should prefer
/// [`expand_transclusions_rooted`].
pub fn expand_transclusions(body: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    expand_transclusions_rooted(body, None, resolve)
}

/// [`expand_transclusions`] with the root body's identity seeded: pass
/// `Some(title)` when `body` belongs to a note of that title (any case
/// form; the key is folded here), so a transclusion resolving back to
/// the root (the A -> B -> A shape) is caught as a cycle instead of
/// expanding the root a second time.
pub fn expand_transclusions_rooted(
    body: &str,
    root_key: Option<&str>,
    resolve: &dyn Fn(&str) -> Option<String>,
) -> String {
    let mut out = String::with_capacity(body.len());
    let mut stack: Vec<String> = root_key.map(crate::fold::fold).into_iter().collect();
    expand_into(body, resolve, &mut stack, 0, &mut out);
    out
}

fn expand_into(
    body: &str,
    resolve: &dyn Fn(&str) -> Option<String>,
    stack: &mut Vec<String>,
    depth: usize,
    out: &mut String,
) {
    if depth >= MAX_EXPANSION_DEPTH {
        out.push_str(body);
        return;
    }
    let mut scan = 0usize;
    while let Some(open_rel) = body[scan..].find("{{") {
        let open = scan + open_rel;
        let interior_start = open + 2;
        let Some(end_rel) = body[interior_start..].find("}}") else {
            out.push_str(&body[scan..]);
            return;
        };
        let interior_raw = &body[interior_start..interior_start + end_rel];
        let close = interior_start + end_rel + 2;
        let interior = interior_raw.trim();
        let expandable = !interior.is_empty()
            && !interior.contains('{')
            && !interior.contains('}')
            && !interior.contains('\n');
        if !expandable {
            // Invalid interior: the whole `{{...}}` span stays literal.
            out.push_str(&body[scan..close]);
            scan = close;
            continue;
        }
        let key = crate::fold::fold(interior);
        let resolved = if stack.contains(&key) {
            // Cycle: the branch ends with the literal span.
            None
        } else {
            resolve(interior)
        };
        match resolved {
            Some(target) => {
                out.push_str(&body[scan..open]);
                stack.push(key);
                expand_into(&target, resolve, stack, depth + 1, out);
                stack.pop();
                scan = close;
            }
            None => {
                out.push_str(&body[scan..close]);
                scan = close;
            }
        }
    }
    out.push_str(&body[scan..]);
}
