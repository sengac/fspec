//! RPC-393 — the SINGLE private codec for diff display rows.
//!
//! Feature: spec/features/agentview-edit-diff-structured-rows.feature
//!
//! Option 1 (chosen): `ChunkSource::text` / `full_text` stay `String` and are
//! re-wrapped on resize. This module is the SOLE encode/decode pair carrying a
//! typed [`DiffDisplayRow`] through that string: [`to_line`] is the only writer
//! and [`parse_line`] its exact inverse (round-trip property-tested). This
//! replaces the old `[R]`/`[A]` steganography + the `context_gutter_len`
//! byte-scanner + `strip_marker` — two independently-evolving string
//! heuristics — with one deliberate, tested codec.
//!
//! The canonical line shape is deliberately the legacy
//! `{num} [R]- {text}` / `{num} [A]+ {text}` / `{num}   {text}` so the RPC-390
//! golden display string is byte-for-byte preserved; `style_row` strips the
//! marker so nothing reaches the screen.

use super::diff_format::GUTTER_MIN_WIDTH;

/// Private sentinel prefixed to an [`DiffDisplayRow::Elision`]'s canonical
/// line so the parser can recover it UNAMBIGUOUSLY even when its text is
/// shaped like a context/marker row (e.g. `"42   trailing"` or
/// `"  7 [R]- x"`). It is a single C0 control char that never occurs in real
/// diff content and is stripped by both [`parse_line`] and the styler before
/// anything reaches the screen — so the codec is an exact inverse for EVERY
/// variant (Rule #10) with no visible artefact.
pub(super) const ELISION_SENTINEL: char = '\u{1}';

/// A fully-resolved diff DISPLAY row. [`super::diff_format::build_diff_rows`]
/// emits these; the renderer styles them directly via the single
/// [`super::diff_decode::style_row`] — NO `[R]`/`[A]` steganography, NO
/// re-parsing heuristics. Because `ChunkSource::text` stays a `String` and is
/// re-wrapped on resize (Option 1), the row is serialized to a canonical line
/// by [`to_line`] and recovered by [`parse_line`] — an exact-inverse codec
/// owned solely by this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffDisplayRow {
    /// A removed source line with its 1-based file line number.
    Removed { line_no: usize, text: String },
    /// An added source line with its 1-based file line number.
    Added { line_no: usize, text: String },
    /// An unchanged context line with its 1-based file line number.
    Context { line_no: usize, text: String },
    /// An elision row: a context gap (`"... (N lines)"`) or the collapse
    /// hint (`"... +N lines (select turn to /expand)"`). One uniform kind;
    /// `text` carries the already-indented elision string.
    Elision { text: String },
}

/// **Codec writer**: serialize a [`DiffDisplayRow`] to its canonical line.
pub fn to_line(row: &DiffDisplayRow) -> String {
    match row {
        DiffDisplayRow::Removed { line_no, text } => {
            format!("{} [R]- {text}", pad_left(*line_no))
        }
        DiffDisplayRow::Added { line_no, text } => {
            format!("{} [A]+ {text}", pad_left(*line_no))
        }
        DiffDisplayRow::Context { line_no, text } => {
            format!("{}   {text}", pad_left(*line_no))
        }
        DiffDisplayRow::Elision { text } => format!("{ELISION_SENTINEL}{text}"),
    }
}

/// **Codec reader**: parse a canonical line produced by [`to_line`] (possibly
/// after tree-connector prefixing / width re-wrap) back into a typed
/// [`DiffDisplayRow`]. Any line that is not a recognized changed/context shape
/// is an Elision row (gap markers, collapse hints, wrapped continuations).
pub fn parse_line(line: &str) -> DiffDisplayRow {
    // **CRITICAL #2**: an Elision is encoded with [`ELISION_SENTINEL`] so it
    // can never be confused with a context/marker row, even when its text is
    // shaped like one. The sentinel may sit behind a tree-connector prefix
    // (`"L "` on row 0, `"  "` on later rows) that `format_with_tree_connectors`
    // prepends AFTER `to_line`, so strip that first.
    if let Some(text) = strip_elision(line) {
        return DiffDisplayRow::Elision {
            text: text.to_string(),
        };
    }
    if let Some((line_no, text)) = split_marker(line, "[R]- ") {
        return DiffDisplayRow::Removed { line_no, text };
    }
    if let Some((line_no, text)) = split_marker(line, "[A]+ ") {
        return DiffDisplayRow::Added { line_no, text };
    }
    if let Some((line_no, text)) = split_context(line) {
        return DiffDisplayRow::Context { line_no, text };
    }
    DiffDisplayRow::Elision {
        text: line.to_string(),
    }
}

/// If `line` is a sentinel-encoded Elision (optionally behind a tree-connector
/// `"L "` / `"  "` prefix), return its original text with the sentinel removed.
pub(super) fn strip_elision(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix(ELISION_SENTINEL) {
        return Some(rest);
    }
    let after_connector = line
        .strip_prefix("L ")
        .or_else(|| line.strip_prefix("  "))?;
    after_connector.strip_prefix(ELISION_SENTINEL)
}

/// Right-align a line number to at least [`GUTTER_MIN_WIDTH`] columns.
pub fn pad_left(value: usize) -> String {
    let s = value.to_string();
    if s.len() >= GUTTER_MIN_WIDTH {
        s
    } else {
        format!("{}{}", " ".repeat(GUTTER_MIN_WIDTH - s.len()), s)
    }
}

/// Parse `"{connector?}{num} {MARKER}{text}"` → `(line_no, text)` where
/// `MARKER` is `"[R]- "` or `"[A]+ "`.
fn split_marker(line: &str, marker: &str) -> Option<(usize, String)> {
    let pos = line.find(marker)?;
    let before = line.get(..pos)?.strip_suffix(' ')?;
    let line_no = leading_line_no(before)?;
    let text = line.get(pos + marker.len()..)?.to_string();
    Some((line_no, text))
}

/// Parse a context line `"{connector?}{num}   {text}"` (exactly three spaces
/// after the number) → `(line_no, text)`.
fn split_context(line: &str) -> Option<(usize, String)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'L' || bytes[i] == b' ') {
        i += 1;
    }
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return None;
    }
    if line.get(i..i + 3) != Some("   ") {
        return None;
    }
    let line_no = line.get(digit_start..i)?.parse::<usize>().ok()?;
    let text = line.get(i + 3..)?.to_string();
    Some((line_no, text))
}

/// Recover the 1-based line number from the gutter prefix.
fn leading_line_no(prefix: &str) -> Option<usize> {
    let trimmed = prefix
        .strip_prefix('L')
        .or_else(|| prefix.strip_prefix(' '))
        .unwrap_or(prefix)
        .trim_start_matches(' ');
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    trimmed.parse::<usize>().ok()
}
