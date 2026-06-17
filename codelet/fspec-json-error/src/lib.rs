//! Human-friendly `serde_json` parse-error formatter for fspec.
//!
//! # Provenance
//!
//! This crate is **vendored and trimmed** from
//! [`format_serde_error`](https://github.com/AlexanderThaller/format_serde_error)
//! version `0.3.0` by Alexander Thaller (MIT licensed). The original supports
//! `serde_yaml`, `toml`, and terminal colouring via the `colored` crate behind
//! Cargo features. fspec needs none of those:
//!
//! * Only `serde_json` is relevant (every fspec state file is JSON).
//! * Colour is applied at the CLI bridge layer (chalk-equivalent), not here —
//!   the core stays colour-free so its `Display` output is byte-stable and
//!   easy to assert on in tests.
//!
//! So this version drops the YAML/TOML/colour code paths and the global
//! `AtomicBool`/`AtomicUsize` configuration statics (which were un-idiomatic
//! for a library — see upstream issue #19). Configuration is per-instance.
//!
//! # Fixes applied versus upstream 0.3.0
//!
//! * **Upstream issue #20 — "Panic on small characters count".** The original
//!   computed the caret column as `error_column - whitespace_count +
//!   ellipse_space`, which underflows (panics in debug, wraps in release) when
//!   a long line is contextualised with a small `context_characters` value.
//!   Root cause: the de-indentation offset (`whitespace_count`) was subtracted
//!   a second time from a column that had already been re-based by the
//!   long-line windowing. The fix de-indents the error column **once**, before
//!   windowing, and never subtracts `whitespace_count` again.
//! * Removed the buggy `get_default_contextualize` free function (it read the
//!   `CONTEXT_LINES` counter and returned a `usize` for a boolean concept).
//!
//! # Usage
//!
//! ```
//! use codelet_fspec_json_error::SerdeError;
//!
//! let input = "{ bad";
//! let err = serde_json::from_str::<serde_json::Value>(input).unwrap_err();
//! let pretty = SerdeError::from_json(input.to_string(), &err).to_string();
//! // pretty contains the offending line plus a caret under the error column.
//! ```

#![forbid(unsafe_code)]

use std::fmt;

use serde_json::Error as JsonError;
use unicode_segmentation::UnicodeSegmentation;

/// Default number of context lines shown before and after the error line.
pub const CONTEXT_LINES_DEFAULT: usize = 3;

/// Default number of context characters shown before and after the error
/// column when a long line is shortened.
pub const CONTEXT_CHARACTERS_DEFAULT: usize = 30;

/// Separator drawn between the line-number gutter and the source text.
const SEPARATOR: &str = " | ";

/// Ellipsis used to indicate a long line has been shortened.
const ELLIPSE: &str = "...";

/// A `serde_json` parse error paired with the source text it came from,
/// rendered on `Display` as the offending line plus a caret under the exact
/// error column.
#[derive(Debug, Clone)]
pub struct SerdeError {
    input: String,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
    contextualize: bool,
    context_lines: usize,
    context_characters: usize,
}

impl std::error::Error for SerdeError {}

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f)
    }
}

impl SerdeError {
    /// Build a [`SerdeError`] from the source `input` and a [`serde_json`]
    /// error. Uses the default context settings ([`CONTEXT_LINES_DEFAULT`],
    /// [`CONTEXT_CHARACTERS_DEFAULT`]).
    #[must_use]
    pub fn from_json(input: String, err: &JsonError) -> Self {
        Self {
            input,
            message: err.to_string(),
            line: Some(err.line()),
            column: Some(err.column()),
            contextualize: true,
            context_lines: CONTEXT_LINES_DEFAULT,
            context_characters: CONTEXT_CHARACTERS_DEFAULT,
        }
    }

    /// Enable or disable contextualisation. When disabled, no surrounding
    /// lines are shown and long lines are left intact.
    pub fn set_contextualize(&mut self, should_contextualize: bool) -> &mut Self {
        self.contextualize = should_contextualize;
        self
    }

    /// Number of lines shown before and after the error line.
    pub fn set_context_lines(&mut self, amount: usize) -> &mut Self {
        self.context_lines = amount;
        self
    }

    /// Number of characters shown before and after the error column when a
    /// long line is shortened.
    pub fn set_context_characters(&mut self, amount: usize) -> &mut Self {
        self.context_characters = amount;
        self
    }

    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // No location → nothing nicer than the raw message can be produced.
        if self.line.is_none() && self.column.is_none() {
            return writeln!(f, "{}", self.message);
        }

        let error_line = self.line.unwrap_or_default();
        let error_column = self.column.unwrap_or_default();

        let context_lines = self.context_lines;

        // Skip to `context_lines` before the error line (+1 for the error line
        // itself); saturating because the error may be near the top.
        let skip = usize::saturating_sub(error_line, context_lines + 1);
        let take = context_lines * 2 + 1;

        let minimized_input = self
            .input
            .lines()
            .skip(skip)
            .take(take)
            .map(|line| line.replace('\t', " "))
            .collect::<Vec<_>>();

        // Empty window → input was effectively empty; fall back to raw message.
        if minimized_input.is_empty() {
            return writeln!(f, "{}", self.message);
        }

        // Strip the common leading indentation so the snippet is compact.
        let whitespace_count = minimized_input
            .iter()
            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or_default();

        let separator = SEPARATOR;

        // Filler matching the width of the largest line number in the gutter.
        let fill_line_position = format!("{: >fill$}", "", fill = error_line.to_string().len());

        // Leading newline: callers (e.g. an "Error: " prefix) may have already
        // written to the start of the line.
        writeln!(f)?;

        self.input
            .lines()
            .enumerate()
            .skip(skip)
            .take(take)
            .map(|(index, text)| {
                (
                    index + 1,
                    text.chars()
                        .skip(whitespace_count)
                        .collect::<String>()
                        .replace('\t', " "),
                )
            })
            .try_for_each(|(line_position, text)| {
                self.format_line(
                    f,
                    line_position,
                    error_line,
                    error_column,
                    whitespace_count,
                    text,
                    separator,
                    &fill_line_position,
                )
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn format_line(
        &self,
        f: &mut fmt::Formatter<'_>,
        line_position: usize,
        error_line: usize,
        error_column: usize,
        whitespace_count: usize,
        text: String,
        separator: &str,
        fill_line_position: &str,
    ) -> fmt::Result {
        if line_position != error_line {
            return if self.contextualize {
                Self::format_context_line(f, &text, separator, fill_line_position)
            } else {
                Ok(())
            };
        }

        // De-indent the error column ONCE, to match the de-indented `text`.
        // (Upstream issue #20 fix: never subtract `whitespace_count` again.)
        let de_indented_column = error_column.saturating_sub(whitespace_count);

        let long_line = self.contextualize && (self.context_characters * 2 + 1) < text.len();

        let (context_line, marker_column, context_before, context_after) = if long_line {
            Self::context_long_line(&text, de_indented_column, self.context_characters)
        } else {
            (text, de_indented_column, false, false)
        };

        Self::format_error_line(
            f,
            &context_line,
            line_position,
            separator,
            context_before,
            context_after,
        )?;

        self.format_error_information(
            f,
            separator,
            fill_line_position,
            marker_column,
            context_before,
        )
    }

    fn format_error_line(
        f: &mut fmt::Formatter<'_>,
        text: &str,
        line_position: usize,
        separator: &str,
        context_before: bool,
        context_after: bool,
    ) -> fmt::Result {
        write!(f, " {line_position}{separator}")?;

        if context_before {
            write!(f, "{ELLIPSE}")?;
        }

        write!(f, "{text}")?;

        if context_after {
            write!(f, "{ELLIPSE}")?;
        }

        writeln!(f)
    }

    fn format_error_information(
        &self,
        f: &mut fmt::Formatter<'_>,
        separator: &str,
        fill_line_position: &str,
        marker_column: usize,
        context_before: bool,
    ) -> fmt::Result {
        let ellipse_space = if context_before { ELLIPSE.len() } else { 0 };

        // `marker_column` is already de-indented (and re-based for long lines),
        // so we only add the ellipsis width. No `whitespace_count` subtraction
        // here — that was the source of upstream issue #20's overflow.
        let column = marker_column + ellipse_space;

        let caret = format!("{: >column$}^ {}", "", self.message);

        writeln!(f, " {fill_line_position}{separator}{caret}")
    }

    fn format_context_line(
        f: &mut fmt::Formatter<'_>,
        text: &str,
        separator: &str,
        fill_line_position: &str,
    ) -> fmt::Result {
        writeln!(f, " {fill_line_position}{separator}{text}")
    }

    /// Shorten a long error line to a window of `context_chars` graphemes on
    /// each side of the error column. Returns the windowed text, the re-based
    /// marker column, and whether an ellipsis is needed before/after.
    fn context_long_line(
        text: &str,
        error_column: usize,
        context_chars: usize,
    ) -> (String, usize, bool, bool) {
        // Graphemes, not chars: a single user-perceived character may be
        // multiple code points.
        let input = text.graphemes(true).collect::<Vec<_>>();

        let skip = usize::saturating_sub(error_column, context_chars + 1);
        let take = context_chars * 2 + 1;

        let context_before = skip != 0;
        let context_after = skip + take < input.len();

        let minimized_input = input.into_iter().skip(skip).take(take).collect();
        let new_error_column = usize::saturating_sub(error_column, skip);

        (
            minimized_input,
            new_error_column,
            context_before,
            context_after,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn render(input: &str) -> String {
        let err = serde_json::from_str::<serde_json::Value>(input).unwrap_err();
        SerdeError::from_json(input.to_string(), &err).to_string()
    }

    #[test]
    fn points_caret_at_error_column_on_single_line() {
        let out = render("{ bad");
        assert!(out.contains("1 | { bad"), "missing source line: {out}");
        // serde reports column 3 → caret under the third column.
        assert!(out.contains("^ "), "missing caret: {out}");
        assert!(out.contains("line 1 column 3"), "missing message: {out}");
    }

    #[test]
    fn shows_offending_line_with_context_for_multiline_input() {
        let input = "{\n  \"version\": \"0.7.1\",\n  \"workUnits\": {\n    \"AUTH-001\": { \"id\": \"x\", status: \"done\" }\n  }\n}";
        let out = render(input);
        // The error line (4) must be numbered and present.
        assert!(out.contains("4 |"), "missing numbered error line: {out}");
        assert!(out.contains("status:"), "missing offending content: {out}");
    }

    #[test]
    fn falls_back_to_raw_message_for_empty_input() {
        // EOF errors report line 1 column 0; the window is empty so we expect
        // the bare message with no caret.
        let out = render("");
        assert!(out.contains("EOF while parsing"), "unexpected: {out}");
        assert!(!out.contains('^'), "should not draw a caret: {out}");
    }

    #[test]
    fn issue_20_small_context_characters_does_not_panic() {
        // Verbatim reproduction from upstream issue #20.
        let input = r#"[
                [1],
                [2],
                [3],
                [4],
                [1, 2, 3, 4, -5],
                [6]
            ]"#;
        let err = serde_json::from_str::<Vec<Vec<u32>>>(input).unwrap_err();
        let mut se = SerdeError::from_json(input.to_string(), &err);
        se.set_context_characters(9);
        // Must not panic.
        let out = se.to_string();
        assert!(out.contains('^'), "expected a caret in: {out}");
    }

    #[test]
    fn context_characters_zero_does_not_panic() {
        let input = "[\n  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, -5]\n]";
        let err = serde_json::from_str::<Vec<Vec<u32>>>(input).unwrap_err();
        let mut se = SerdeError::from_json(input.to_string(), &err);
        se.set_context_characters(0);
        let _ = se.to_string();
    }

    #[test]
    fn contextualize_disabled_emits_no_context_lines() {
        let input = "{\n  \"a\": 1,\n  \"b\": 2,\n  bad\n}";
        let err = serde_json::from_str::<serde_json::Value>(input).unwrap_err();
        let mut se = SerdeError::from_json(input.to_string(), &err);
        se.set_contextualize(false);
        let out = se.to_string();
        // Only the error line should appear, not the surrounding "a"/"b" lines.
        assert!(!out.contains("\"a\": 1"), "should omit context: {out}");
    }
}
