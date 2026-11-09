//! Implementation mappings in `fspec audit`'s coverage area.
//!
//! Feature: spec/features/audit-fix-carries-implementation-mappings-through-the-git-line-diff.feature
//! Feature: spec/features/audit-reports-a-mapping-whose-range-changed-under-matching-anchor-lines.feature
//!
//! BUG-515: every implementation mapping lists at least one line, and every
//! line lies between 1 and the last line of its (existing) file.
//!
//! COV-079: a mapping records the trimmed text of its lowest and highest line
//! (`firstLine`, `lastLine`), so a mapping that still lies inside its file but
//! now points at other code is detected exactly. BUG-635: `--fix` carries
//! such a mapping through the git line diff from the commit that linked it
//! ([`History::carry`]); an edit to its first or last line, or a mapping git
//! cannot carry, is reported with the reason (this replaced COV-079's shift
//! by one offset, which left a range one line short after an insertion).
//! BUG-666: a committed mapping whose anchors still read their text is
//! carried too, and reported as `impl-range-changed` when its range changed
//! under them. A
//! mapping without recorded text gets it recorded by `--fix` when its first
//! line is non-blank and unique and (BUG-591) git shows its lines did not
//! move since the commit that linked it ([`super::history`]); otherwise the
//! problem names why. Only line text is compared, never symbols.

use std::collections::HashMap;
use std::path::Path;

use super::drift::line_ranges;
use super::history::{Carried, History, Mapping, Verified};
use super::source::Source;
use crate::error::FspecCoreError;
use crate::types::coverage::{CoverageFile, ImplLines};

/// The mechanical repair of an implementation mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ImplRepair {
    /// Move every line of the mapping to `new_lines`.
    Shift { new_lines: Vec<i64> },
    /// Record the text of the mapping's first and last line.
    Record {
        first_line: String,
        last_line: String,
    },
}

/// An implementation mapping problem.
pub(crate) struct ImplRangeProblem {
    pub(crate) scenario: String,
    /// `impl-unreadable-range`, `impl-no-lines`, `impl-outside-file`,
    /// `impl-text-changed`, `impl-range-changed` (BUG-666) or
    /// `impl-text-unrecorded`; the mapping's test file
    /// and lines, implementation file and lines, its location and the fact
    /// without the location.
    pub(crate) code: &'static str,
    pub(crate) test_file: String,
    pub(crate) test_lines: String,
    pub(crate) impl_file: String,
    pub(crate) lines: Vec<i64>,
    pub(crate) location: String,
    pub(crate) fact: String,
    /// The exact repair `--fix` applies, when there is one.
    pub(crate) repair: Option<ImplRepair>,
}

/// COV-079: the trimmed text of the first and last of `lines` in a file with
/// `content`, refusing a first line that is blank or occurs more than once.
/// `file` names the file in the error.
pub(crate) fn record_text(
    file: &str,
    content: &str,
    lines: &[i64],
) -> Result<(String, String), String> {
    let text: Vec<&str> = content.lines().map(str::trim).collect();
    let (Some(first), Some(last)) = (lines.iter().min(), lines.iter().max()) else {
        return Err(format!("{file}: the mapping lists no lines"));
    };
    let at = |line: i64| {
        usize::try_from(line)
            .ok()
            .filter(|line| *line >= 1)
            .and_then(|line| text.get(line - 1))
            .copied()
    };
    let (Some(first_text), Some(last_text)) = (at(*first), at(*last)) else {
        return Err(format!(
            "{file}: lines {first}-{last} lie outside the file ({} lines)",
            text.len()
        ));
    };
    if first_text.is_empty() {
        return Err(format!(
            "{file} line {first} is blank; start the implementation mapping at a distinctive line such as a signature"
        ));
    }
    let count = text.iter().filter(|line| **line == first_text).count();
    if count > 1 {
        return Err(format!(
            "{file} line {first} ('{first_text}') occurs {count} times; start the implementation mapping at a distinctive line such as a signature"
        ));
    }
    Ok((first_text.to_string(), last_text.to_string()))
}

/// The line numbers of an implementation mapping's `lines`; `None` when a
/// range string does not parse.
pub(crate) fn impl_lines(lines: &ImplLines) -> Option<Vec<i64>> {
    match lines {
        ImplLines::Array(lines) => Some(lines.clone()),
        ImplLines::String(range) => line_ranges(range).map(|ranges| {
            ranges
                .into_iter()
                .flat_map(|(start, end)| start..=end)
                .filter_map(|line| i64::try_from(line).ok())
                .collect()
        }),
    }
}

/// Every implementation mapping of `coverage` whose lines are unreadable,
/// empty or outside its file (BUG-515), or whose recorded text no longer
/// matches or was never recorded (COV-079). Mappings whose file is missing
/// are left to the existence check.
/// `coverage_file` is the coverage file's path; BUG-591: `history` answers
/// whether a legacy mapping's lines did not move.
pub(crate) fn impl_range_problems(
    project_root: &Path,
    source: &Source,
    coverage_file: &str,
    coverage: &CoverageFile,
    history: &History,
) -> Result<Vec<ImplRangeProblem>, FspecCoreError> {
    let mut contents: HashMap<&str, String> = HashMap::new();
    let mut problems = Vec::new();
    for scenario in &coverage.scenarios {
        for (test_mapping, impl_mapping) in scenario.test_mappings.iter().flat_map(|mapping| {
            mapping
                .impl_mappings
                .iter()
                .map(move |impl_mapping| (mapping, impl_mapping))
        }) {
            let file = impl_mapping.file.as_str();
            if !source.exists(project_root, file) {
                continue; // the existence check reports it
            }
            let lines = impl_lines(&impl_mapping.lines).unwrap_or_default();
            let problem = |code: &'static str, location: String, fact: String| ImplRangeProblem {
                scenario: scenario.name.clone(),
                code,
                test_file: test_mapping.file.clone(),
                test_lines: test_mapping.lines.clone(),
                impl_file: file.to_string(),
                lines: lines.clone(),
                location,
                fact,
                repair: None,
            };
            if let ImplLines::String(range) = &impl_mapping.lines {
                if line_ranges(range).is_none() {
                    problems.push(problem(
                        "impl-unreadable-range",
                        file.to_string(),
                        format!("has an unreadable line range '{range}'"),
                    ));
                    continue;
                }
            }
            if lines.is_empty() {
                problems.push(problem(
                    "impl-no-lines",
                    file.to_string(),
                    "lists no lines".to_string(),
                ));
                continue;
            }
            if !contents.contains_key(file) {
                let content = source.read(project_root, file)?.unwrap_or_default();
                contents.insert(file, content);
            }
            let content = contents.get(file).map(String::as_str).unwrap_or_default();
            let text: Vec<&str> = content.lines().map(str::trim).collect();
            let count = text.len();
            let location = format!("{file}:{}", compact(&lines));
            let outside = |line: &i64| usize::try_from(*line).map_or(true, |l| l < 1 || l > count);
            if lines.iter().any(outside) {
                problems.push(problem(
                    "impl-outside-file",
                    location,
                    format!("lies outside the file ({count} lines)"),
                ));
                continue;
            }
            match (&impl_mapping.first_line, &impl_mapping.last_line) {
                (Some(first_line), Some(last_line)) => {
                    let first = lines.iter().min().copied().unwrap_or(1);
                    let last = lines.iter().max().copied().unwrap_or(1);
                    let at = |line: i64| {
                        text.get(usize::try_from(line - 1).unwrap_or(usize::MAX))
                            .copied()
                    };
                    let mapping = Mapping {
                        scenario: &scenario.name,
                        test_file: &test_mapping.file,
                        impl_file: file,
                        lines: &lines,
                    };
                    if at(first) == Some(first_line.as_str())
                        && at(last) == Some(last_line.as_str())
                    {
                        // BUG-666: the anchors still read their text, but the
                        // range may have grown or shrunk under them.
                        let change = history.range_change(
                            project_root,
                            coverage_file,
                            &mapping,
                            first_line,
                            last_line,
                            content,
                            &text,
                        )?;
                        let Some(change) = change else {
                            continue;
                        };
                        let commit_fact =
                            "the line diff since the commit that linked it changed the range";
                        let mut found = problem("impl-range-changed", location, String::new());
                        match change {
                            Carried::Lines(new_lines) => {
                                found.fact = format!(
                                    "{commit_fact}: lines {} now hold it",
                                    compact(&new_lines)
                                );
                                found.repair = Some(ImplRepair::Shift { new_lines });
                            }
                            Carried::Reported(reason) => {
                                found.fact = format!("{commit_fact}; {reason}");
                            }
                        }
                        problems.push(found);
                        continue;
                    }
                    let mut found = problem(
                        "impl-text-changed",
                        location,
                        format!("lines {first} and {last} no longer read '{first_line}' and '{last_line}'"),
                    );
                    // BUG-635: carry it through the line diff from the
                    // commit that linked it; otherwise name why not.
                    match history.carry(
                        project_root,
                        coverage_file,
                        &mapping,
                        first_line,
                        last_line,
                        content,
                        &text,
                    )? {
                        Carried::Lines(new_lines) => {
                            found.repair = Some(ImplRepair::Shift { new_lines });
                        }
                        Carried::Reported(reason) => {
                            found.fact = format!("{}; {reason}", found.fact);
                        }
                    }
                    problems.push(found);
                }
                _ => {
                    let mut found = problem(
                        "impl-text-unrecorded",
                        location,
                        "the mapping records no first and last line text".to_string(),
                    );
                    if let Ok((first_line, last_line)) = record_text(file, content, &lines) {
                        // BUG-591: record only lines git shows did not move.
                        let mapping = Mapping {
                            scenario: &scenario.name,
                            test_file: &test_mapping.file,
                            impl_file: file,
                            lines: &lines,
                        };
                        match history.verify(project_root, coverage_file, &mapping, content)? {
                            Verified::Unmoved => {
                                found.repair = Some(ImplRepair::Record {
                                    first_line,
                                    last_line,
                                });
                            }
                            Verified::Unverified(reason) => {
                                found.fact = format!("{}; {reason}", found.fact);
                            }
                        }
                    }
                    problems.push(found);
                }
            }
        }
    }
    Ok(problems)
}

/// `[18, 19, 20, 5]` → `"5,18-20"`.
pub(crate) fn compact(lines: &[i64]) -> String {
    let mut sorted = lines.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut parts: Vec<String> = Vec::new();
    let mut index = 0;
    while index < sorted.len() {
        let start = sorted[index];
        let mut end = start;
        while index + 1 < sorted.len() && sorted[index + 1] == end + 1 {
            index += 1;
            end = sorted[index];
        }
        parts.push(if start == end {
            start.to_string()
        } else {
            format!("{start}-{end}")
        });
        index += 1;
    }
    parts.join(",")
}
