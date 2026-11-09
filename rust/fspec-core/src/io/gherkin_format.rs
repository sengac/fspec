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
    let mut docstring = DocstringTracker::default();
    for (idx, line) in source.split('\n').enumerate() {
        // BUG-479: a `#` line inside a `"""` / ``` block is text, not a
        // comment (step docstrings and fspec's description blocks).
        if docstring.is_text(line) {
            continue;
        }
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

/// Tracks `"""` / ``` docstring blocks line by line (BUG-479). A line that
/// opens or closes a block, and every line between, is docstring text.
#[derive(Default)]
struct DocstringTracker {
    open: Option<&'static str>,
}

impl DocstringTracker {
    /// Feed the next line; true when it belongs to a docstring block.
    fn is_text(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start();
        match self.open {
            Some(delimiter) => {
                if trimmed.starts_with(delimiter) {
                    self.open = None;
                }
                true
            }
            None => {
                for delimiter in ["\"\"\"", "```"] {
                    if trimmed.starts_with(delimiter) {
                        self.open = Some(delimiter);
                        return true;
                    }
                }
                false
            }
        }
    }
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
/// verbatim from `source` (see
/// [`extract_description_verbatim`]).
///
/// `source` is also used to re-extract comments (which the parser discards)
/// so they are re-inserted at the correct positions during formatting.
pub fn format_feature(feature: &Feature, source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let comments = extract_comments(source);
    let mut comment_map = comments;
    format_feature_into(feature, source, &mut lines, &mut comment_map);
    // BUG-479: comments after the last construct stay at the end of the file,
    // separated by one blank line like every feature child.
    if !comment_map.is_empty() {
        lines.push(String::new());
        lines.extend(comment_map.drain(..).map(|c| c.text));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// `<indent><keyword>: <name>`, or `<indent><keyword>:` with no trailing
/// space when the name is empty (BUG-479).
fn keyword_line(ind: &str, keyword: &str, name: &str) -> String {
    if name.is_empty() {
        format!("{ind}{keyword}:")
    } else {
        format!("{ind}{keyword}: {name}")
    }
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
/// The one verbatim description extractor: the formatter and CLI-019's
/// `fspec context` (through [`feature_header_lines`] and [`scenario_lines`])
/// both read descriptions through it.
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
    // description blocks in both Gherkin parsers — unless it sits inside a
    // `"""` / ``` block, where it is text (BUG-479).
    let mut end = start;
    let mut docstring = DocstringTracker::default();
    while end < slice.len() {
        if docstring.is_text(slice[end]) {
            end += 1;
            continue;
        }
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
        let first_tag_line =
            find_first_tag_line(source, 0, feature.position.line.saturating_sub(1));
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
    lines.push(keyword_line("", "Feature", &feature.name));

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
    lines.push(keyword_line(&ind, "Background", &bg.name));

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
    lines.push(keyword_line(&ind, scenario.keyword.trim(), &scenario.name));

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

    lines.push(keyword_line(&ind, "Rule", &rule.name));

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
        format_background(
            bg,
            source,
            lines,
            base_indent + 1,
            comment_map,
            next_sibling,
        );
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
        format_scenario(
            scenario,
            source,
            lines,
            base_indent + 1,
            comment_map,
            next_sibling,
        );
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

/// CLI-019: the canonical lines of a feature's header without its scenarios
/// and rules: the tags on one line, `Feature: <name>`, the verbatim
/// description and the Background. Source comments are not included.
pub(crate) fn feature_header_lines(feature: &Feature, source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    if !feature.tags.is_empty() {
        lines.push(tag_line(&feature.tags));
    }
    lines.push(keyword_line("", "Feature", &feature.name));
    let first_child = feature
        .scenarios
        .first()
        .map(|s| s.position.line)
        .into_iter()
        .chain(feature.rules.first().map(|r| r.position.line))
        .min();
    let desc_end = feature
        .background
        .as_ref()
        .map(|bg| bg.position.line)
        .or(first_child);
    format_description_block(
        source,
        feature.position.line,
        desc_end,
        feature.description.as_ref(),
        &mut lines,
        1,
    );
    if let Some(bg) = &feature.background {
        lines.push(String::new());
        format_background(bg, source, &mut lines, 0, &mut Vec::new(), first_child);
    }
    lines
}

/// CLI-019: the canonical lines of one scenario (its tags, keyword line,
/// description, steps with doc strings and tables, and Examples). A scenario
/// of a Rule is nested under the Rule's tags, `Rule: <name>` line and
/// Background; any other scenario starts at column 0. `next_sibling_line`
/// bounds the description of a step-less scenario (BUG-158).
pub(crate) fn scenario_lines(
    scenario: &Scenario,
    rule: Option<&Rule>,
    source: &str,
    next_sibling_line: Option<usize>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(rule) = rule else {
        format_scenario(
            scenario,
            source,
            &mut lines,
            0,
            &mut Vec::new(),
            next_sibling_line,
        );
        return lines
            .into_iter()
            .map(|line| {
                line.strip_prefix(INDENT)
                    .map_or(line.clone(), str::to_string)
            })
            .collect();
    };
    if !rule.tags.is_empty() {
        lines.push(tag_line(&rule.tags));
    }
    lines.push(keyword_line("", "Rule", &rule.name));
    if let Some(bg) = &rule.background {
        lines.push(String::new());
        let first = rule.scenarios.first().map(|s| s.position.line);
        format_background(bg, source, &mut lines, 0, &mut Vec::new(), first);
    }
    lines.push(String::new());
    format_scenario(
        scenario,
        source,
        &mut lines,
        0,
        &mut Vec::new(),
        next_sibling_line,
    );
    lines
}

/// `@a @b` for the parsed tags `a` and `b` (the parser strips the `@`).
fn tag_line(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| format!("@{tag}"))
        .collect::<Vec<_>>()
        .join(" ")
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
}

#[cfg(test)]
mod description_tests;

#[cfg(test)]
mod background_tests;

#[cfg(test)]
mod stepless_scenario_tests;

#[cfg(test)]
mod format_rules_tests;

#[cfg(test)]
mod comment_tests;
