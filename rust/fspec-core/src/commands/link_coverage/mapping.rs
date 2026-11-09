//! Mapping operations for `link-coverage` (parity with mapping-ops.ts).

use serde_json::{json, Value};

use super::invalid;
use crate::error::FspecCoreError;

// ─────────────────────────────────────────────────────────────────────────
// Mapping operations (parity with mapping-ops.ts)
// ─────────────────────────────────────────────────────────────────────────

/// Append a test mapping (allowing multiples for the same file).
pub(super) fn add_test_mapping(entry: &mut Value, test_file: &str, test_lines: &str) -> String {
    // BUG-557: the same file and range is mapped once; linking it again
    // adds nothing.
    if exact_row(entry, test_file, test_lines).is_some() {
        return already_mapped(test_file, test_lines);
    }
    push_test_mapping(
        entry,
        json!({
            "file": test_file,
            "lines": test_lines,
            "implMappings": [],
        }),
    );
    let count = count_test_mappings_for(entry, test_file);
    if count > 1 {
        format!("✓ Added second test mapping for {test_file}:{test_lines}")
    } else {
        format!("✓ Linked test mapping: {test_file}:{test_lines}")
    }
}

/// BUG-557: the scenario's test mapping with exactly this file and lines
/// text, if it has one.
fn exact_row(entry: &Value, test_file: &str, test_lines: &str) -> Option<usize> {
    entry
        .get("testMappings")
        .and_then(Value::as_array)?
        .iter()
        .position(|row| {
            row.get("file").and_then(Value::as_str) == Some(test_file)
                && row.get("lines").and_then(Value::as_str) == Some(test_lines)
        })
}

fn already_mapped(test_file: &str, test_lines: &str) -> String {
    format!("✓ {test_file}:{test_lines} is already mapped; no duplicate added")
}

/// Add an implementation mapping to an existing test mapping (smart append).
pub(super) fn add_impl_mapping(
    entry: &mut Value,
    test_file: &str,
    impl_file: &str,
    impl_lines: &str,
    text: Option<&ImplText>,
) -> Result<String, FspecCoreError> {
    let test_mappings = entry
        .get_mut("testMappings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            invalid(format!(
                "Test mapping not found: {test_file}\nSuggestion: Link the test file first using --test-file and --test-lines"
            ))
        })?;
    let tm = test_mappings
        .iter_mut()
        .find(|tm| tm.get("file").and_then(Value::as_str) == Some(test_file));
    let tm = match tm {
        Some(tm) => tm,
        None => {
            return Err(invalid(format!(
                "Test mapping not found: {test_file}\nSuggestion: Link the test file first using --test-file and --test-lines"
            )));
        }
    };
    add_impl_to_row(tm, impl_file, impl_lines, text)
}

/// Smart-append an implementation mapping to one exact test-mapping row.
/// COV-079: the trimmed text of an implementation mapping's first and last
/// line (`firstLine`, `lastLine`).
pub(super) type ImplText = (String, String);

/// An implementation mapping object: file, lines and (COV-079) the recorded
/// first and last line text.
fn impl_object(impl_file: &str, parsed: &[i64], text: Option<&ImplText>) -> Value {
    let mut object = json!({ "file": impl_file, "lines": parsed });
    if let Some((first, last)) = text {
        object["firstLine"] = json!(first);
        object["lastLine"] = json!(last);
    }
    object
}

pub(super) fn add_impl_to_row(
    tm: &mut Value,
    impl_file: &str,
    impl_lines: &str,
    text: Option<&ImplText>,
) -> Result<String, FspecCoreError> {
    let parsed = parse_impl_lines(impl_lines);
    let impl_mappings = tm
        .as_object_mut()
        .map(|o| o.entry("implMappings").or_insert_with(|| json!([])))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("implMappings is not an array".to_string()))?;

    if let Some(existing) = impl_mappings
        .iter_mut()
        .find(|im| im.get("file").and_then(Value::as_str) == Some(impl_file))
    {
        // Only the lines and their recorded text change; other fields stay.
        existing["lines"] = json!(parsed);
        if let Some(object) = existing.as_object_mut() {
            match text {
                Some((first, last)) => {
                    object.insert("firstLine".to_string(), json!(first));
                    object.insert("lastLine".to_string(), json!(last));
                }
                None => {
                    object.remove("firstLine");
                    object.remove("lastLine");
                }
            }
        }
        Ok(format!(
            "✓ Updated implementation mapping: {impl_file}:{impl_lines}"
        ))
    } else {
        impl_mappings.push(impl_object(impl_file, &parsed, text));
        Ok(format!(
            "✓ Added implementation mapping: {impl_file}:{impl_lines}"
        ))
    }
}

/// Append a test mapping carrying a single implementation mapping, or
/// (BUG-557) add the implementation to the scenario's existing mapping of the
/// same file and range.
pub(super) fn add_both_mappings(
    entry: &mut Value,
    test_file: &str,
    test_lines: &str,
    impl_file: &str,
    impl_lines: &str,
    text: Option<&ImplText>,
) -> Result<String, FspecCoreError> {
    if let Some(index) = exact_row(entry, test_file, test_lines) {
        let row = entry
            .get_mut("testMappings")
            .and_then(Value::as_array_mut)
            .and_then(|rows| rows.get_mut(index))
            .ok_or_else(|| invalid("the existing test mapping disappeared".to_string()))?;
        let added = add_impl_to_row(row, impl_file, impl_lines, text)?;
        return Ok(format!(
            "{}\n{added}",
            already_mapped(test_file, test_lines)
        ));
    }
    let parsed = parse_impl_lines(impl_lines);
    push_test_mapping(
        entry,
        json!({
            "file": test_file,
            "lines": test_lines,
            "implMappings": [ impl_object(impl_file, &parsed, text) ],
        }),
    );
    Ok(format!("✓ Linked test mapping with implementation: {test_file}:{test_lines} → {impl_file}:{impl_lines}"))
}

/// BUG-502: the outcome of [`replace_test_mapping`].
pub(crate) struct Replaced {
    /// Index of the replacement row in `testMappings`.
    pub(super) index: usize,
    pub(super) message: String,
}

/// Replace every mapping of `test_file` in the scenario with one mapping
/// holding `test_lines` that carries all of their implementation mappings
/// (one per implementation file; lines of a file mapped twice are merged).
/// The first replaced row keeps its position and any other fields. `None`
/// when the scenario has no mapping of `test_file`.
pub(super) fn replace_test_mapping(
    entry: &mut Value,
    test_file: &str,
    test_lines: &str,
) -> Option<Replaced> {
    let rows = entry
        .get_mut("testMappings")
        .and_then(Value::as_array_mut)?;
    let index = rows
        .iter()
        .position(|row| row.get("file").and_then(Value::as_str) == Some(test_file))?;
    let mut replaced = 0usize;
    let mut impls: Vec<Value> = Vec::new();
    let mut first: Option<Value> = None;
    rows.retain(|row| {
        if row.get("file").and_then(Value::as_str) != Some(test_file) {
            return true;
        }
        replaced += 1;
        for im in row
            .get("implMappings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            merge_impl(&mut impls, im);
        }
        if first.is_none() {
            first = Some(row.clone());
        }
        false
    });
    let mut row = first?;
    row["lines"] = json!(test_lines);
    row["implMappings"] = Value::Array(impls);
    let kept = row["implMappings"].as_array().map_or(0, Vec::len);
    rows.insert(index, row);
    Some(Replaced {
        index,
        message: format!(
            "✓ Replaced {replaced} test mapping(s) of {test_file} with {test_file}:{test_lines} (kept {kept} implementation mapping(s))"
        ),
    })
}

/// COV-078: [`replace_test_mapping`] onto `new_file` (a test that moved to
/// another file): the scenario's mappings of `old_file` become one mapping of
/// `new_file` at `test_lines`, keeping their implementation mappings. `None`
/// when the scenario has no mapping of `old_file`, or (for a move) already
/// maps `new_file`.
pub(crate) fn relink_test_mapping(
    entry: &mut Value,
    old_file: &str,
    new_file: &str,
    test_lines: &str,
) -> Option<Replaced> {
    if new_file != old_file
        && entry
            .get("testMappings")
            .and_then(Value::as_array)?
            .iter()
            .any(|row| row.get("file").and_then(Value::as_str) == Some(new_file))
    {
        return None;
    }
    let replaced = replace_test_mapping(entry, old_file, test_lines)?;
    entry["testMappings"][replaced.index]["file"] = json!(new_file);
    Some(replaced)
}

/// Add `im` to `impls`, merging its lines into an existing mapping of the
/// same implementation file.
fn merge_impl(impls: &mut Vec<Value>, im: &Value) {
    let file = im.get("file").and_then(Value::as_str);
    let Some(existing) = impls
        .iter_mut()
        .find(|known| known.get("file").and_then(Value::as_str) == file)
    else {
        impls.push(im.clone());
        return;
    };
    let mut lines = impl_line_numbers(existing.get("lines"));
    lines.extend(impl_line_numbers(im.get("lines")));
    lines.sort_unstable();
    lines.dedup();
    existing["lines"] = json!(lines);
}

/// An implementation mapping's `lines` (number array or range string) as
/// line numbers.
fn impl_line_numbers(lines: Option<&Value>) -> Vec<i64> {
    match lines {
        Some(Value::Array(numbers)) => numbers.iter().filter_map(Value::as_i64).collect(),
        Some(Value::String(range)) => parse_impl_lines(range),
        _ => Vec::new(),
    }
}

pub(super) fn push_test_mapping(entry: &mut Value, mapping: Value) {
    if let Some(obj) = entry.as_object_mut() {
        let arr = obj.entry("testMappings").or_insert_with(|| json!([]));
        if let Some(a) = arr.as_array_mut() {
            a.push(mapping);
        }
    }
}

pub(super) fn count_test_mappings_for(entry: &Value, test_file: &str) -> usize {
    entry
        .get("testMappings")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|tm| tm.get("file").and_then(Value::as_str) == Some(test_file))
                .count()
        })
        .unwrap_or(0)
}

/// Parse impl lines: comma-separated individual lines, ranges, and mixed.
/// "10-15" → [10..=15], "10,11,12" → [10,11,12], "89-99,316-405" → expanded.
pub(super) fn parse_impl_lines(impl_lines: &str) -> Vec<i64> {
    let mut result = Vec::new();
    for segment in impl_lines.split(',') {
        let segment = segment.trim();
        if let Some((s, e)) = segment.split_once('-') {
            if let (Ok(start), Ok(end)) = (s.trim().parse::<i64>(), e.trim().parse::<i64>()) {
                let mut i = start;
                while i <= end {
                    result.push(i);
                    i += 1;
                }
            }
        } else if let Ok(num) = segment.parse::<i64>() {
            result.push(num);
        }
    }
    result
}

/// The "To remove this mapping" hint appended to the success message. The TS
/// uses `chalk.gray`, but chalk auto-disables ANSI when stdout is not a TTY
/// (the dispatcher and piped-CLI paths), so we emit plain text for parity with
/// the common non-TTY case.
pub(super) fn removal_hint(feature_name: &str, scenario: &str, test_file: Option<&str>) -> String {
    let tf = test_file
        .map(|t| format!(" --test-file {t}"))
        .unwrap_or_default();
    format!(
        "\n\nTo remove this mapping:\n  fspec unlink-coverage {feature_name} --scenario \"{scenario}\"{tf}"
    )
}
