//! RPC-405 — pure wrap geometry for the AgentView MultiLineInput.
//!
//! Feature: spec/features/agent-input-soft-wrap-auto-grow.feature
//!
//! Segments logical lines into visual rows by unicode display width
//! (never splitting a wide char; empty lines = one visual row), maps
//! the logical cursor to a (visual row, display column) position, and
//! ports the tui-textarea `next_scroll_top` cursor-follow algorithm
//! into visual-row space. No terminal types here — everything is
//! unit-testable without a Buffer.

use unicode_width::UnicodeWidthChar;

/// One wrapped segment of a logical line: char offset of the segment
/// start within the line + the segment text (display width <=
/// wrap_width).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub char_start: usize,
    pub text: String,
}

/// One visual row across the whole buffer: which logical line it came
/// from + the segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualRow {
    pub logical_row: usize,
    pub char_start: usize,
    pub text: String,
}

/// Split `line` into display-width segments of at most `wrap_width`
/// columns, breaking BEFORE any char that would exceed the width
/// (wide chars are never split). An empty line yields exactly one
/// empty segment. `wrap_width == 0` is degenerate: one empty segment
/// (guard — the renderer paints nothing at zero width).
pub fn segment_line(line: &str, wrap_width: u16) -> Vec<Segment> {
    if wrap_width == 0 || line.is_empty() {
        return vec![Segment {
            char_start: 0,
            text: String::new(),
        }];
    }
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut current_width = 0usize;
    let wrap = wrap_width as usize;
    for (i, c) in line.chars().enumerate() {
        let w = c.width().unwrap_or(0);
        if current_width + w > wrap && !current.is_empty() {
            segments.push(Segment {
                char_start: current_start,
                text: std::mem::take(&mut current),
            });
            current_start = i;
            current_width = 0;
        }
        current.push(c);
        current_width += w;
    }
    segments.push(Segment {
        char_start: current_start,
        text: current,
    });
    segments
}

/// Wrap every logical line into visual rows.
pub fn wrap_lines(lines: &[String], wrap_width: u16) -> Vec<VisualRow> {
    let mut rows = Vec::new();
    for (logical_row, line) in lines.iter().enumerate() {
        for seg in segment_line(line, wrap_width) {
            rows.push(VisualRow {
                logical_row,
                char_start: seg.char_start,
                text: seg.text,
            });
        }
    }
    rows
}

/// Total number of visual rows the buffer occupies at `wrap_width`.
pub fn total_visual_rows(lines: &[String], wrap_width: u16) -> usize {
    lines
        .iter()
        .map(|l| segment_line(l, wrap_width).len())
        .sum()
}

/// Map the logical cursor (row, col in chars) to (visual row index
/// across the whole buffer, display column within that visual row).
///
/// A cursor sitting exactly at a wrap boundary (display column an
/// exact multiple of `wrap_width`, not at char 0) lands at column 0
/// of the NEXT visual row — matching where typing continues.
pub fn cursor_visual_position(
    lines: &[String],
    cursor: (usize, usize),
    wrap_width: u16,
) -> (usize, u16) {
    let (crow, ccol) = cursor;
    let mut vrow = 0usize;
    for line in lines.iter().take(crow) {
        vrow += segment_line(line, wrap_width).len();
    }
    let line = match lines.get(crow) {
        Some(l) => l.as_str(),
        None => return (vrow, 0),
    };
    if wrap_width == 0 {
        return (vrow, 0);
    }
    // Display column of the cursor within its logical line.
    let cursor_disp: usize = line
        .chars()
        .take(ccol)
        .map(|c| c.width().unwrap_or(0))
        .sum();
    let wrap = wrap_width as usize;
    // Walk the segments accumulating display width; the cursor lives
    // in the first segment whose [start, start+width) range contains
    // it. At an exact boundary it belongs to the NEXT segment (col 0).
    let segments = segment_line(line, wrap_width);
    let mut seg_start_disp = 0usize;
    for (i, seg) in segments.iter().enumerate() {
        let seg_width: usize = seg.text.chars().map(|c| c.width().unwrap_or(0)).sum();
        let is_last = i + 1 == segments.len();
        if cursor_disp < seg_start_disp + seg_width
            || (is_last && cursor_disp < seg_start_disp + wrap)
        {
            return (vrow + i, (cursor_disp - seg_start_disp) as u16);
        }
        seg_start_disp += seg_width;
    }
    // Cursor past the last segment: end-of-line. If the line width is
    // an exact multiple of wrap_width (and non-empty), the cursor sits
    // on a NEW visual row below; otherwise at the end of the last one.
    let last_index = segments.len().saturating_sub(1);
    let last_width: usize = segments
        .last()
        .map(|s| s.text.chars().map(|c| c.width().unwrap_or(0)).sum())
        .unwrap_or(0);
    if !line.is_empty() && last_width >= wrap {
        (vrow + last_index + 1, 0)
    } else {
        (vrow + last_index, last_width as u16)
    }
}

/// tui-textarea's cursor-follow algorithm (widget.rs) transplanted
/// into visual-row space: keep `cursor_row` inside the
/// `[top, top + height)` window.
pub fn next_scroll_top(prev_top: usize, cursor_row: usize, height: usize) -> usize {
    if height == 0 {
        return prev_top;
    }
    if cursor_row < prev_top {
        cursor_row
    } else if prev_top + height <= cursor_row {
        cursor_row + 1 - height
    } else {
        prev_top
    }
}

/// Clamp `top` so the window never scrolls past the content
/// (deleting text scrolls back up).
pub fn clamp_scroll_top(top: usize, total_rows: usize, height: usize) -> usize {
    top.min(total_rows.saturating_sub(height))
}
