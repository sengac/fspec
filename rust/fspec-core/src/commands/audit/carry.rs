//! BUG-635: `fspec audit --fix` carries an implementation mapping through the
//! git line diff from the commit that linked it.
//!
//! Feature: spec/features/audit-fix-carries-implementation-mappings-through-the-git-line-diff.feature
//!
//! The diff is between the file at that commit (read from git) and the file
//! the audit reads (BUG-666: [`line_diff`], Myers, in-process, so a run needs
//! no git process per file). Hunks follow `git diff -U0`'s convention, so the
//! carry is exact and language-agnostic: an unchanged mapped line moves by the lines
//! the hunks before it added or removed; lines a hunk inserted or changed
//! strictly inside a contiguous mapped run join the mapping; lines a hunk
//! deleted or changed leave it; lines inserted before the mapping's first or
//! after its last line stay outside. A hunk that changes or deletes the
//! mapping's first or last line cannot be carried exactly, so it is reported.

/// One `@@ -old_start,old_count +new_start,new_count @@` hunk. For a pure
/// insertion (`old_count` 0) `old_start` is the line it follows; for a pure
/// deletion (`new_count` 0) `new_start` is the line before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hunk {
    pub(crate) old_start: i64,
    pub(crate) old_count: i64,
    pub(crate) new_start: i64,
    pub(crate) new_count: i64,
}

impl Hunk {
    /// The last old line the hunk changes or deletes (only when
    /// `old_count` > 0).
    fn old_end(&self) -> i64 {
        self.old_start + self.old_count - 1
    }

    /// Whether the hunk lies wholly before old line `line`.
    fn before(&self, line: i64) -> bool {
        if self.old_count == 0 {
            self.old_start < line
        } else {
            self.old_end() < line
        }
    }

    /// Whether the hunk changes or deletes old line `line`.
    fn touches(&self, line: i64) -> bool {
        self.old_count > 0 && self.old_start <= line && line <= self.old_end()
    }
}

/// BUG-666: the line diff (Myers, minimal) from `old` to `new` as hunks, in
/// the `git diff -U0` convention: a pure insertion's `old_start` is the line
/// it follows, a pure deletion's `new_start` the line before it.
pub(crate) fn line_diff(old: &str, new: &str) -> Vec<Hunk> {
    use similar::{Algorithm, DiffOp};
    let old: Vec<&str> = old.lines().collect();
    let new: Vec<&str> = new.lines().collect();
    let as_line = |index: usize| i64::try_from(index).unwrap_or(i64::MAX);
    similar::capture_diff_slices(Algorithm::Myers, &old, &new)
        .into_iter()
        .filter_map(|op| match op {
            DiffOp::Equal { .. } => None,
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => Some(Hunk {
                old_start: as_line(old_index),
                old_count: 0,
                new_start: as_line(new_index) + 1,
                new_count: as_line(new_len),
            }),
            DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => Some(Hunk {
                old_start: as_line(old_index) + 1,
                old_count: as_line(old_len),
                new_start: as_line(new_index),
                new_count: 0,
            }),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => Some(Hunk {
                old_start: as_line(old_index) + 1,
                old_count: as_line(old_len),
                new_start: as_line(new_index) + 1,
                new_count: as_line(new_len),
            }),
        })
        .collect()
}

/// `lines` as contiguous runs `(first, last)`, ascending.
fn runs(lines: &[i64]) -> Vec<(i64, i64)> {
    let mut sorted = lines.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut runs: Vec<(i64, i64)> = Vec::new();
    for line in sorted {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == line => *last = line,
            _ => runs.push((line, line)),
        }
    }
    runs
}

/// `lines` (a mapping at the old version) carried through `hunks` to the new
/// version, or why that cannot be done exactly.
pub(crate) fn carry(lines: &[i64], hunks: &[Hunk]) -> Result<Vec<i64>, String> {
    let (Some(first), Some(last)) = (lines.iter().min(), lines.iter().max()) else {
        return Err("the mapping lists no lines".to_string());
    };
    for hunk in hunks {
        if hunk.touches(*first) {
            return Err(format!("an edit changed its first line {first}"));
        }
        if hunk.touches(*last) {
            return Err(format!("an edit changed its last line {last}"));
        }
    }
    let mut carried = Vec::new();
    for (run_first, run_last) in runs(lines) {
        for line in run_first..=run_last {
            if hunks.iter().any(|hunk| hunk.touches(line)) {
                continue; // changed or deleted: it leaves the mapping
            }
            let shift: i64 = hunks
                .iter()
                .filter(|hunk| hunk.before(line))
                .map(|hunk| hunk.new_count - hunk.old_count)
                .sum();
            carried.push(line + shift);
        }
        for hunk in hunks.iter().filter(|hunk| hunk.new_count > 0) {
            let inside = if hunk.old_count == 0 {
                run_first <= hunk.old_start && hunk.old_start < run_last
            } else {
                run_first < hunk.old_start && hunk.old_end() < run_last
            };
            if inside {
                carried.extend(hunk.new_start..hunk.new_start + hunk.new_count);
            }
        }
    }
    carried.sort_unstable();
    carried.dedup();
    Ok(carried)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn range(from: i64, to: i64) -> Vec<i64> {
        (from..=to).collect()
    }

    #[test]
    fn insertions_before_inside_and_at_the_edges() {
        let before = Hunk {
            old_start: 4,
            old_count: 0,
            new_start: 5,
            new_count: 3,
        };
        assert_eq!(carry(&range(10, 24), &[before]).unwrap(), range(13, 27));
        let inside = Hunk {
            old_start: 15,
            old_count: 0,
            new_start: 19,
            new_count: 1,
        };
        assert_eq!(
            carry(&range(10, 24), &[before, inside]).unwrap(),
            range(13, 28)
        );
        let at_start = Hunk {
            old_start: 9,
            old_count: 0,
            new_start: 10,
            new_count: 2,
        };
        assert_eq!(carry(&range(10, 24), &[at_start]).unwrap(), range(12, 26));
        let at_end = Hunk {
            old_start: 24,
            old_count: 0,
            new_start: 25,
            new_count: 2,
        };
        assert_eq!(carry(&range(10, 24), &[at_end]).unwrap(), range(10, 24));
    }

    #[test]
    fn deletions_and_edits_inside_and_at_the_edges() {
        let deleted = Hunk {
            old_start: 15,
            old_count: 2,
            new_start: 14,
            new_count: 0,
        };
        assert_eq!(carry(&range(10, 24), &[deleted]).unwrap(), range(10, 22));
        let changed = Hunk {
            old_start: 15,
            old_count: 1,
            new_start: 15,
            new_count: 3,
        };
        assert_eq!(carry(&range(10, 24), &[changed]).unwrap(), range(10, 26));
        let first = Hunk {
            old_start: 10,
            old_count: 1,
            new_start: 10,
            new_count: 1,
        };
        assert_eq!(
            carry(&range(10, 24), &[first]).unwrap_err(),
            "an edit changed its first line 10"
        );
        let last = Hunk {
            old_start: 24,
            old_count: 1,
            new_start: 24,
            new_count: 0,
        };
        assert_eq!(
            carry(&range(10, 24), &[last]).unwrap_err(),
            "an edit changed its last line 24"
        );
    }

    #[test]
    fn line_diff_reports_insertions_edits_and_deletions() {
        let old = "a\nb\nc\nd\n";
        assert_eq!(
            line_diff(old, "a\nb\nx\nc\nd\n"),
            vec![Hunk {
                old_start: 2,
                old_count: 0,
                new_start: 3,
                new_count: 1
            }]
        );
        assert_eq!(
            line_diff(old, "a\nB\nc\nd\n"),
            vec![Hunk {
                old_start: 2,
                old_count: 1,
                new_start: 2,
                new_count: 1
            }]
        );
        assert_eq!(
            line_diff(old, "a\nd\n"),
            vec![Hunk {
                old_start: 2,
                old_count: 2,
                new_start: 1,
                new_count: 0
            }]
        );
        assert!(line_diff(old, old).is_empty());
    }

    #[test]
    fn separate_runs_keep_their_gaps() {
        let mut lines = range(1, 5);
        lines.extend(range(10, 12));
        let between = Hunk {
            old_start: 7,
            old_count: 0,
            new_start: 8,
            new_count: 4,
        };
        let mut expected = range(1, 5);
        expected.extend(range(14, 16));
        assert_eq!(carry(&lines, &[between]).unwrap(), expected);
    }
}
