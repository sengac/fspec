//! Autolink-literal transform — parity with marked's GFM autolink output.
//!
//! pulldown-cmark has no autolink-literal support, so this module post-processes
//! the passthrough `Event` buffer (see [`super::render`]) and rewrites eligible
//! `Event::Text` nodes into `Text + Start(Link) + Text + End(Link)` sequences for
//! bare `http`/`https` URLs and bare emails (`mailto:` prefix).
//!
//! Scope decisions:
//!   - Only `Event::Text` is transformed. Inline code is `Event::Code` (skipped).
//!   - Text inside an existing `Tag::Link` is never autolinked (no nested anchors).
//!   - Headings render via a separate `push_html` path and are intentionally left
//!     plain.
//!   - Schemes are narrowed to `http://` / `https://` plus bare emails; `www.` and
//!     other schemes are out of scope for this card.
//!   - Email detection is deliberately pragmatic, not RFC-complete: the local part
//!     accepts `[A-Za-z0-9._%+-]`, the domain accepts `[A-Za-z0-9.-]`, and the only
//!     TLD check is "the domain contains a dot". No real TLD/MX validation is done.
//!
//! Scanning is UTF-8 safe: candidate positions come from [`str::char_indices`] and
//! every slice is taken at a char boundary, so multibyte text (accents, em-dashes,
//! emoji, CJK) before or around a URL never causes a panic.

use pulldown_cmark::{CowStr, Event, LinkType, Tag, TagEnd};

/// Rewrite bare URLs/emails in `Event::Text` nodes into link event sequences.
///
/// Text appearing inside an existing link (tracked via `Tag::Link` nesting depth)
/// is left untouched, as are all non-`Text` events.
pub fn autolink_events(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out: Vec<Event> = Vec::with_capacity(events.len());
    let mut link_depth: u32 = 0;
    for event in events {
        match event {
            Event::Start(Tag::Link { .. }) => {
                link_depth += 1;
                out.push(event);
            }
            Event::End(TagEnd::Link) => {
                link_depth = link_depth.saturating_sub(1);
                out.push(event);
            }
            Event::Text(text) if link_depth == 0 => split_text(&text, &mut out),
            other => out.push(other),
        }
    }
    out
}

/// Split one text node around detected URLs/emails, pushing the resulting events.
///
/// Candidate start positions are char boundaries from [`str::char_indices`], so the
/// scan is UTF-8 safe regardless of multibyte content.
fn split_text<'a>(text: &str, out: &mut Vec<Event<'a>>) {
    let mut cursor = 0usize;
    for (idx, _ch) in text.char_indices() {
        if idx < cursor {
            continue;
        }
        if let Some((start, end, dest)) = token_at(text, idx) {
            if start < cursor {
                continue;
            }
            if start > cursor {
                out.push(text_event(&text[cursor..start]));
            }
            out.push(Event::Start(link_tag(dest)));
            out.push(text_event(&text[start..end]));
            out.push(Event::End(TagEnd::Link));
            cursor = end;
        }
    }
    if cursor < text.len() {
        out.push(text_event(&text[cursor..]));
    }
}

/// Owned text `Event` (the borrow of `text` does not outlive this call).
fn text_event<'a>(s: &str) -> Event<'a> {
    Event::Text(CowStr::Boxed(s.to_string().into_boxed_str()))
}

/// Build an inline `Tag::Link` with the given destination URL and empty metadata.
fn link_tag<'a>(dest: String) -> Tag<'a> {
    Tag::Link {
        link_type: LinkType::Inline,
        dest_url: CowStr::Boxed(dest.into_boxed_str()),
        title: CowStr::Borrowed(""),
        id: CowStr::Borrowed(""),
    }
}

/// Detect a URL or email token anchored at char-boundary byte index `idx`.
///
/// Returns `(start, end, dest_url)` where `[start, end)` is the visible span in
/// `text` and `dest_url` is the href (`mailto:` prefixed for emails). For URLs the
/// token starts at `idx`; for emails `idx` is the `@` and the local part precedes it.
fn token_at(text: &str, idx: usize) -> Option<(usize, usize, String)> {
    if let Some(end) = match_scheme(text, idx) {
        let trimmed = trim_url_end(text, idx, end);
        if trimmed > idx {
            return Some((idx, trimmed, text[idx..trimmed].to_string()));
        }
    }
    if let Some((start, end)) = match_email(text, idx) {
        return Some((start, end, format!("mailto:{}", &text[start..end])));
    }
    None
}

/// If `text` at char-boundary `i` starts with `http://`/`https://`, return the end
/// index of the raw URL run, else `None`.
///
/// The run terminates on ANY Unicode whitespace (not just ASCII), so a non-breaking
/// space after a URL is not pulled into the href.
fn match_scheme(text: &str, i: usize) -> Option<usize> {
    let rest = text.get(i..)?;
    let scheme_len = if rest.starts_with("https://") {
        8
    } else if rest.starts_with("http://") {
        7
    } else {
        return None;
    };
    let after = &rest[scheme_len..];
    let mut run_len = 0usize;
    for (off, ch) in after.char_indices() {
        if ch.is_whitespace() {
            break;
        }
        run_len = off + ch.len_utf8();
    }
    // Require at least one host char after the scheme.
    if run_len > 0 {
        Some(i + scheme_len + run_len)
    } else {
        None
    }
}

/// Apply GFM trailing-punctuation trimming to a URL run `[start, end)`.
fn trim_url_end(text: &str, start: usize, mut end: usize) -> usize {
    let bytes = text.as_bytes();
    loop {
        if end <= start {
            break;
        }
        let last = bytes[end - 1];
        if matches!(
            last,
            b'<' | b'>' | b'?' | b'!' | b'.' | b',' | b':' | b';' | b'*' | b'_' | b'~'
        ) {
            end -= 1;
            continue;
        }
        if last == b')' && !parens_balanced(&text[start..end]) {
            end -= 1;
            continue;
        }
        break;
    }
    end
}

/// Whether the run has balanced `(`/`)` (closing never exceeds opening).
fn parens_balanced(run: &str) -> bool {
    let mut depth: i32 = 0;
    for ch in run.bytes() {
        match ch {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// If a bare email (`local@domain.tld`) ends at byte index `i`, return its span.
///
/// `i` must point at the byte just past the end of the local part, i.e. the `@`.
/// To keep scanning simple we instead detect an email whose `@` is at `i`.
fn match_email(text: &str, i: usize) -> Option<(usize, usize)> {
    if text.as_bytes().get(i) != Some(&b'@') {
        return None;
    }
    let start = local_start(text, i)?;
    let (end, has_dot) = domain_end(text, i + 1)?;
    if has_dot && end > i + 1 {
        Some((start, end))
    } else {
        None
    }
}

/// Walk backwards from the `@` to the start of the local part.
fn local_start(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut s = at;
    while s > 0 && is_local_char(bytes[s - 1]) {
        s -= 1;
    }
    if s < at {
        Some(s)
    } else {
        None
    }
}

/// Walk forward over the domain, reporting the end index and whether a dot exists.
fn domain_end(text: &str, from: usize) -> Option<(usize, bool)> {
    let bytes = text.as_bytes();
    let mut e = from;
    let mut has_dot = false;
    while e < bytes.len() && is_domain_char(bytes[e]) {
        if bytes[e] == b'.' {
            has_dot = true;
        }
        e += 1;
    }
    // Trim a trailing dot out of the domain (not part of the address).
    while e > from && bytes[e - 1] == b'.' {
        e -= 1;
    }
    if e > from {
        Some((e, has_dot))
    } else {
        None
    }
}

/// Local-part character class (a pragmatic subset of RFC 5322).
fn is_local_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-')
}

/// Domain character class (letters, digits, dot, hyphen).
fn is_domain_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-')
}
