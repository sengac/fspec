//! Lenient Gherkin parser front-end that closes a known parity gap
//! between TypeScript `@cucumber/gherkin` and the Rust `gherkin-0.16.0`
//! crate.
//!
//! ## Why this exists
//!
//! `@cucumber/gherkin` (TS) treats lines inside description blocks as
//! free-form prose: a line that *starts* with a keyword such as
//! `Background-priority placeholder` is fine because the keyword is not
//! followed by `:`. The Rust `gherkin-0.16.0` parser is stricter — its
//! `description_line` rule rejects any line whose first non-whitespace
//! characters happen to match a structural keyword prefix, regardless
//! of what follows, which makes the *whole feature file* fail to
//! parse.
//!
//! The Rust parser is also stricter about table-cell escape sequences:
//! only `\n`, `\|`, and `\\` are accepted, so a cell containing a JSON
//! payload such as `"{\"command\":\"board\"}"` aborts the parse. TS
//! treats `\"` as a literal `"` (or, more precisely, falls back to the
//! verbatim characters).
//!
//! These two gaps cause five real `spec/features/*.feature` files in
//! this repo to be silently dropped from any command that calls
//! `Feature::parse` (RPC-299 `show-acceptance-criteria`, RPC-304
//! `show-feature`, RPC-130 `list-scenario-tags`, RPC-198
//! `show-work-unit`). The parity-fix orchestration treats this as
//! "fix ALL issues, even pre-existing".
//!
//! ## Strategy
//!
//! [`parse_feature_lenient`] is a two-stage parser:
//!
//! 1. Try `Feature::parse(content, GherkinEnv::default())` unmodified.
//! 2. On any parse error, run [`sanitize_for_gherkin`] on the source
//!    and retry. The sanitiser walks the file line by line, tracks
//!    which lines belong to a description block, and rewrites only
//!    those lines:
//!    - Description lines whose first non-whitespace word matches a
//!      Gherkin structural or step keyword are prefixed with `U+200B`
//!      (ZERO WIDTH SPACE). `[_]` in pest matches one Unicode codepoint
//!      so the prefix breaks the `starts_with("Background")` check in
//!      `keyword1`, but `not_nl()` still captures the line as
//!      description text.
//!    - Step-table cell content `\"` is rewritten to `\\"` (escape the
//!      backslash) so the parser's official `\\` → `\` rule reproduces
//!      the original `\"` byte pair when the cell is materialised by
//!      downstream code.
//!
//! Verbatim description rendering (see
//! `commands/show_acceptance_criteria::extract_description_verbatim`)
//! continues to read from the **original**, un-sanitised content, so
//! the U+200B never appears in user-visible output.

use gherkin::{Feature, GherkinEnv, ParseError};

/// Parse a Gherkin feature file with a one-shot lenient fallback when
/// the strict parser fails on description prose or table escape
/// sequences that `@cucumber/gherkin` would accept.
///
/// Returns `Ok(feature)` on success (lenient or strict), `Err(_)`
/// when even the sanitised content cannot be parsed.
pub fn parse_feature_lenient(content: &str) -> Result<Feature, ParseError> {
    match Feature::parse(content, GherkinEnv::default()) {
        Ok(f) => Ok(f),
        Err(_strict_err) => {
            let sanitised = sanitize_for_gherkin(content);
            Feature::parse(&sanitised, GherkinEnv::default())
        }
    }
}

/// Rewrite description prose and step-table cells so the Rust
/// `gherkin-0.16.0` parser accepts content that `@cucumber/gherkin`
/// would have accepted unchanged. The sanitiser is conservative — it
/// only edits lines inside description blocks or step tables, and only
/// when they trigger one of the two known divergences.
///
/// Description preprocessing inserts `U+200B` (zero width space) after
/// existing indentation when a line's first non-whitespace word is a
/// Gherkin structural / step keyword used without a trailing colon
/// (`Background-priority`, `Scenario Outline if present)`, etc.). The
/// prefix breaks the parser's `starts_with` keyword check but is
/// otherwise invisible because the verbatim description extractor in
/// `show_acceptance_criteria` reads from the original source.
///
/// Step-table cell preprocessing rewrites `\"` → `\\"` so the parser's
/// official `\\` escape collapses back to the original `\"` byte pair.
pub fn sanitize_for_gherkin(content: &str) -> String {
    let mut out = String::with_capacity(content.len() + 64);
    let mut state = State::Outside;

    for raw_line in split_lines_preserving_endings(content) {
        let (line, ending) = raw_line;
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();

        // Update state machine using the *original* line content.
        let starts_block = is_description_block_header(trimmed);
        // What terminates a description block? Only structural keywords
        // (a new Scenario:/Background:/etc. is caught via `starts_block`)
        // and the first step (`Given`/`When`/`Then`/…) line of the next
        // scenario. We deliberately do NOT treat `"""` as a terminator
        // because Gherkin feature descriptions sometimes wrap their
        // architecture-note prose in a triple-quoted block (a docstring
        // used purely for visual grouping), and that whole block is
        // still part of the description so far as the parser is
        // concerned. Pipe lines and `@` tag lines DO terminate a
        // description because the next scenario's table or tag run
        // begins there.
        let ends_block_strong =
            trimmed.starts_with('|') || is_step_keyword_prefix(trimmed) || trimmed.starts_with('@');
        let comment = trimmed.starts_with('#');

        let mut rewritten: Option<String> = None;

        if matches!(state, State::InDescription) && !trimmed.is_empty() && !comment {
            if ends_block_strong {
                state = State::Outside;
            } else if line_starts_with_keyword(trimmed) {
                // Rewrite: preserve indentation, then ZWSP, then trimmed text.
                let mut s = String::with_capacity(indent_len + 3 + trimmed.len());
                s.push_str(&line[..indent_len]);
                s.push('\u{200B}');
                s.push_str(trimmed);
                rewritten = Some(s);
            }
        }

        if starts_block {
            state = State::InDescription;
        }

        // Step-table escape preprocessing operates independently of
        // description state — pipe tables can appear under any step
        // keyword or Examples header.
        let mut s_to_push = rewritten.unwrap_or_else(|| line.to_string());
        if trimmed.starts_with('|') {
            s_to_push = rewrite_table_row_escapes(&s_to_push);
        }

        out.push_str(&s_to_push);
        out.push_str(ending);
    }

    out
}

#[derive(Copy, Clone)]
enum State {
    Outside,
    InDescription,
}

/// Split `content` into `(line_without_ending, ending)` pairs,
/// preserving original line terminators (`\r\n`, `\n`, or empty for
/// the trailing run when the file does not end in a newline).
fn split_lines_preserving_endings(content: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i] != b'\n' {
            i += 1;
        }
        let line_end = i;
        let mut ending_end = i;
        if i < bytes.len() && bytes[i] == b'\n' {
            ending_end = i + 1;
            i = ending_end;
        }
        // Trim trailing \r from the line content but keep it in the
        // ending so we round-trip CRLF unchanged.
        let line_str = if line_end > start && bytes[line_end - 1] == b'\r' {
            &content[start..line_end - 1]
        } else {
            &content[start..line_end]
        };
        let ending_str = if line_end > start && bytes[line_end - 1] == b'\r' {
            &content[line_end - 1..ending_end]
        } else {
            &content[line_end..ending_end]
        };
        out.push((line_str, ending_str));
    }
    out
}

/// Identify lines that start a new description-capable block
/// (`Feature:`, `Rule:`, `Background:`, `Scenario:`, `Scenario
/// Outline:`, `Example:`, `Examples:`).
fn is_description_block_header(trimmed: &str) -> bool {
    const HEADERS: &[&str] = &[
        "Feature:",
        "Rule:",
        "Background:",
        "Scenario Outline:",
        "Scenario Template:",
        "Scenario:",
        "Example:",
        "Examples:",
    ];
    HEADERS.iter().any(|h| trimmed.starts_with(h))
}

/// True when the trimmed line starts with a step keyword followed by a
/// space — the lines that terminate a Scenario/Background description.
fn is_step_keyword_prefix(trimmed: &str) -> bool {
    const STEPS: &[&str] = &["Given ", "When ", "Then ", "And ", "But ", "* "];
    STEPS.iter().any(|k| trimmed.starts_with(k))
}

/// True when the line's first word matches a Gherkin keyword used
/// without a trailing colon — this is exactly the case the strict
/// `description_line` rule rejects.
fn line_starts_with_keyword(trimmed: &str) -> bool {
    // Ordered longest-first so `Scenario Outline` is checked before
    // `Scenario`.
    const KEYWORDS: &[&str] = &[
        "Scenario Outline",
        "Scenario Template",
        "Background",
        "Scenario",
        "Examples",
        "Example",
        "Feature",
        "Rule",
        "Given",
        "When",
        "Then",
        "And",
        "But",
    ];
    for kw in KEYWORDS {
        if let Some(rest) = trimmed.strip_prefix(kw) {
            // If the line is *exactly* the keyword followed by a colon
            // (or about to be — leading whitespace then `:`), let the
            // parser handle it normally.
            let next = rest.chars().next();
            if next == Some(':') {
                return false;
            }
            // `Background-priority`, `Scenario Outline if present)`,
            // `BackgroundOutput at the canonical call sites`, etc. all
            // hit this branch.
            return true;
        }
    }
    false
}

/// Rewrite a step-table row so unknown escape sequences (`\"`, `\'`,
/// `\t`, …) are doubled into `\\X`. The strict parser only accepts
/// `\n`, `\|`, and `\\` inside cells; everything else aborts the row.
/// `@cucumber/gherkin` falls back to literal bytes for unknown
/// escapes, which is what we approximate here.
fn rewrite_table_row_escapes(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 8);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('\\') => {
                    // Real `\\` escape — leave as-is.
                    out.push('\\');
                    out.push('\\');
                    chars.next();
                }
                Some(&c2 @ ('|' | 'n')) => {
                    out.push('\\');
                    out.push(c2);
                    chars.next();
                }
                Some(_) => {
                    // Unknown escape — double the backslash so the
                    // parser's `\\` rule produces a single `\` and the
                    // following char survives verbatim.
                    out.push('\\');
                    out.push('\\');
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn parse_strict_first_succeeds_without_mutation() {
        let src = "Feature: T\n  As a user\n  I want\n  Scenario: A\n    Given x\n";
        let f = parse_feature_lenient(src).expect("strict parse");
        assert_eq!(f.name.as_str(), "T");
    }

    #[test]
    fn description_with_background_word_parses_via_sanitiser() {
        let src = concat!(
            "Feature: HelloComponent (Background-priority placeholder)\n",
            "  Background-priority placeholder Component\n",
            "  that renders a centred static greeting.\n",
            "\n",
            "  Scenario: A\n",
            "    Given x\n",
        );
        // Strict parse should fail.
        assert!(Feature::parse(src, GherkinEnv::default()).is_err());
        // Lenient parse should succeed.
        let f = parse_feature_lenient(src).expect("lenient parse");
        assert_eq!(
            f.name.as_str(),
            "HelloComponent (Background-priority placeholder)"
        );
        assert_eq!(f.scenarios.len(), 1);
    }

    #[test]
    fn description_with_scenario_outline_word_parses_via_sanitiser() {
        let src = concat!(
            "Feature: Add Scenario\n",
            "  - MUST insert scenario in correct location (after other scenarios, before\n",
            "  Scenario Outline if present)\n",
            "\n",
            "  Scenario: A\n",
            "    Given x\n",
        );
        assert!(Feature::parse(src, GherkinEnv::default()).is_err());
        let f = parse_feature_lenient(src).expect("lenient parse");
        assert_eq!(f.name.as_str(), "Add Scenario");
    }

    #[test]
    fn table_cell_with_escaped_quote_parses_via_sanitiser() {
        let src = concat!(
            "Feature: T\n",
            "  Scenario: A\n",
            "    When the chunks subscriber forwards:\n",
            "      | chunk                                                  |\n",
            "      | ToolCall { input: \"{\\\"command\\\":\\\"board\\\"}\" } |\n",
            "    Then ok\n",
        );
        assert!(Feature::parse(src, GherkinEnv::default()).is_err());
        let f = parse_feature_lenient(src).expect("lenient parse");
        assert_eq!(f.scenarios.len(), 1);
    }

    #[test]
    fn sanitizer_is_noop_for_clean_descriptions() {
        let src = "Feature: T\n  As a user.\n\n  Scenario: A\n    Given x\n";
        assert_eq!(sanitize_for_gherkin(src), src);
    }

    #[test]
    fn sanitizer_preserves_line_endings() {
        let src = "Feature: T\r\n  As a user.\r\n\r\n  Scenario: A\r\n    Given x\r\n";
        assert_eq!(sanitize_for_gherkin(src), src);
    }
}
