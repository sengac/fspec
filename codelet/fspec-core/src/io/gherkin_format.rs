//! Hand-ported AST-based Gherkin formatter — Rust port of
//! `src/utils/gherkin-formatter.ts` (RPC-230).
//!
//! Re-emits a parsed [`gherkin::Feature`] as canonical feature-file text.
//! Reproduces the TypeScript `GherkinFormatter` byte-for-byte against the
//! structural content the Rust `gherkin-0.16.0` parser preserves:
//!
//! - 2-space scenario / Background / Rule indentation, 4-space step indentation
//! - per-column-aligned data tables and Examples tables
//! - preserved doc strings (`"""`), tags (each on its own line)
//! - a blank line before each feature child and before each Examples block
//! - a single trailing newline
//!
//! ## Parity caveats (Rust gherkin crate vs `@cucumber/messages`)
//!
//! The Rust `gherkin-0.16.0` AST is structurally leaner than the TS
//! `@cucumber/messages` AST:
//!   - It does NOT retain comments → comment re-insertion is a no-op (the TS
//!     `commentMap` machinery has nothing to feed it for re-parsed files).
//!   - Step keywords are stored WITHOUT a trailing space; we re-add the single
//!     space between keyword and value to match TS `${step.keyword}${step.text}`
//!     (TS keyword carries the trailing space).
//!   - DocString media types / custom delimiters are not retained; we emit the
//!     canonical `"""` delimiter pair.
//!
//! These are acceptable for the formatter's purpose: feature files written by
//! fspec use the canonical keyword set and `"""` doc strings, so a
//! parse→format round-trip is idempotent.

use gherkin::{Background, Examples, Feature, Rule, Scenario, Step, Table};

const INDENT: &str = "  ";

/// Format a parsed [`Feature`] back to canonical feature-file text with a
/// single trailing newline.
pub fn format_feature(feature: &Feature) -> String {
    let mut lines: Vec<String> = Vec::new();
    format_feature_into(feature, &mut lines);
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn indent(level: usize) -> String {
    INDENT.repeat(level)
}

fn format_feature_into(feature: &Feature, lines: &mut Vec<String>) {
    // Tags — each on its own line (no indentation at feature level).
    for tag in &feature.tags {
        lines.push(format!("@{tag}"));
    }

    // Feature keyword and name.
    lines.push(format!("Feature: {}", feature.name));

    // Description (free-form prose under the Feature header, indent level 1).
    if let Some(desc) = &feature.description {
        format_description(desc, lines, 1);
    }

    // Children: a Background (if any) then scenarios then rules, in the order
    // the TS formatter walks `feature.children`. The Rust AST splits these
    // into separate typed collections; we emit Background first, then
    // scenarios, then rules — matching the canonical authoring order fspec
    // uses (Background always precedes scenarios; rules follow). TS pushes a
    // blank line before EVERY feature child, so we do the same unconditionally.
    if let Some(bg) = &feature.background {
        push_blank_before_child(lines);
        format_background(bg, lines, 0);
    }

    for scenario in &feature.scenarios {
        push_blank_before_child(lines);
        format_scenario(scenario, lines, 0);
    }

    for rule in &feature.rules {
        push_blank_before_child(lines);
        format_rule(rule, lines, 0);
    }
}

/// TS pushes a blank line before EVERY feature child (index 0 and onward).
fn push_blank_before_child(lines: &mut Vec<String>) {
    lines.push(String::new());
}

fn format_background(bg: &Background, lines: &mut Vec<String>, base_indent: usize) {
    let ind = indent(base_indent + 1);
    lines.push(format!("{ind}Background: {}", bg.name));

    if let Some(desc) = &bg.description {
        format_description(desc, lines, base_indent + 2);
    }

    for step in &bg.steps {
        format_step(step, lines, base_indent + 2);
    }
}

fn format_scenario(scenario: &Scenario, lines: &mut Vec<String>, base_indent: usize) {
    let ind = indent(base_indent + 1);

    // Tags.
    for tag in &scenario.tags {
        lines.push(format!("{ind}@{tag}"));
    }

    // Keyword (raw keyword, e.g. "Scenario" or "Scenario Outline").
    lines.push(format!("{ind}{}: {}", scenario.keyword.trim(), scenario.name));

    if let Some(desc) = &scenario.description {
        format_description(desc, lines, base_indent + 2);
    }

    for step in &scenario.steps {
        format_step(step, lines, base_indent + 2);
    }

    // Examples (Scenario Outline).
    for examples in &scenario.examples {
        lines.push(String::new()); // Blank line before Examples.
        format_examples(examples, lines, base_indent + 2);
    }
}

fn format_rule(rule: &Rule, lines: &mut Vec<String>, base_indent: usize) {
    let ind = indent(base_indent + 1);

    for tag in &rule.tags {
        lines.push(format!("{ind}@{tag}"));
    }

    lines.push(format!("{ind}Rule: {}", rule.name));

    if let Some(desc) = &rule.description {
        format_description(desc, lines, base_indent + 2);
    }

    // Rule children: a Background then scenarios. TS inserts a blank line
    // before each child when `index > 0 || rule.description`.
    let mut idx = 0usize;
    let has_desc = rule.description.is_some();

    if let Some(bg) = &rule.background {
        if idx > 0 || has_desc {
            lines.push(String::new());
        }
        format_background(bg, lines, base_indent + 1);
        idx += 1;
    }

    for scenario in &rule.scenarios {
        if idx > 0 || has_desc {
            lines.push(String::new());
        }
        format_scenario(scenario, lines, base_indent + 1);
        idx += 1;
    }
}

fn format_step(step: &Step, lines: &mut Vec<String>, indent_level: usize) {
    let ind = indent(indent_level);

    // Rust keyword has no trailing space; TS keyword includes one. Re-add a
    // single separating space.
    lines.push(format!("{ind}{} {}", step.keyword.trim_end(), step.value));

    if let Some(docstring) = &step.docstring {
        format_docstring(docstring, lines, indent_level + 1);
    }

    if let Some(table) = &step.table {
        format_table(table, lines, indent_level + 1);
    }
}

fn format_examples(examples: &Examples, lines: &mut Vec<String>, indent_level: usize) {
    let ind = indent(indent_level);

    for tag in &examples.tags {
        lines.push(format!("{ind}@{tag}"));
    }

    // Examples keyword followed by colon (TS: `${keyword}:`, ignoring name).
    lines.push(format!("{ind}{}:", examples.keyword.trim()));

    if let Some(desc) = &examples.description {
        format_description(desc, lines, indent_level + 1);
    }

    if let Some(table) = &examples.table {
        format_table(table, lines, indent_level + 1);
    }
}

/// Format a step doc string back to canonical `"""` form.
///
/// ## Parity with the TS cucumber-messages `DocString`
///
/// The TS formatter receives `docString.content` (the text BETWEEN the
/// delimiter lines, with the structural newline after the opening delimiter
/// and before the closing delimiter already removed) plus a separate
/// `docString.mediaType`.
///
/// The Rust `gherkin-0.16.0` parser instead stores the raw text between the
/// delimiters run through `textwrap::dedent`. For the canonical
/// `"""`-on-its-own-line form that fspec emits, that value therefore carries:
///   1. a leading `\n` (the newline right after the opening `"""`),
///   2. a trailing `\n` (the newline right before the closing `"""`),
///   3. — when a media type is present (`"""json`) — the media type as the
///      first line, which also defeats `dedent` (the unindented media-type
///      token forces the common prefix to `""`, leaving the body indented).
///
/// To reproduce the TS bytes (and keep `format` idempotent) we: split off the
/// media type, strip the one structural leading + trailing newline, and
/// re-dedent the body so the body indentation matches cucumber's content.
fn format_docstring(docstring: &str, lines: &mut Vec<String>, indent_level: usize) {
    let ind = indent(indent_level);
    let delimiter = "\"\"\"";

    // Separate the optional media type from the body. When the value starts
    // with `\n`, the opening delimiter was on its own line → no media type.
    let (media_type, body) = match docstring.strip_prefix('\n') {
        Some(rest) => (String::new(), rest.to_string()),
        None => match docstring.split_once('\n') {
            Some((mt, rest)) => (mt.trim().to_string(), rest.to_string()),
            None => (docstring.trim().to_string(), String::new()),
        },
    };

    // Strip the single structural newline before the closing delimiter.
    let body = body.strip_suffix('\n').unwrap_or(&body);

    // Re-dedent the body. This is a no-op for the no-media-type case (the
    // parser already dedented it) and removes the residual indentation the
    // parser could not strip when a media type was present.
    let body = dedent(body);

    lines.push(format!("{ind}{delimiter}{media_type}"));

    for line in body.split('\n') {
        if line.trim().is_empty() {
            lines.push(String::new());
        } else {
            lines.push(format!("{ind}{line}"));
        }
    }

    lines.push(format!("{ind}{delimiter}"));
}

/// Remove the longest common leading-whitespace prefix shared by all
/// non-blank lines, normalising blank (whitespace-only) lines to empty.
///
/// Faithful re-implementation of `textwrap::dedent` (the same routine the
/// `gherkin` parser applies to doc-string bodies) so a re-dedent here is a
/// strict no-op on already-dedented content.
fn dedent(s: &str) -> String {
    let mut prefix: Option<&str> = None;
    for line in s.lines() {
        let trimmed_len = line.len() - line.trim_start().len();
        if trimmed_len == line.len() {
            // Whitespace-only (or empty) line — ignored when computing prefix.
            continue;
        }
        let leading = &line[..trimmed_len];
        prefix = Some(match prefix {
            None => leading,
            Some(p) => common_prefix(p, leading),
        });
    }

    let prefix = prefix.unwrap_or("");
    let mut result = String::new();
    let trailing_newline = s.ends_with('\n');
    let mut lines = s.lines().peekable();
    while let Some(line) = lines.next() {
        if line.chars().any(|c| !c.is_whitespace()) {
            result.push_str(&line[prefix.len()..]);
        }
        if lines.peek().is_some() || trailing_newline {
            result.push('\n');
        }
    }
    result
}

/// Longest common leading substring of two whitespace prefixes.
fn common_prefix<'a>(a: &'a str, b: &str) -> &'a str {
    let end = a
        .char_indices()
        .zip(b.chars())
        .take_while(|((_, ac), bc)| ac == bc)
        .map(|((i, ac), _)| i + ac.len_utf8())
        .last()
        .unwrap_or(0);
    &a[..end]
}

fn format_table(table: &Table, lines: &mut Vec<String>, indent_level: usize) {
    if table.rows.is_empty() {
        return;
    }

    let ind = indent(indent_level);

    // Per-column max width (in chars, matching TS `cell.value.length`).
    let mut column_widths: Vec<usize> = Vec::new();
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            let width = cell.chars().count();
            if i >= column_widths.len() {
                column_widths.resize(i + 1, 0);
            }
            if width > column_widths[i] {
                column_widths[i] = width;
            }
        }
    }

    for row in &table.rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| pad_end(cell, column_widths[i]))
            .collect();
        lines.push(format!("{ind}| {} |", cells.join(" | ")));
    }
}

/// Mirror JS `String.prototype.padEnd`: pad with spaces to `width` chars
/// (counted by Unicode scalar values, as JS counts UTF-16 code units — close
/// enough for the ASCII-dominant content fspec emits, and identical for the
/// common case).
fn pad_end(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        let mut out = String::from(s);
        out.push_str(&" ".repeat(width - len));
        out
    }
}

/// Format free-form description prose at the given indent level, collapsing
/// runs of blank lines to at most 2 consecutive (parity with TS
/// `consecutiveBlankLines < 2`).
fn format_description(description: &str, lines: &mut Vec<String>, indent_level: usize) {
    let ind = indent(indent_level);
    let mut consecutive_blank = 0usize;

    for line in description.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if consecutive_blank < 2 {
                lines.push(String::new());
                consecutive_blank += 1;
            }
        } else {
            consecutive_blank = 0;
            lines.push(format!("{ind}{trimmed}"));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use gherkin::GherkinEnv;

    fn parse(src: &str) -> Feature {
        Feature::parse(src, GherkinEnv::default()).expect("parse")
    }

    #[test]
    fn renormalises_step_indentation_to_four_spaces() {
        let src = "Feature: One\n\n  Scenario: A\n  Given x\n  When y\n  Then z\n";
        let out = format_feature(&parse(src));
        assert!(out.contains("    Given x"), "got:\n{out}");
        assert!(out.contains("    When y"), "got:\n{out}");
        assert!(out.contains("    Then z"), "got:\n{out}");
    }

    #[test]
    fn single_trailing_newline() {
        let src = "Feature: One\n\n  Scenario: A\n    Given x\n";
        let out = format_feature(&parse(src));
        assert!(out.ends_with('\n'));
        assert!(!out.ends_with("\n\n"));
    }

    #[test]
    fn idempotent() {
        let src = "Feature: One\n\n  Scenario: A\n  Given x\n  When y\n  Then z\n";
        let once = format_feature(&parse(src));
        let twice = format_feature(&parse(&once));
        assert_eq!(once, twice, "format must be idempotent");
    }

    #[test]
    fn preserves_tags() {
        let src = "@foo\n@bar\nFeature: One\n\n  @baz\n  Scenario: A\n    Given x\n";
        let out = format_feature(&parse(src));
        assert!(out.contains("@foo\n@bar\nFeature: One"), "got:\n{out}");
        assert!(out.contains("  @baz\n  Scenario: A"), "got:\n{out}");
    }

    #[test]
    fn aligns_table_columns() {
        let src = "Feature: T\n\n  Scenario: A\n    Given a table\n      | a | bbbb |\n      | cccc | d |\n";
        let out = format_feature(&parse(src));
        assert!(out.contains("| a    | bbbb |"), "got:\n{out}");
        assert!(out.contains("| cccc | d    |"), "got:\n{out}");
    }

    #[test]
    fn blank_line_before_each_child() {
        let src = "Feature: T\n  Scenario: A\n    Given x\n  Scenario: B\n    Given y\n";
        let out = format_feature(&parse(src));
        assert!(out.contains("Feature: T\n\n  Scenario: A"), "got:\n{out}");
        assert!(out.contains("    Given x\n\n  Scenario: B"), "got:\n{out}");
    }

    #[test]
    fn docstring_has_no_spurious_blank_lines() {
        let src = "Feature: D\n\n  Scenario: A\n    Given step:\n      \"\"\"\n      line1\n      line2\n      \"\"\"\n    Then ok\n";
        let out = format_feature(&parse(src));
        // No blank line immediately after the opening delimiter or before the
        // closing delimiter (the RPC-230 parity bug).
        assert!(out.contains("      \"\"\"\n      line1\n"), "got:\n{out}");
        assert!(out.contains("      line2\n      \"\"\"\n"), "got:\n{out}");
        assert!(!out.contains("\"\"\"\n\n"), "spurious blank after opening, got:\n{out}");
        assert!(!out.contains("\n\n      \"\"\""), "spurious blank before closing, got:\n{out}");
    }

    #[test]
    fn docstring_is_idempotent() {
        let src = "Feature: D\n\n  Scenario: A\n    Given step:\n      \"\"\"\n      line1\n      line2\n      \"\"\"\n    Then ok\n";
        let once = format_feature(&parse(src));
        let twice = format_feature(&parse(&once));
        assert_eq!(once, twice, "docstring formatting must be idempotent");
    }

    #[test]
    fn docstring_media_type_preserved_on_opening_line() {
        let src = "Feature: D\n\n  Scenario: A\n    Given step:\n      \"\"\"json\n      {\"a\": 1}\n      \"\"\"\n    Then ok\n";
        let out = format_feature(&parse(src));
        assert!(out.contains("      \"\"\"json\n"), "media type must stay on opening line, got:\n{out}");
        assert!(out.contains("      {\"a\": 1}\n"), "content must be dedented, got:\n{out}");
        // And it must be idempotent.
        let twice = format_feature(&parse(&out));
        assert_eq!(out, twice, "media-type docstring must be idempotent");
    }
}
