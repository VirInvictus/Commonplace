//! The comrak wiring behind core's `render::Renderer` boundary (spec
//! row 66): the one sanctioned frontend dependency, pinned `=1.0.0`
//! per crates §1's risk note. The app crate joins with the preview
//! phase (Phase 15), where its own option set (unsafe + sanitize) is
//! decided; this adapter pins the export/preview option surface.

use commonplace_core::render::Renderer;
use comrak::{Options, markdown_to_html};

/// comrak behind the engine's `Renderer` trait. Options (spec row 66):
/// GFM plus footnotes and math dollars on; wikilinks deliberately
/// un-parsed so `[[X]]` survives as literal text and the needs-wiki
/// detection fires (upstream Apex parity); rendering stays safe by
/// default, raw HTML scrubbed.
pub struct ComrakRenderer;

impl ComrakRenderer {
    fn options() -> Options<'static> {
        let mut options = Options::default();
        let ext = &mut options.extension;
        ext.table = true;
        ext.strikethrough = true;
        ext.tasklist = true;
        ext.autolink = true;
        ext.footnotes = true;
        ext.math_dollars = true;
        options
    }
}

impl Renderer for ComrakRenderer {
    fn render_markdown(&self, markdown: &str) -> String {
        normalize_math_style(&markdown_to_html(markdown, &Self::options()))
    }
}

/// comrak 1.0 marks math with `data-math-style="inline|display"` on
/// span/div/pre/code; upstream Apex emitted `class="math"` spans, and
/// row 68's detection needles are those exact bytes. The adapter
/// normalizes the attribute once, at the boundary, so the detection
/// pass never grows comrak-specific needles.
fn normalize_math_style(html: &str) -> String {
    html.replace("data-math-style=\"inline\"", "class=\"math\"")
        .replace("data-math-style=\"display\"", "class=\"math display\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonplace_core::render::preview::{needs_code_highlight, preview_prep};

    #[test]
    fn headings_and_gfm_render() {
        let html = ComrakRenderer.render_markdown("# Title\n\n| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(html.contains("<table>"), "{html}");
    }

    #[test]
    fn wikilinks_stay_literal_text_so_detection_fires() {
        let (html, needs) = preview_prep(&ComrakRenderer.render_markdown("see [[Some Note]]"));
        assert!(
            html.contains("[[Some Note]]"),
            "wikilink not consumed: {html}"
        );
        assert!(needs.wiki, "{html}");
    }

    #[test]
    fn math_dollars_emit_detectable_spans() {
        let (html, needs) = preview_prep(&ComrakRenderer.render_markdown("inline $x^2$ here"));
        assert!(needs.math, "{html}");
        assert!(html.contains("class=\"math"), "{html}");
    }

    #[test]
    fn mermaid_fences_rewrite_into_diagram_blocks() {
        let (html, needs) =
            preview_prep(&ComrakRenderer.render_markdown("```mermaid\ngraph TD\nA-->B\n```\n"));
        assert!(
            html.contains("<pre class=\"mermaid\">graph TD\nA--&gt;B\n</pre>"),
            "{html}"
        );
        assert!(needs.mermaid, "{html}");
        assert!(!needs_code_highlight(&html), "{html}");
    }

    #[test]
    fn task_lists_emit_markable_checkboxes() {
        let (html, _) = preview_prep(&ComrakRenderer.render_markdown("- [x] done\n- [ ] open\n"));
        assert!(
            html.contains("<li class=\"nv-task-done\"><input"),
            "done items marked: {html}"
        );
        assert!(html.contains("type=\"checkbox\""), "{html}");
    }

    #[test]
    fn raw_html_is_scrubbed_by_default() {
        let html = ComrakRenderer.render_markdown("<script>alert(1)</script>");
        assert!(!html.contains("<script>"), "raw HTML escaped: {html}");
    }

    #[test]
    fn code_blocks_flag_highlighting() {
        let (html, _) =
            preview_prep(&ComrakRenderer.render_markdown("```rust\nfn main() {}\n```\n"));
        assert!(needs_code_highlight(&html), "{html}");
    }
}
