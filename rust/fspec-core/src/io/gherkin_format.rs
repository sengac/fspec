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
//! - **comment preservation** — re-extracted from raw source (the parser discards
//!   them, so we scan the source ourselves)
//!
//! ## Parity caveats (Rust gherkin crate vs `@cucumber/messages`)
//!
//! The Rust `gherkin-0.16.0` AST is structurally leaner than the TS
//! `@cucumber/messages` AST:
//!   - It does NOT retain comments → we extract them from raw source text
//!     (same technique used for `extract_description_verbatim`).
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

/// A comment extracted from the raw source, keyed by its 1-based line number.
struct CommentEntry {
    line: usize,
    text: String,
}

/// Extract all `#` comment lines from `source` into a map keyed by 1-based
/// line number. Mirrors the `ast.comments` array that `@cucumber/gherkin`
/// produces — each entry carries its original text (including indentation).
///
/// The Rust `gherkin-0.16.0` parser consumes comments as whitespace and
/// discards them entirely. We re-extract them from the raw source so the
/// formatter can re-insert them at the correct positions.
fn extract_comments(source: &str) -> Vec<CommentEntry> {
    let mut entries: Vec<CommentEntry> = Vec::new();
    for (idx, line) in source.split('\n').enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            entries.push(CommentEntry {
                line: idx + 1, // 1-based
                text: line.to_string(),
            });
        }
    }
    entries
}

/// Find the 1-based line number of the first tag (`@...`) line in `source`
/// within the range `[start_idx..end_idx]` (0-based, exclusive). Returns
/// `None` when no tag line is found.
///
/// The Rust `gherkin-0.16.0` AST stores tags as plain `String` (just the
/// name), with no position information. We scan the raw source to locate
/// where the tags sit so comments can be inserted before them.
fn find_first_tag_line(source: &str, start_idx: usize, end_idx: usize) -> Option<usize> {
    let lines: Vec<&str> = source.split('\n').collect();
    for (idx, line) in lines.iter().enumerate() {
        if idx < start_idx || idx >= end_idx {
            continue;
        }
        if line.trim_start().starts_with('@') {
            return Some(idx + 1); // 1-based
        }
    }
    None
}

/// Insert all comments whose line number is strictly less than `before_line`,
/// removing them from `comment_map` so they are emitted exactly once.
///
/// Mirrors TS `insertCommentsBeforeLine()`.
fn insert_comments_before(
    before_line: usize,
    comment_map: &mut Vec<CommentEntry>,
    lines: &mut Vec<String>,
) {
    // Iterate in order; comments are already sorted by line number.
    let mut i = 0usize;
    while i < comment_map.len() {
        if comment_map[i].line < before_line {
            lines.push(comment_map[i].text.clone());
            comment_map.remove(i);
            // Don't increment i — next comment shifted into slot i.
        } else {
            i += 1;
        }
    }
}

/// Format a parsed [`Feature`] back to canonical feature-file text with a
/// single trailing newline.
///
/// `source` is the RAW feature-file text the `feature` was parsed from. It is
/// required because the `gherkin-0.16.0` parser collapses the blank lines
/// between description paragraphs (its `description = (description_line ** _)`
/// rule treats blank lines as a *separator* and discards them), so the parsed
/// `Feature.description` field is lossy. To preserve inter-paragraph blank
/// lines we re-extract each feature/scenario/Background/Rule description
/// verbatim from `source` (mirroring
/// `commands::show_acceptance_criteria::extract_description_verbatim`).
///
/// `source` is also used to re-extract comments (which the parser discards)
/// so they are re-inserted at the correct positions during formatting.
pub fn format_feature(feature: &Feature, source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let comments = extract_comments(source);
    let mut comment_map = comments;
    format_feature_into(feature, source, &mut lines, &mut comment_map);
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn indent(level: usize) -> String {
    INDENT.repeat(level)
}

/// Re-extract a description block verbatim from the raw `source`, preserving
/// the inter-paragraph blank lines the parser discarded.
///
/// `start_line` is the 1-based line of the owning header keyword
/// (`feature.position.line` etc.); `end_line_exclusive` is the 1-based line of
/// the first child construct that terminates the description (a step,
/// Scenario, Rule, Examples table, …) when known. Leading and trailing blank
/// lines are stripped (parser parity), and the block ends early at the first
/// comment (`#`) or tag (`@`) line — internal blank lines are kept. Returns
/// `None` when no description text is present.
///
/// Faithful mirror of
/// `commands::show_acceptance_criteria::extract_description_verbatim`; kept
/// local so the `io` layer does not depend on the `commands` layer.
fn extract_description_verbatim(
    source: &str,
    start_line: usize,
    end_line_exclusive: Option<usize>,
) -> Option<String> {
    let lines: Vec<&str> = source.split('\n').collect();
    // 0-based start: `start_line` is the 1-based header line, so index
    // `start_line` is the first line AFTER the header.
    let start_idx = start_line;
    let end_idx_exclusive = end_line_exclusive
        .map(|n| n.saturating_sub(1))
        .unwrap_or(lines.len());
    if start_idx >= end_idx_exclusive || start_idx >= lines.len() {
        return None;
    }
    let slice = &lines[start_idx..end_idx_exclusive.min(lines.len())];

    // Strip leading blank lines (parser `_` consumer skips leading whitespace
    // before the first description line).
    let mut start = 0usize;
    while start < slice.len() && slice[start].chars().all(char::is_whitespace) {
        start += 1;
    }
    // Walk forward and stop at the first comment or tag line — both terminate
    // description blocks in both Gherkin parsers.
    let mut end = start;
    while end < slice.len() {
        let trimmed = slice[end].trim_start();
        if trimmed.starts_with('#') || trimmed.starts_with('@') {
            break;
        }
        end += 1;
    }
    // Strip trailing blank lines (parser `__` consumer trims trailing
    // whitespace/newlines).
    while end > start && slice[end - 1].chars().all(char::is_whitespace) {
        end -= 1;
    }
    if start >= end {
        return None;
    }
    let joined = slice[start..end].join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// Emit a description block: prefer the verbatim text re-extracted from
/// `source` (blank-line preserving) and fall back to the lossy parsed
/// `parsed` field only when raw extraction finds nothing.
fn format_description_block(
    source: &str,
    start_line: usize,
    end_line_exclusive: Option<usize>,
    parsed: Option<&String>,
    lines: &mut Vec<String>,
    indent_level: usize,
) {
    let verbatim = extract_description_verbatim(source, start_line, end_line_exclusive)
        .or_else(|| parsed.cloned());
    if let Some(desc) = &verbatim {
        format_description(desc, lines, indent_level);
    }
}

fn format_feature_into(
    feature: &Feature,
    source: &str,
    lines: &mut Vec<String>,
    comment_map: &mut Vec<CommentEntry>,
) {
    // Insert comments before tags (or before feature keyword if no tags).
    // Tags appear before the feature keyword line. Scan source to find them.
    if !feature.tags.is_empty() {
        let first_tag_line = find_first_tag_line(source, 0, feature.position.line.saturating_sub(1));
        if let Some(tag_line) = first_tag_line {
            insert_comments_before(tag_line, comment_map, lines);
        } else {
            insert_comments_before(feature.position.line, comment_map, lines);
        }
    } else {
        insert_comments_before(feature.position.line, comment_map, lines);
    }

    // Tags — each on its own line (no indentation at feature level).
    for tag in &feature.tags {
        lines.push(format!("@{tag}"));
    }

    // Insert comments before Feature keyword.
    insert_comments_before(feature.position.line, comment_map, lines);

    // Feature keyword and name.
    lines.push(format!("Feature: {}", feature.name));

    // Description (free-form prose under the Feature header, indent level 1),
    // re-extracted verbatim so inter-paragraph blank lines survive. The block
    // ends at the first child construct (Background → Scenario → Rule).
    let feature_desc_end = feature
        .background
        .as_ref()
        .map(|bg| bg.position.line)
        .or_else(|| feature.scenarios.first().map(|s| s.position.line))
        .or_else(|| feature.rules.first().map(|r| r.position.line));
    format_description_block(
        source,
        feature.position.line,
        feature_desc_end,
        feature.description.as_ref(),
        lines,
        1,
    );

    // Children: a Background (if any) then scenarios then rules, in the order
    // the TS formatter walks `feature.children`. The Rust AST splits these
    // into separate typed collections; we emit Background first, then
    // scenarios, then rules — matching the canonical authoring order fspec
    // uses (Background always precedes scenarios; rules follow). TS pushes a
    // blank line before EVERY feature child, so we do the same unconditionally.
    if let Some(bg) = &feature.background {
        push_blank_before_child(lines);
        // Insert comments before Background.
        insert_comments_before(bg.position.line, comment_map, lines);
        // Bound the prose-only Background description by the first sibling
        // construct (BUG-157).
        let next_sibling = feature
            .scenarios
            .first()
            .map(|s| s.position.line)
            .or_else(|| feature.rules.first().map(|r| r.position.line));
        format_background(bg, source, lines, 0, comment_map, next_sibling);
    }

    let scenarios = &feature.scenarios;
    for (idx, scenario) in scenarios.iter().enumerate() {
        push_blank_before_child(lines);
        // Insert comments before scenario (tags or keyword line).
        let scenario_line = if !scenario.tags.is_empty() {
            find_first_tag_line(
                source,
                scenario.position.line.saturating_sub(20),
                scenario.position.line.saturating_sub(1),
            )
            .unwrap_or(scenario.position.line)
        } else {
            scenario.position.line
        };
        insert_comments_before(scenario_line, comment_map, lines);
        // Bound a step-less scenario's description by the next sibling
        // construct (BUG-158).
        let next_sibling = scenarios
            .get(idx + 1)
            .map(|s| s.position.line)
            .or_else(|| feature.rules.first().map(|r| r.position.line));
        format_scenario(scenario, source, lines, 0, comment_map, next_sibling);
    }

    for rule in &feature.rules {
        push_blank_before_child(lines);
        // Insert comments before rule (tags or keyword line).
        let rule_line = if !rule.tags.is_empty() {
            find_first_tag_line(
                source,
                rule.position.line.saturating_sub(20),
                rule.position.line.saturating_sub(1),
            )
            .unwrap_or(rule.position.line)
        } else {
            rule.position.line
        };
        insert_comments_before(rule_line, comment_map, lines);
        format_rule(rule, source, lines, 0, comment_map);
    }
}

/// TS pushes a blank line before EVERY feature child (index 0 and onward).
fn push_blank_before_child(lines: &mut Vec<String>) {
    lines.push(String::new());
}

/// Format a Background block.
///
/// `next_sibling_line` is the 1-based line of the first construct that
/// follows the Background at the same nesting level (the next scenario or
/// rule). It bounds the verbatim description extraction when the Background
/// has no steps of its own: without a bound, a prose-only Background would
/// swallow every trailing scenario into its description and re-emit them
/// nested under the Background (BUG-157).
fn format_background(
    bg: &Background,
    source: &str,
    lines: &mut Vec<String>,
    base_indent: usize,
    comment_map: &mut Vec<CommentEntry>,
    next_sibling_line: Option<usize>,
) {
    let ind = indent(base_indent + 1);
    lines.push(format!("{ind}Background: {}", bg.name));

    // Description ends at the first step of the Background, or — when the
    // Background is prose-only — at the next sibling construct.
    let desc_end = bg
        .steps
        .first()
        .map(|s| s.position.line)
        .or(next_sibling_line);
    format_description_block(
        source,
        bg.position.line,
        desc_end,
        bg.description.as_ref(),
        lines,
        base_indent + 2,
    );

    for step in &bg.steps {
        format_step(step, lines, base_indent + 2, comment_map);
    }
}

fn format_scenario(
    scenario: &Scenario,
    source: &str,
    lines: &mut Vec<String>,
    base_indent: usize,
    comment_map: &mut Vec<CommentEntry>,
    next_sibling_line: Option<usize>,
) {
    let ind = indent(base_indent + 1);

    // Tags.
    for tag in &scenario.tags {
        lines.push(format!("{ind}@{tag}"));
    }

    // Keyword (raw keyword, e.g. "Scenario" or "Scenario Outline").
    lines.push(format!(
        "{ind}{}: {}",
        scenario.keyword.trim(),
        scenario.name
    ));

    // Description ends at the first step (or first Examples table when the
    // scenario has no steps), re-extracted verbatim to keep blank lines.
    // When the scenario has neither steps nor Examples (a step-less
    // scenario — e.g. one whose step lines use lowercase keywords the
    // Rust gherkin parser does not recognize, or genuinely prose-only),
    // the description is bounded by the next sibling construct so the
    // verbatim extraction cannot swallow the trailing scenarios (BUG-158).
    let scenario_desc_end = scenario
        .steps
        .first()
        .map(|s| s.position.line)
        .or_else(|| scenario.examples.first().map(|e| e.position.line))
        .or(next_sibling_line);
    format_description_block(
        source,
        scenario.position.line,
        scenario_desc_end,
        scenario.description.as_ref(),
        lines,
        base_indent + 2,
    );

    for step in &scenario.steps {
        format_step(step, lines, base_indent + 2, comment_map);
    }

    // Examples (Scenario Outline).
    for examples in &scenario.examples {
        lines.push(String::new()); // Blank line before Examples.
        format_examples(examples, lines, base_indent + 2, comment_map);
    }
}

fn format_rule(
    rule: &Rule,
    source: &str,
    lines: &mut Vec<String>,
    base_indent: usize,
    comment_map: &mut Vec<CommentEntry>,
) {
    let ind = indent(base_indent + 1);

    for tag in &rule.tags {
        lines.push(format!("{ind}@{tag}"));
    }

    lines.push(format!("{ind}Rule: {}", rule.name));

    // Description ends at the Rule's first child (Background → Scenario).
    let rule_desc_end = rule
        .background
        .as_ref()
        .map(|bg| bg.position.line)
        .or_else(|| rule.scenarios.first().map(|s| s.position.line));
    format_description_block(
        source,
        rule.position.line,
        rule_desc_end,
        rule.description.as_ref(),
        lines,
        base_indent + 2,
    );

    // Rule children: a Background then scenarios. TS inserts a blank line
    // before each child when `index > 0 || rule.description`.
    let mut idx = 0usize;
    let has_desc = rule.description.is_some();

    if let Some(bg) = &rule.background {
        if idx > 0 || has_desc {
            lines.push(String::new());
        }
        insert_comments_before(bg.position.line, comment_map, lines);
        // Bound the prose-only Background description by the first sibling
        // scenario inside the Rule (BUG-157).
        let next_sibling = rule.scenarios.first().map(|s| s.position.line);
        format_background(bg, source, lines, base_indent + 1, comment_map, next_sibling);
        idx += 1;
    }

    let rule_scenarios = &rule.scenarios;
    for (sidx, scenario) in rule_scenarios.iter().enumerate() {
        if idx > 0 || has_desc {
            lines.push(String::new());
        }
        let scenario_line = if !scenario.tags.is_empty() {
            find_first_tag_line(
                source,
                scenario.position.line.saturating_sub(20),
                scenario.position.line.saturating_sub(1),
            )
            .unwrap_or(scenario.position.line)
        } else {
            scenario.position.line
        };
        insert_comments_before(scenario_line, comment_map, lines);
        // Bound a step-less scenario's description by the next sibling
        // scenario inside the Rule (BUG-158).
        let next_sibling = rule_scenarios.get(sidx + 1).map(|s| s.position.line);
        format_scenario(scenario, source, lines, base_indent + 1, comment_map, next_sibling);
        idx += 1;
    }
}

fn format_step(
    step: &Step,
    lines: &mut Vec<String>,
    indent_level: usize,
    comment_map: &mut Vec<CommentEntry>,
) {
    let ind = indent(indent_level);

    // Insert comments before step.
    insert_comments_before(step.position.line, comment_map, lines);

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

fn format_examples(
    examples: &Examples,
    lines: &mut Vec<String>,
    indent_level: usize,
    comment_map: &mut Vec<CommentEntry>,
) {
    let ind = indent(indent_level);

    // Insert comments before Examples.
    insert_comments_before(examples.position.line, comment_map, lines);

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
        let out = format_feature(&parse(src), src);
        assert!(out.contains("    Given x"), "got:\n{out}");
        assert!(out.contains("    When y"), "got:\n{out}");
        assert!(out.contains("    Then z"), "got:\n{out}");
    }

    #[test]
    fn single_trailing_newline() {
        let src = "Feature: One\n\n  Scenario: A\n    Given x\n";
        let out = format_feature(&parse(src), src);
        assert!(out.ends_with('\n'));
        assert!(!out.ends_with("\n\n"));
    }

    #[test]
    fn idempotent() {
        let src = "Feature: One\n\n  Scenario: A\n  Given x\n  When y\n  Then z\n";
        let once = format_feature(&parse(src), src);
        let twice = format_feature(&parse(&once), &once);
        assert_eq!(once, twice, "format must be idempotent");
    }

    #[test]
    fn preserves_tags() {
        let src = "@foo\n@bar\nFeature: One\n\n  @baz\n  Scenario: A\n    Given x\n";
        let out = format_feature(&parse(src), src);
        assert!(out.contains("@foo\n@bar\nFeature: One"), "got:\n{out}");
        assert!(out.contains("  @baz\n  Scenario: A"), "got:\n{out}");
    }

    #[test]
    fn aligns_table_columns() {
        let src = "Feature: T\n\n  Scenario: A\n    Given a table\n      | a | bbbb |\n      | cccc | d |\n";
        let out = format_feature(&parse(src), src);
        assert!(out.contains("| a    | bbbb |"), "got:\n{out}");
        assert!(out.contains("| cccc | d    |"), "got:\n{out}");
    }

    #[test]
    fn blank_line_before_each_child() {
        let src = "Feature: T\n  Scenario: A\n    Given x\n  Scenario: B\n    Given y\n";
        let out = format_feature(&parse(src), src);
        assert!(out.contains("Feature: T\n\n  Scenario: A"), "got:\n{out}");
        assert!(out.contains("    Given x\n\n  Scenario: B"), "got:\n{out}");
    }

    #[test]
    fn docstring_has_no_spurious_blank_lines() {
        let src = "Feature: D\n\n  Scenario: A\n    Given step:\n      \"\"\"\n      line1\n      line2\n      \"\"\"\n    Then ok\n";
        let out = format_feature(&parse(src), src);
        // No blank line immediately after the opening delimiter or before the
        // closing delimiter (the RPC-230 parity bug).
        assert!(out.contains("      \"\"\"\n      line1\n"), "got:\n{out}");
        assert!(out.contains("      line2\n      \"\"\"\n"), "got:\n{out}");
        assert!(
            !out.contains("\"\"\"\n\n"),
            "spurious blank after opening, got:\n{out}"
        );
        assert!(
            !out.contains("\n\n      \"\"\""),
            "spurious blank before closing, got:\n{out}"
        );
    }

    #[test]
    fn docstring_is_idempotent() {
        let src = "Feature: D\n\n  Scenario: A\n    Given step:\n      \"\"\"\n      line1\n      line2\n      \"\"\"\n    Then ok\n";
        let once = format_feature(&parse(src), src);
        let twice = format_feature(&parse(&once), &once);
        assert_eq!(once, twice, "docstring formatting must be idempotent");
    }

    #[test]
    fn docstring_media_type_preserved_on_opening_line() {
        let src = "Feature: D\n\n  Scenario: A\n    Given step:\n      \"\"\"json\n      {\"a\": 1}\n      \"\"\"\n    Then ok\n";
        let out = format_feature(&parse(src), src);
        assert!(
            out.contains("      \"\"\"json\n"),
            "media type must stay on opening line, got:\n{out}"
        );
        assert!(
            out.contains("      {\"a\": 1}\n"),
            "content must be dedented, got:\n{out}"
        );
        // And it must be idempotent.
        let twice = format_feature(&parse(&out), &out);
        assert_eq!(out, twice, "media-type docstring must be idempotent");
    }

    // ====================================================================
    // RPC-330: Gherkin Description Blank-Line Preservation in Formatter
    //
    // Feature: spec/features/gherkin-description-formatting.feature
    //
    // These tests assert that `fspec format` (exercised here through the
    // formatter entry point `format_feature` over a parsed source) preserves
    // blank lines between paragraphs inside feature/scenario descriptions.
    // They are RED until the formatter re-extracts description text from the
    // raw source instead of the lossy parsed `description` field.
    // ====================================================================

    #[test]
    fn rpc330_preserves_blank_line_between_feature_description_paragraphs() {
        // @step Given a feature file whose Feature header is followed by this description:
        let src = "Feature: Multi paragraph\n\n  First paragraph of the feature description.\n\n  Second paragraph of the feature description.\n\n  Scenario: A\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the feature description retains exactly one blank line between the two paragraphs:
        assert_eq!(
            out,
            "Feature: Multi paragraph\n  First paragraph of the feature description.\n\n  Second paragraph of the feature description.\n\n  Scenario: A\n    Given x\n",
            "full formatted output must match TS-parity layout (no leading blank after Feature header, inter-paragraph blank preserved), got:\n{out}"
        );
    }

    #[test]
    fn rpc330_preserves_blank_line_between_scenario_description_paragraphs() {
        // @step Given a feature file with a scenario whose header is followed by this description:
        let src = "Feature: Scenario desc\n\n  Scenario: Has prose\n    First paragraph of the scenario description.\n\n    Second paragraph of the scenario description.\n\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the scenario description retains exactly one blank line between the two paragraphs:
        assert!(
            out.contains(
                "    First paragraph of the scenario description.\n\n    Second paragraph of the scenario description.\n"
            ),
            "blank line between scenario description paragraphs was dropped, got:\n{out}"
        );
    }

    #[test]
    fn rpc330_single_paragraph_description_is_unchanged() {
        // @step Given a feature file with a single-line feature description and no internal blank lines:
        let src =
            "Feature: One line desc\n\n  Only one paragraph here.\n\n  Scenario: A\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the description is emitted with no leading or internal blank line and is byte-identical to the input layout:
        assert_eq!(
            out,
            "Feature: One line desc\n  Only one paragraph here.\n\n  Scenario: A\n    Given x\n",
            "single-paragraph description must have no leading/internal blank line, got:\n{out}"
        );
    }

    #[test]
    fn rpc330_collapses_more_than_two_blank_lines_to_two() {
        // @step Given a feature file whose feature description separates "Paragraph one." and "Paragraph two." by four consecutive blank lines
        let src = "Feature: Excessive blanks\n\n  Paragraph one.\n\n\n\n\n  Paragraph two.\n\n  Scenario: A\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the two paragraphs are separated by exactly two blank lines:
        assert!(
            out.contains("  Paragraph one.\n\n\n  Paragraph two.\n"),
            "four blank lines must collapse to exactly two, got:\n{out}"
        );
        assert!(
            !out.contains("  Paragraph one.\n\n\n\n  Paragraph two.\n"),
            "must not emit three or more blank lines between paragraphs, got:\n{out}"
        );
    }

    #[test]
    fn rpc330_step_docstring_with_internal_blank_line_is_not_regressed() {
        // @step Given a feature file with a step doc string whose body contains a blank line:
        let src = "Feature: Docstring intact\n\n  Scenario: A\n    Given step:\n      \"\"\"\n      line one\n\n      line three\n      \"\"\"\n    Then ok\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the step doc string body is emitted unchanged with no spurious blanks added or removed:
        assert!(
            out.contains("      \"\"\"\n      line one\n\n      line three\n      \"\"\"\n"),
            "step doc string body must be preserved exactly, got:\n{out}"
        );
    }

    #[test]
    fn rpc330_multi_paragraph_description_is_idempotent() {
        // @step Given a feature file with a two-paragraph feature description that has already been formatted once
        let src = "Feature: Multi paragraph\n\n  First paragraph of the feature description.\n\n  Second paragraph of the feature description.\n\n  Scenario: A\n    Given x\n";
        let once = format_feature(&parse(src), src);

        // @step When the formatter formats the feature file a second time
        let twice = format_feature(&parse(&once), &once);

        // @step Then the output of the second run is byte-identical to the output of the first run
        assert_eq!(
            once, twice,
            "multi-paragraph description formatting must be idempotent"
        );
    }

    // ====================================================================
    // Comment Preservation
    //
    // The Rust gherkin-0.16.0 parser discards comments entirely. We re-extract
    // them from raw source text and re-insert them at the correct positions
    // during formatting, mirroring the TypeScript `commentMap` machinery.
    // ====================================================================

    #[test]
    fn preserves_comment_before_feature_keyword() {
        // @step Given a feature file with a comment before the Feature keyword
        let src = "# This is a comment\nFeature: One\n\n  Scenario: A\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the comment is preserved in the output
        assert!(
            out.contains("# This is a comment"),
            "comment before feature was lost, got:\n{out}"
        );
        // Comment must appear before the Feature keyword
        let comment_pos = out.find("# This is a comment").expect("comment must exist");
        let feature_pos = out.find("Feature: One").expect("feature must exist");
        assert!(
            comment_pos < feature_pos,
            "comment must appear before Feature keyword, got:\n{out}"
        );
    }

    #[test]
    fn preserves_comment_block_before_scenario() {
        // @step Given a feature file with a comment block before a Scenario
        let src = "Feature: One\n\n# Comment before scenario\n  Scenario: A\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the comment block is preserved in the output
        assert!(
            out.contains("# Comment before scenario"),
            "comment block before scenario was lost, got:\n{out}"
        );
    }

    #[test]
    fn preserves_multiple_comments_before_steps() {
        // @step Given a feature file with comments between steps
        let src = "Feature: One\n\n  Scenario: A\n    Given x\n# Comment before When\n    When y\n    Then z\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then all comments are preserved in the output
        assert!(
            out.contains("# Comment before When"),
            "comment before step was lost, got:\n{out}"
        );
    }

    #[test]
    fn preserves_example_mapping_comment_block() {
        // @step Given a feature file with an EXAMPLE MAPPING CONTEXT comment block
        let src = "Feature: Example Mapping\n\n# EXAMPLE MAPPING CONTEXT\n# Rules:\n#   1. Password must be 8+ characters\n# Examples:\n#   1. User enters valid credentials\n\n  Scenario: Login\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the entire comment block is preserved
        assert!(
            out.contains("# EXAMPLE MAPPING CONTEXT"),
            "EXAMPLE MAPPING CONTEXT header was lost, got:\n{out}"
        );
        assert!(
            out.contains("# Rules:"),
            "Rules comment was lost, got:\n{out}"
        );
        assert!(
            out.contains("#   1. Password must be 8+ characters"),
            "Rule detail comment was lost, got:\n{out}"
        );
        assert!(
            out.contains("# Examples:"),
            "Examples comment was lost, got:\n{out}"
        );
        assert!(
            out.contains("#   1. User enters valid credentials"),
            "Example detail comment was lost, got:\n{out}"
        );
    }

    #[test]
    fn preserves_indented_comments_before_steps() {
        // @step Given a feature file with indented comments before steps
        let src = "Feature: One\n\n  Scenario: A\n    # Indented comment\n    Given x\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the indented comment is preserved with its original indentation
        assert!(
            out.contains("    # Indented comment"),
            "indented comment was lost or had wrong indentation, got:\n{out}"
        );
    }

    // ====================================================================
    // BUG-157: Background section must not duplicate scenarios
    //
    // Feature: spec/features/gherkin-background-formatting.feature
    //
    // The gherkin-0.16.0 parser stores Background prose in
    // `Background.description`, so the formatter must NOT re-extract the
    // Background description verbatim from raw source without bounding the
    // extraction by the Background's own content. An unbounded extraction
    // swallows the trailing top-level scenarios and re-emits them nested
    // under the Background, duplicating every scenario block.
    // ====================================================================

    /// Feature: spec/features/gherkin-background-formatting.feature
    #[test]
    fn bug157_background_prose_does_not_duplicate_scenarios() {
        // @step Given a feature file with a Background section followed by two scenarios
        let src = "@test\nFeature: Formatter Background duplication repro\n\n  Background: User Story\n    As a user\n    I want to test\n    So that I can verify\n\n  Scenario: First scenario\n    Given a precondition\n    When I do the action\n    Then I see the result\n\n  Scenario: Second scenario\n    Given another precondition\n    When I do another action\n    Then I see another result\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the formatted output contains exactly two Scenario lines
        let scenario_count = out
            .lines()
            .filter(|l| l.trim_start().starts_with("Scenario:"))
            .count();
        assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");

        // @step And every scenario is indented at the top level (two spaces)
        for line in out.lines() {
            if line.trim_start().starts_with("Scenario:") {
                assert_eq!(
                    line,
                    format!("  {}", line.trim_start()),
                    "scenario must be 2-space top-level indented, got:\n{out}"
                );
            }
        }
    }

    /// Feature: spec/features/gherkin-background-formatting.feature
    #[test]
    fn bug157_background_prose_formatting_is_idempotent() {
        // @step Given a feature file with a Background section and two scenarios
        let src = "Feature: Background idempotent\n\n  Background: User Story\n    As a user\n    I want to test\n    So that I can verify\n\n  Scenario: First scenario\n    Given a precondition\n\n  Scenario: Second scenario\n    Given another precondition\n";
        let once = format_feature(&parse(src), src);

        // @step When the formatter formats the file twice
        let twice = format_feature(&parse(&once), &once);

        // @step Then the output of the second run is byte-identical to the output of the first run
        assert_eq!(once, twice, "formatting must be idempotent, got:\n{once}\n---\n{twice}");
    }

    /// Feature: spec/features/gherkin-background-formatting.feature
    #[test]
    fn bug157_background_with_steps_is_formatted() {
        // @step Given a feature file with a Background section containing a Given step and one scenario
        let src = "Feature: Background with steps\n\n  Background: Setup\n    Given the app is loaded\n\n  Scenario: A scenario\n    Given a precondition\n    Then I see the result\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the Background step is emitted under the Background
        assert!(
            out.contains("  Background: Setup\n    Given the app is loaded"),
            "background step missing, got:\n{out}"
        );

        // @step And the scenario is emitted exactly once at the top level
        let scenario_count = out
            .lines()
            .filter(|l| l.trim_start().starts_with("Scenario:"))
            .count();
        assert_eq!(scenario_count, 1, "scenario was duplicated, got:\n{out}");
        assert!(
            out.contains("  Scenario: A scenario"),
            "scenario must be 2-space indented, got:\n{out}"
        );
    }

    /// Feature: spec/features/gherkin-background-formatting.feature
    #[test]
    fn bug157_rule_with_background_is_formatted_without_duplication() {
        // @step Given a feature file with a Rule containing a Background and one scenario
        let src = "Feature: Rule with background\n\n  Rule: My rule\n    Background: Setup\n      Given the app is loaded\n\n    Scenario: A scenario\n      Given a precondition\n      Then I see the result\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the scenario is emitted exactly once
        let scenario_count = out
            .lines()
            .filter(|l| l.trim_start().starts_with("Scenario:"))
            .count();
        assert_eq!(scenario_count, 1, "scenario was duplicated, got:\n{out}");

        // @step And the Background step is emitted under the Rule's Background
        assert!(
            out.contains("    Background: Setup\n      Given the app is loaded"),
            "rule background step missing, got:\n{out}"
        );
    }

    /// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
    #[test]
    fn bug158_stepless_scenario_does_not_duplicate_scenarios() {
        // @step Given a feature file where the first scenario is prose-only (no steps) followed by a second scenario with steps
        let src = "Feature: Step-less scenario\n\n  Scenario: First\n    just some prose here\n\n  Scenario: Second\n    Given a precondition\n    When I do the action\n    Then I see the result\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the formatted output contains exactly two Scenario lines
        let scenario_count = out
            .lines()
            .filter(|l| l.trim_start().starts_with("Scenario:"))
            .count();
        assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");

        // @step And every scenario is indented at the top level (two spaces)
        for line in out.lines() {
            if line.trim_start().starts_with("Scenario:") {
                assert_eq!(
                    line,
                    format!("  {}", line.trim_start()),
                    "scenario must be 2-space top-level indented, got:\n{out}"
                );
            }
        }
    }

    /// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
    #[test]
    fn bug158_stepless_scenario_formatting_is_idempotent() {
        // @step Given a feature file where the first scenario is prose-only (no steps) followed by a second scenario
        let src = "Feature: Step-less idempotent\n\n  Scenario: First\n    just some prose here\n\n  Scenario: Second\n    Given a precondition\n    When I do the action\n    Then I see the result\n";
        let once = format_feature(&parse(src), src);

        // @step When the formatter formats the file twice
        let twice = format_feature(&parse(&once), &once);

        // @step Then the output of the second run is byte-identical to the output of the first run
        assert_eq!(once, twice, "formatting must be idempotent, got:\n{once}\n---\n{twice}");
    }

    /// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
    #[test]
    fn bug158_lowercase_step_keywords_are_formatted_without_duplication() {
        // @step Given a feature file whose scenarios use lowercase step keywords (given, when, then)
        let src = "Feature: Lowercase keywords\n\n  Scenario: One\n    given a precondition\n    when I do the action\n    then I see the result\n\n  Scenario: Two\n    given another precondition\n    when I do another action\n    then I see another result\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then the formatted output contains exactly the original number of Scenario lines
        let scenario_count = out
            .lines()
            .filter(|l| l.trim_start().starts_with("Scenario:"))
            .count();
        assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");

        // @step And the lowercase step lines are preserved verbatim under their scenario
        assert!(
            out.contains("    given a precondition"),
            "lowercase step line was lost or rewritten, got:\n{out}"
        );
        assert!(
            out.contains("    then I see another result"),
            "lowercase step line was lost or rewritten, got:\n{out}"
        );
    }

    /// Feature: spec/features/gherkin-stepless-scenario-formatting.feature
    #[test]
    fn bug158_rule_with_stepless_scenario_is_formatted_without_duplication() {
        // @step Given a feature file with a Rule containing a prose-only scenario followed by a second scenario
        let src = "Feature: Rule with step-less scenario\n\n  Rule: My rule\n    Scenario: First\n      just some prose here\n\n    Scenario: Second\n      Given a precondition\n      Then I see the result\n";

        // @step When the formatter formats the feature file
        let out = format_feature(&parse(src), src);

        // @step Then both scenarios are emitted exactly once at the Rule nesting level
        let scenario_count = out
            .lines()
            .filter(|l| l.trim_start().starts_with("Scenario:"))
            .count();
        assert_eq!(scenario_count, 2, "scenarios were duplicated, got:\n{out}");
        assert!(
            out.contains("    Scenario: First"),
            "first scenario must be 4-space Rule-nested, got:\n{out}"
        );
        assert!(
            out.contains("    Scenario: Second"),
            "second scenario must be 4-space Rule-nested, got:\n{out}"
        );
    }

    #[test]
    fn comment_preservation_is_idempotent() {
        // @step Given a feature file with comments that has been formatted once
        let src = "# Header comment\nFeature: One\n\n# Before scenario\n  Scenario: A\n    # Before step\n    Given x\n";
        let once = format_feature(&parse(src), src);

        // @step When the formatter formats the output a second time
        let twice = format_feature(&parse(&once), &once);

        // @step Then the second formatting is byte-identical to the first
        assert_eq!(once, twice, "comment preservation must be idempotent");
    }
}
