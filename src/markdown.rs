//! Safe HTML rendering of the AI summary's Markdown, via
//! `pulldown-cmark`.
//!
//! The summary is model output, so it is untrusted: feeding it to the
//! DOM as raw HTML would let a hallucinated (or prompt-injected)
//! `<script>` or `<img onerror>` run in the page, next to the stored
//! `OpenRouter` key. Raw-HTML events from the source are therefore
//! demoted to plain text (escaped on output), and link/image targets
//! are rewritten to `#` unless they are web or mail links.

use pulldown_cmark::{Event, Options, Parser, Tag, html};

fn is_safe_url(url: &str) -> bool {
    let url = url.trim();
    url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("mailto:")
        || url.starts_with('#')
        || url.starts_with('/')
}

/// Render Markdown to sanitised HTML for `inner_html`.
#[must_use]
pub fn render_markdown(source: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let events = Parser::new_ext(source, options).map(|event| match event {
        Event::Html(html) | Event::InlineHtml(html) => Event::Text(html),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) if !is_safe_url(&dest_url) => Event::Start(Tag::Link {
            link_type,
            dest_url: "#".into(),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) if !is_safe_url(&dest_url) => Event::Start(Tag::Image {
            link_type,
            dest_url: "#".into(),
            title,
            id,
        }),
        other => other,
    });
    let mut out = String::with_capacity(source.len());
    html::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_structure() {
        let html = render_markdown(
            "## Verdict\n\nPro wins on **turns** and *speed*.\n\n- first\n- second\n\n1. one\n2. two",
        );
        assert!(html.contains("<h2>Verdict</h2>"), "{html}");
        assert!(html.contains("<strong>turns</strong>"), "{html}");
        assert!(html.contains("<em>speed</em>"), "{html}");
        assert!(html.contains("<li>first</li>"), "{html}");
        assert!(html.contains("<ol>"), "{html}");
    }

    #[test]
    fn raw_html_is_escaped_not_emitted() {
        let html = render_markdown("hi <script>alert(1)</script> <img src=x onerror=alert(1)>");
        assert!(!html.contains("<script>"), "{html}");
        assert!(!html.contains("<img"), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
    }

    #[test]
    fn dangerous_link_targets_are_rewritten() {
        let html = render_markdown("[ok](https://example.com) [no](javascript:alert(1))");
        assert!(html.contains("href=\"https://example.com\""), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
        assert!(html.contains("href=\"#\""), "{html}");
    }
}
