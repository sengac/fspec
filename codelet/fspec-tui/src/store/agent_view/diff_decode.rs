//! RPC-393 — style typed `DiffDisplayRow`s into colored ratatui spans.
//!
//! Feature: spec/features/agentview-edit-diff-structured-rows.feature
//!          spec/features/agentview-edit-diff-rendering.feature
//!          spec/features/agentview-edit-diff-padding.feature
//!
//! Replaces the RPC-391 marker-decode (`[R]`/`[A]` `line.find` + the
//! hand-rolled `context_gutter_len` byte-scanner + `strip_marker`) with a
//! single typed styling function. The formatter ([`diff_format::build_diff_rows`])
//! emits typed rows; the canonical-string codec
//! ([`diff_format::to_line`]/[`diff_format::parse_line`]) carries them through
//! the stored `ChunkSource::text` (re-wrapped on resize); the renderer parses
//! a wrapped line back to a typed row and calls [`style_row`] — ONE styling
//! function shared by the scrollback (`chunk_wrap`) and the modal
//! (`turn_modal`). No `[R]`/`[A]` ever reaches the screen.
//!
//! **Gutter consistency (RPC-393 fix A)**: the line-number gutter is ALWAYS
//! rendered dim/gray and OUTSIDE the colored bar; the red/green background
//! fills from the marker column to the right edge. This makes the gutter
//! column uniform top-to-bottom (no per-row-type flip) while preserving the
//! RPC-392 full-width bars.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::diff_codec::{pad_left, parse_line};
use super::diff_format::DiffDisplayRow;
use super::stderr::style_modal_raw_line;
use crate::views::agent::text_wrap::wrap_to_width;

/// Removed-line background `#8B0000` (TS `DIFF_COLORS.removed`).
pub const DIFF_BG_REMOVED: Color = Color::Rgb(139, 0, 0);
/// Added-line background `#006400` (TS `DIFF_COLORS.added`).
pub const DIFF_BG_ADDED: Color = Color::Rgb(0, 100, 0);

/// **RPC-393 (WARNING #1)**: the single-line styler, now derived from the ONE
/// styling core. Both `style_row` and `style_row_lines` build their colored
/// bars / gutters / elision spans through the SAME private helpers
/// ([`changed_bar_row`], [`context_row`], [`elision_row`]) so the gutter / bar
/// / elision rule lives in exactly one place.
///
/// `style_row` styles the row as a SINGLE visual line at the real render
/// `width`: a Removed/Added row is the dim gutter + ONE contiguous bar padded
/// to `width - gutter_w` (so the spans total exactly `width` when content is
/// shorter, and the content rides through unpadded — no trailing space — when
/// it is already ≥ that room). This is the wrapped-fragment entry point
/// (`style_wrapped_line` hands it ONE already-width-fit fragment), so it does
/// not re-wrap. Saturating arithmetic — `width == 0` adds no padding and never
/// panics. Display-width is the `chars().count()` proxy shared with
/// `wrap_to_width` (DRY).
pub fn style_row(row: &DiffDisplayRow, width: usize) -> Vec<Span<'static>> {
    match row {
        DiffDisplayRow::Removed { line_no, text } => {
            changed_bar_row(*line_no, '-', text, DIFF_BG_REMOVED, width)
        }
        DiffDisplayRow::Added { line_no, text } => {
            changed_bar_row(*line_no, '+', text, DIFF_BG_ADDED, width)
        }
        DiffDisplayRow::Context { line_no, text } => context_row(*line_no, text),
        DiffDisplayRow::Elision { text } => elision_row(text),
    }
}

/// **RPC-393**: style a single WRAPPED diff body line (a fragment produced by
/// `wrap_to_width` over a canonical [`to_line`] string). The first fragment of
/// a row parses to its typed kind via [`parse_line`]; continuation fragments
/// (which lost their gutter prefix on wrap) fall through to [`parse_line`]'s
/// Elision branch and render as plain dim text — never as a leaked marker.
/// `width` is the render width used to pad changed bars full-width.
pub fn style_wrapped_line(line: &str, width: usize) -> Vec<Span<'static>> {
    style_row(&parse_line(line), width)
}

/// **RPC-393 (WARNING #4)**: style one wrapped MODAL hard line, returning ONE
/// styled `Vec<Span>` per visual row. `is_diff` gates ALL diff styling: when
/// the turn is NOT a diff card the line is returned as a single raw span so a
/// plain body line that merely LOOKS line-numbered (e.g. `"42   indented log"`)
/// is never diff-styled. When it IS a diff card the line is parsed ONCE and
/// wrapped continuation-safe via [`style_row_lines`] (CRITICAL #3) — no
/// per-fragment re-parse, no phantom rows.
pub fn style_modal_lines(line: &str, width: usize, is_diff: bool) -> Vec<Vec<Span<'static>>> {
    if !is_diff {
        return style_modal_raw_line(line, |l| wrap_to_width(l, width.max(1)));
    }
    let parsed = parse_line(line);
    if let DiffDisplayRow::Elision { text } = &parsed {
        // Plain elision text (sentinel already stripped by parse_line): keep
        // it as raw modal text so non-diff-looking gaps stay verbatim.
        let mut frags = wrap_to_width(text, width.max(1));
        if frags.is_empty() {
            frags.push(String::new());
        }
        return frags.into_iter().map(|f| vec![Span::raw(f)]).collect();
    }
    style_row_lines(&parsed, width)
}

/// **RPC-393 (CRITICAL #3)**: style a typed [`DiffDisplayRow`] across a
/// width-based wrap, returning ONE styled `Vec<Span>` per visual row. The
/// gutter/marker/bar styling is applied only to the FIRST visual row; every
/// continuation fragment is plain content of the SAME row (same background for
/// changed bars, NO gutter, NO marker glyph) and is NEVER re-parsed as a fresh
/// row — so a content fragment shaped like `"999 [A]+ …"` can never resurrect a
/// phantom colored/context row on resize. Saturating at `width == 0`.
pub fn style_row_lines(row: &DiffDisplayRow, width: usize) -> Vec<Vec<Span<'static>>> {
    match row {
        DiffDisplayRow::Removed { line_no, text } => {
            changed_lines(*line_no, '-', text, DIFF_BG_REMOVED, width)
        }
        DiffDisplayRow::Added { line_no, text } => {
            changed_lines(*line_no, '+', text, DIFF_BG_ADDED, width)
        }
        DiffDisplayRow::Context { line_no, text } => context_lines(*line_no, text, width),
        DiffDisplayRow::Elision { text } => elision_lines(text, width),
    }
}

/// Build the spans for ONE Removed/Added visual row: dim gray gutter (line
/// number + space) OUTSIDE the bar, then a single colored bar span of
/// `"{glyph} {text}"` padded to fill `width - gutter_w` on `bg` (white fg).
/// Already-wide content rides through unpadded (no trailing space). Saturating
/// — `width <= gutter_w` (incl. `width == 0`) pads nothing. This is the ONE
/// place a changed bar is built; both `style_row` and `changed_lines` use it.
fn changed_bar_row(
    line_no: usize,
    glyph: char,
    text: &str,
    bg: Color,
    width: usize,
) -> Vec<Span<'static>> {
    let gutter = format!("{} ", gutter_num(line_no));
    let gutter_w = gutter.chars().count();
    let bar = pad_to_width(format!("{glyph} {text}"), width.saturating_sub(gutter_w));
    vec![
        Span::styled(gutter, gutter_style()),
        Span::styled(bar, Style::default().bg(bg).fg(Color::White)),
    ]
}

/// Build the spans for ONE Context visual row: dim gray gutter (line number +
/// three separating spaces) + white content, NO background, NOT padded. The
/// ONE place a context row is built.
fn context_row(line_no: usize, text: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!("{}   ", gutter_num(line_no)), gutter_style()),
        Span::styled(text.to_string(), Style::default().fg(Color::White)),
    ]
}

/// Build the span for ONE Elision visual row: a single dim span carrying the
/// already-indented text. Gap markers and collapse hints share this helper.
fn elision_row(text: &str) -> Vec<Span<'static>> {
    vec![Span::styled(
        text.to_string(),
        Style::default().add_modifier(Modifier::DIM),
    )]
}

/// Wrap a Removed/Added row's content into one contiguous colored bar across a
/// width-based wrap. The first visual row is built by [`changed_bar_row`] (dim
/// gutter OUTSIDE the bar). Continuation rows are bar-bg content padded full
/// width with NO gutter and NO marker — keeping the bar contiguous and never
/// re-parsing a fragment as a fresh row.
fn changed_lines(
    line_no: usize,
    glyph: char,
    text: &str,
    bg: Color,
    width: usize,
) -> Vec<Vec<Span<'static>>> {
    let gutter_w = format!("{} ", gutter_num(line_no)).chars().count();
    let content = format!("{glyph} {text}");
    // The bar occupies the columns after the gutter. When the viewport is too
    // narrow to hold even the gutter (`width <= gutter_w`, incl. `width == 0`)
    // there is no room to wrap, so the content rides through unsliced on a
    // single row — keeping the zero-width path panic-free with content intact.
    let bar_room = width.saturating_sub(gutter_w);
    if bar_room == 0 {
        return vec![changed_bar_row(line_no, glyph, text, bg, width)];
    }
    let bar_style = Style::default().bg(bg).fg(Color::White);
    let mut frags = wrap_to_width(&content, bar_room);
    if frags.is_empty() {
        frags.push(String::new());
    }
    let gutter = format!("{} ", gutter_num(line_no));
    let indent = " ".repeat(gutter_w);
    frags
        .into_iter()
        .enumerate()
        .map(|(i, frag)| {
            // First row: dim gutter OUTSIDE the bar. Continuation rows: blank
            // bar-bg indent (NO gutter, NO marker) so the colored column stays
            // contiguous and a fragment is never re-parsed as a fresh row.
            let lead = if i == 0 {
                Span::styled(gutter.clone(), gutter_style())
            } else {
                Span::styled(indent.clone(), bar_style)
            };
            vec![lead, Span::styled(pad_to_width(frag, bar_room), bar_style)]
        })
        .collect()
}

/// Wrap a Context row's content; the first visual row is built by
/// [`context_row`]'s gutter rule, continuation rows are plain white content
/// (no background, no gutter).
fn context_lines(line_no: usize, text: &str, width: usize) -> Vec<Vec<Span<'static>>> {
    let gutter = format!("{}   ", gutter_num(line_no));
    let gutter_w = gutter.chars().count();
    let content_width = width.saturating_sub(gutter_w).max(1);
    let mut frags = wrap_to_width(text, content_width);
    if frags.is_empty() {
        frags.push(String::new());
    }
    let white = Style::default().fg(Color::White);
    let mut out: Vec<Vec<Span<'static>>> = Vec::with_capacity(frags.len());
    for (i, frag) in frags.into_iter().enumerate() {
        if i == 0 {
            out.push(vec![
                Span::styled(gutter.clone(), gutter_style()),
                Span::styled(frag, white),
            ]);
        } else {
            out.push(vec![Span::styled(frag, white)]);
        }
    }
    out
}

/// Wrap an Elision row's (sentinel-stripped) text into dim continuation rows.
///
/// The leading indent (gutter-width spaces decided by `elision_indent`) is
/// preserved on every visual row: `wrap_to_width` word-wraps over-width
/// paragraphs via `split_whitespace`, which would otherwise drop the leading
/// spaces. We split the indent off, wrap only the trailing content against the
/// reduced width, then re-apply the indent to each fragment via [`elision_row`]
/// so a gap marker and a collapse hint keep ONE uniform indentation whether or
/// not they wrap.
fn elision_lines(text: &str, width: usize) -> Vec<Vec<Span<'static>>> {
    let indent_len = text.chars().take_while(|c| *c == ' ').count();
    let indent: String = " ".repeat(indent_len);
    let content: String = text.chars().skip(indent_len).collect();
    let content_width = width.saturating_sub(indent_len).max(1);
    let mut frags = wrap_to_width(&content, content_width);
    if frags.is_empty() {
        frags.push(String::new());
    }
    frags
        .into_iter()
        .map(|f| elision_row(&format!("{indent}{f}")))
        .collect()
}

/// The uniform gutter style: dim/gray, no background. Applied identically to
/// every row type so the line-number column never flips styling vertically.
fn gutter_style() -> Style {
    Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
}

/// Right-pad `text` with spaces to `width` DISPLAY columns (the
/// `chars().count()` proxy shared with `wrap_to_width`). Already-wide content
/// is returned unchanged; saturating — `width == 0` adds nothing.
fn pad_to_width(text: String, width: usize) -> String {
    let pad = width.saturating_sub(text.chars().count());
    if pad == 0 {
        return text;
    }
    let mut out = text;
    out.extend(std::iter::repeat_n(' ', pad));
    out
}

/// Left-pad a line number to at least the minimum gutter width with spaces,
/// matching the canonical codec column layout. Delegates to the codec's
/// [`pad_left`] (DRY — one padding rule).
fn gutter_num(value: usize) -> String {
    pad_left(value)
}

#[cfg(test)]
mod tests {
    //! Feature: spec/features/agentview-edit-diff-structured-rows.feature
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn wrapped_line_with_no_width_does_not_panic() {
        let spans = style_wrapped_line("  2 [R]- line2", 0);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("line2"));
        assert!(!text.contains("[R]"));
    }
}
