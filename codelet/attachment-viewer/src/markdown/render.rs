//! Markdown → HTML rendering — port of `src/server/utils/markdown-renderer.ts`.
//!
//! Uses `pulldown-cmark` for the bulk of rendering with the GFM extensions that
//! `marked` (the TS viewer) supports: tables, strikethrough, and task lists.
//! Footnotes are intentionally NOT enabled, since `marked` has no footnote
//! support and never emits footnote markup. This renderer also intercepts:
//!   - fenced code blocks → mermaid / code-block wrappers (see [`render_code_block`])
//!   - headings → emit `<hN id="slug">…</hN>` with GitHub-style anchor ids
//!   - soft line breaks → hard `<br>` (parity with marked `breaks: true`)
//!
//! All code content is HTML-escaped via [`html_escape`].

use std::collections::HashMap;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::escape::html_escape;
use super::slug::slugify;

/// Render GFM markdown to an HTML fragment with mermaid/code-block/anchor handling.
pub fn render_markdown(markdown: &str) -> String {
    // Note: ENABLE_SMART_PUNCTUATION is intentionally NOT set — the TS `marked`
    // renderer does not smart-quote, and code/identifiers must stay literal.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    // Note: ENABLE_FOOTNOTES is intentionally NOT set — marked (the TS viewer)
    // has no footnote support, so `[^1]` must not emit footnote-specific markup.

    let parser = Parser::new_ext(markdown, options);

    let mut html = String::new();
    let mut passthrough: Vec<Event> = Vec::new();
    let mut code_lang: Option<String> = None;
    let mut code_buf = String::new();
    let mut heading: Option<HeadingState> = None;
    let mut seen: HashMap<String, u32> = HashMap::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_passthrough(&mut passthrough, &mut html);
                code_lang = Some(match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                });
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                let lang = code_lang.take().unwrap_or_default();
                html.push_str(&render_code_block(&lang, &code_buf));
                code_buf.clear();
            }
            Event::Text(text) | Event::Code(text) if code_lang.is_some() => {
                code_buf.push_str(&text);
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush_passthrough(&mut passthrough, &mut html);
                heading = Some(HeadingState::new(level));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(state) = heading.take() {
                    html.push_str(&state.render(&mut seen));
                }
            }
            ref ev if heading.is_some() => {
                if let Some(state) = heading.as_mut() {
                    state.push(ev);
                }
            }
            Event::SoftBreak => {
                // Emulate marked `breaks: true`: a single newline becomes a <br>.
                flush_passthrough(&mut passthrough, &mut html);
                html.push_str("<br>\n");
            }
            other => passthrough.push(other),
        }
    }
    flush_passthrough(&mut passthrough, &mut html);
    html
}

/// Accumulated state for one heading: its level, inner events, and plain text.
struct HeadingState {
    level: HeadingLevel,
    events: Vec<Event<'static>>,
    text: String,
}

impl HeadingState {
    fn new(level: HeadingLevel) -> Self {
        Self {
            level,
            events: Vec::new(),
            text: String::new(),
        }
    }

    /// Buffer one inner event and accumulate its plain-text for the slug.
    fn push(&mut self, event: &Event) {
        match event {
            Event::Text(t) | Event::Code(t) => self.text.push_str(t),
            Event::SoftBreak | Event::HardBreak => self.text.push(' '),
            _ => {}
        }
        self.events.push(event.clone().into_static());
    }

    /// Render `<hN id="slug">inner</hN>` for this heading.
    fn render(self, seen: &mut HashMap<String, u32>) -> String {
        let slug = slugify(&self.text, seen);
        let mut inner = String::new();
        pulldown_cmark::html::push_html(&mut inner, self.events.into_iter());
        let level = self.level as u8;
        format!("<h{level} id=\"{slug}\">{inner}</h{level}>\n")
    }
}

/// Render the accumulated non-code-block events to HTML and clear the buffer.
fn flush_passthrough(buffer: &mut Vec<Event>, out: &mut String) {
    if buffer.is_empty() {
        return;
    }
    let events = std::mem::take(buffer);
    let events = super::autolink::autolink_events(events);
    pulldown_cmark::html::push_html(out, events.into_iter());
}

/// Emit the mermaid or generic code-block HTML for one fenced block.
fn render_code_block(lang: &str, code: &str) -> String {
    let trimmed = code.strip_suffix('\n').unwrap_or(code);
    if lang == "mermaid" {
        return format!(
            "<pre class=\"mermaid\">\n{}\n</pre>\n",
            html_escape(trimmed)
        );
    }
    let language = if lang.is_empty() { "text" } else { lang };
    format!(
        "<pre class=\"code-block\" data-language=\"{}\"><code>{}</code></pre>\n",
        html_escape(language),
        html_escape(trimmed)
    )
}
