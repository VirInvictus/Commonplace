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

    /// Row 70: HTML export is well-formed through the real renderer
    /// (the core suite asserts the same over the stub). A dependency-
    /// free tag-balance walk over the exported document.
    #[test]
    fn comrak_export_document_is_well_formed() {
        use commonplace_core::export::{ExportFormat, export_note_to_dir};
        use commonplace_core::store::load_folder;

        let src = std::env::temp_dir().join(format!(
            "commonplace-cli-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("Note.md"),
            "Note\n\n# head\n\n- [x] done\n\n<script>x</script>\n",
        )
        .unwrap();
        let outcome = load_folder(&src).unwrap();
        let out = src.parent().unwrap().join("cli-export-out");
        let written = export_note_to_dir(
            &outcome.store,
            &outcome.store.notes()[0].clone(),
            &out,
            ExportFormat::Html,
            &ComrakRenderer,
        )
        .unwrap();
        let html = std::fs::read_to_string(&written).unwrap();

        assert!(html.contains("<title>Note</title>"), "{html}");
        // Safe by default: the raw script never ships in an export.
        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("type=\"checkbox\""), "{html}");

        const VOID: [&str; 9] = [
            "br", "hr", "img", "meta", "link", "input", "col", "area", "source",
        ];
        let mut stack: Vec<String> = Vec::new();
        let bytes = html.as_bytes();
        let mut scan = 0usize;
        while let Some(lt_rel) = bytes[scan..].iter().position(|&b| b == b'<') {
            let lt = scan + lt_rel;
            let Some(gt_rel) = bytes[lt..].iter().position(|&b| b == b'>') else {
                panic!("unterminated tag: {html}");
            };
            let gt = lt + gt_rel;
            let body = &html[lt + 1..gt];
            scan = gt + 1;
            if body.starts_with('!') || body.ends_with('/') {
                continue;
            }
            if let Some(name) = body.strip_prefix('/') {
                assert_eq!(
                    stack.pop().unwrap().to_ascii_lowercase(),
                    name.trim().to_ascii_lowercase(),
                    "mismatched close: {html}"
                );
            } else {
                let name: String = body
                    .split(|c: char| c.is_ascii_whitespace() || c == '>')
                    .next()
                    .unwrap_or("")
                    .to_owned();
                if !name.is_empty() && !VOID.contains(&name.to_ascii_lowercase().as_str()) {
                    stack.push(name);
                }
            }
        }
        assert!(stack.is_empty(), "unclosed tags {stack:?}: {html}");
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&out);
    }
}
