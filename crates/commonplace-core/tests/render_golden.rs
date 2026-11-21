//! Golden tests for the render boundary (spec rows 66-68): the stub
//! renderer's contract, transclusion expansion (rows 65/67), and the
//! preview-prep detection pass ported from PreviewRender.m.

use commonplace_core::render::preview::{needs_code_highlight, preview_prep};
use commonplace_core::render::{
    Renderer, StubRenderer, expand_transclusions, expand_transclusions_rooted,
};

// ---------------------------------------------------------------------------
// Row 66: the Renderer trait contract, exercised on the dependency-free stub.
// ---------------------------------------------------------------------------

#[test]
fn stub_empty_body_renders_nothing() {
    assert_eq!(StubRenderer.render_markdown(""), "");
    assert_eq!(StubRenderer.render_markdown("\n\n"), "");
}

#[test]
fn stub_renders_one_paragraph_per_blank_line_split() {
    assert_eq!(StubRenderer.render_markdown("hello"), "<p>hello</p>");
    assert_eq!(
        StubRenderer.render_markdown("first\n\nsecond\n\n\nthird"),
        "<p>first</p>\n<p>second</p>\n<p>third</p>"
    );
}

#[test]
fn stub_escapes_html_specials() {
    assert_eq!(
        StubRenderer.render_markdown("a <b> & c"),
        "<p>a &lt;b&gt; &amp; c</p>"
    );
}

#[test]
fn stub_is_deterministic() {
    let body = "# Title\n\nsome **text** & more";
    assert_eq!(
        StubRenderer.render_markdown(body),
        StubRenderer.render_markdown(body)
    );
}

// ---------------------------------------------------------------------------
// Row 67: transclusion expansion (pre-render [CP]).
// ---------------------------------------------------------------------------

/// Resolver over a fixed map; the engine-side store lookup is a thin
/// adapter over the same closure shape.
fn resolver<'a>(map: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name: &str| {
        map.iter()
            .find(|(k, _)| commonplace_core::fold::fold(k) == commonplace_core::fold::fold(name))
            .map(|(_, body)| (*body).to_string())
    }
}

#[test]
fn transclusion_expands_resolved_target_verbatim() {
    let out = expand_transclusions("A {{B}} end", &resolver(&[("B", "body of B")]));
    assert_eq!(out, "A body of B end");
}

#[test]
fn transclusion_folds_target_names_for_resolution() {
    let out = expand_transclusions("A {{b}} end", &resolver(&[("B", "body of B")]));
    assert_eq!(out, "A body of B end");
}

#[test]
fn unresolvable_target_stays_literal_and_does_not_stop_the_walk() {
    let out = expand_transclusions("x {{Missing}} y {{Real}} z", &resolver(&[("Real", "R")]));
    assert_eq!(out, "x {{Missing}} y R z");
}

#[test]
fn nested_transclusions_expand_recursively() {
    let out = expand_transclusions(
        "top {{Mid}}",
        &resolver(&[("Mid", "m {{Deep}}"), ("Deep", "d")]),
    );
    assert_eq!(out, "top m d");
}

#[test]
fn transclusion_cycle_leaves_the_offender_literal() {
    // A -> B -> A, root A: the inner `{{A}}` is the cycle-creating
    // transclusion and stays literal; the walk ends that branch only.
    let out = expand_transclusions_rooted(
        "x {{B}} y",
        Some("A"),
        &resolver(&[("B", "b {{A}}"), ("A", "x {{B}} y")]),
    );
    assert_eq!(out, "x b {{A}} y");
}

#[test]
fn self_transclusion_leaves_itself_literal() {
    let out = expand_transclusions_rooted(
        "s {{Self}}",
        Some("Self"),
        &resolver(&[("Self", "s {{Self}}")]),
    );
    assert_eq!(out, "s {{Self}}");
}

#[test]
fn transclusion_depth_caps_at_16() {
    // A chain of 20 single-line notes; the cap allows 16 nested
    // expansions, so note 17's body is emitted verbatim at depth 16 and
    // its `{{18}}` stays literal (the leaf never loads).
    let map: Vec<(String, String)> = (1..=20)
        .map(|i| {
            let body = if i == 20 {
                String::from("leaf")
            } else {
                format!("{} {{{{{}}}}}", i, i + 1)
            };
            (i.to_string(), body)
        })
        .collect();
    let refs: Vec<(&str, &str)> = map.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let out = expand_transclusions_rooted(&map[0].1, Some("1"), &resolver(&refs));
    assert!(
        out.ends_with("17 {{18}}"),
        "depth cap leaves note 17 verbatim with {{18}} literal: {out}"
    );
    assert!(
        !out.contains("leaf"),
        "nothing expanded past the cap: {out}"
    );
}

#[test]
fn invalid_transclusion_interiors_stay_literal() {
    let resolve = |_: &str| -> Option<String> { Some(String::from("SHOULD NOT APPEAR")) };
    assert_eq!(expand_transclusions("a {{}} b", &resolve), "a {{}} b");
    assert_eq!(
        expand_transclusions("a {{ {x} }} b", &resolve),
        "a {{ {x} }} b"
    );
    assert_eq!(
        expand_transclusions("a {{x\ny}} b", &resolve),
        "a {{x\ny}} b"
    );
    assert_eq!(
        expand_transclusions("a {{unterminated", &resolve),
        "a {{unterminated"
    );
}

#[test]
fn text_without_transclusions_passes_through_unchanged() {
    let out = expand_transclusions("plain {single} [double] text", &resolver(&[]));
    assert_eq!(out, "plain {single} [double] text");
}

// ---------------------------------------------------------------------------
// Row 68: preview prep ported from NVPreviewHTMLPreparingContent.
// ---------------------------------------------------------------------------

#[test]
fn prep_flags_math_on_class_math_needles() {
    let (html, needs) = preview_prep("<p><span class=\"math\">x</span></p>");
    assert!(needs.math, "class=\"math\" fires");
    assert!(!needs.mermaid && !needs.wiki);
    assert_eq!(html, "<p><span class=\"math\">x</span></p>");

    let (_, needs) = preview_prep("<p><span class=\"math display\">x</span></p>");
    assert!(needs.math, "class=\"math \" (trailing space) fires");
}

#[test]
fn prep_flags_wiki_on_literal_bracket_pair() {
    let (_, needs) = preview_prep("<p>see [[Some Note]] here</p>");
    assert!(needs.wiki);
    let (_, needs) = preview_prep("<p>no links</p>");
    assert!(!needs.wiki && !needs.math && !needs.mermaid);
}

#[test]
fn prep_rewrites_language_mermaid_fences() {
    let (html, needs) =
        preview_prep("<p>pre</p><pre><code class=\"language-mermaid\">graph TD</code></pre>");
    assert_eq!(
        html, "<p>pre</p><pre class=\"mermaid\">graph TD</pre>",
        "fence rewritten in place, source verbatim"
    );
    assert!(
        needs.mermaid,
        "class=\"mermaid\" detected after the rewrite"
    );
    assert!(!needs_code_highlight(&html), "mermaid blocks are not code");
}

#[test]
fn prep_rewrites_the_braced_mermaid_form() {
    let (html, needs) = preview_prep("<pre><code class=\"language-{mermaid}\">graph</code></pre>");
    assert_eq!(html, "<pre class=\"mermaid\">graph</pre>");
    assert!(needs.mermaid);
}

#[test]
fn prep_rewrites_the_lang_mermaid_form() {
    let (html, needs) = preview_prep("<pre lang=\"mermaid\"><code>graph</code></pre>");
    assert_eq!(html, "<pre class=\"mermaid\">graph</pre>");
    assert!(needs.mermaid);
}

#[test]
fn prep_rewrite_is_case_insensitive() {
    let (html, needs) = preview_prep("<PRE><CODE CLASS=\"LANGUAGE-MERMAID\">graph</CODE></PRE>");
    assert_eq!(html, "<pre class=\"mermaid\">graph</pre>");
    assert!(needs.mermaid);
}

#[test]
fn prep_leaves_non_mermaid_code_blocks_alone() {
    let src = "<pre><code class=\"language-rust\">fn main() {}</code></pre>";
    let (html, needs) = preview_prep(src);
    assert_eq!(html, src);
    assert!(!needs.mermaid);
    assert!(needs_code_highlight(&html));
}

#[test]
fn prep_flags_wiki_before_renderer_math_brackets_exist() {
    // `[[` is a literal search on the prepared HTML (upstream detects
    // wiki needs before MathJax's own `[[` brackets are injected).
    let (_, needs) = preview_prep("<p>x[[y</p>");
    assert!(needs.wiki);
}

#[test]
fn prep_marks_completed_task_items_and_prepends_task_css() {
    let src = "<ul><li><input type=\"checkbox\" checked=\"\">done</li>\
               <li><input type=\"checkbox\">open</li></ul>";
    let (html, _) = preview_prep(src);
    assert!(
        html.starts_with("<style>li input[type=\"checkbox\"]{margin-right:0.4em;vertical-align:text-bottom;}li.nv-task-done{text-decoration:line-through;}li.nv-task-done input{text-decoration:none;}</style>"),
        "the exact upstream task CSS is prepended when a checkbox exists: {html}"
    );
    assert!(
        html.contains("<li class=\"nv-task-done\"><input type=\"checkbox\" checked=\"\">done</li>"),
        "checked items gain nv-task-done: {html}"
    );
    assert!(
        html.contains("<li><input type=\"checkbox\">open</li>"),
        "unchecked items are untouched: {html}"
    );
}

#[test]
fn prep_prepends_task_css_even_without_completed_items() {
    let src = "<ul><li><input type=\"checkbox\">open</li></ul>";
    let (html, _) = preview_prep(src);
    assert!(
        html.starts_with("<style>"),
        "any checkbox triggers the CSS prepend (upstream index-0 insert): {html}"
    );
    assert!(
        !html.contains("<li class=\"nv-task-done\""),
        "no li gains the class when nothing is checked (the CSS text itself may name it): {html}"
    );
}

#[test]
fn prep_leaves_checkboxless_html_untouched() {
    let src = "<p>plain</p>";
    let (html, needs) = preview_prep(src);
    assert_eq!(html, src);
    assert!(!needs.math && !needs.mermaid && !needs.wiki);
    assert!(!needs_code_highlight(&html));
}

#[test]
fn needs_code_highlight_distinguishes_blocks_from_inline_code() {
    assert!(needs_code_highlight(
        "<p>x</p><pre><code>block</code></pre>"
    ));
    assert!(!needs_code_highlight(
        "<p>inline <code>code</code> only</p>"
    ));
    assert!(!needs_code_highlight(""));
}

#[test]
fn prep_handles_multi_occurrence_mermaid_and_surviving_text() {
    let src = "a<pre><code class=\"language-mermaid\">g1</code></pre>b\
               <pre><code class=\"language-mermaid\">g2</code></pre>c";
    let (html, needs) = preview_prep(src);
    assert_eq!(
        html,
        "a<pre class=\"mermaid\">g1</pre>b<pre class=\"mermaid\">g2</pre>c"
    );
    assert!(needs.mermaid);
}
