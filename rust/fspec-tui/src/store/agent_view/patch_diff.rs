use super::diff_format::{with_tree_connectors, DiffDisplayRow, DIFF_COLLAPSED_LINES};
use crate::terminal::sanitize::sanitize_for_terminal;

pub fn capture_patch_diff(patch: &str) -> Option<Vec<DiffDisplayRow>> {
    let mut lines = patch.lines();
    if lines.next()? != "*** Begin Patch" {
        return None;
    }
    let mut rows = Vec::new();
    let mut line_no = 1;
    let mut in_file = false;
    for line in lines {
        if line == "*** End Patch" {
            return (!rows.is_empty()).then_some(rows);
        }
        if let Some(path) = line.strip_prefix("*** Delete File: ") {
            rows.push(DiffDisplayRow::Elision {
                text: sanitize_for_terminal(line),
            });
            if let Ok(content) = std::fs::read_to_string(path) {
                rows.extend(content.lines().enumerate().map(|(index, text)| {
                    DiffDisplayRow::Removed {
                        line_no: index + 1,
                        text: sanitize_for_terminal(text),
                    }
                }));
            }
            in_file = false;
            continue;
        }
        if line.starts_with("*** Add File: ") || line.starts_with("*** Update File: ") {
            rows.push(DiffDisplayRow::Elision {
                text: sanitize_for_terminal(line),
            });
            line_no = 1;
            in_file = true;
            continue;
        }
        if line.starts_with("@@") || line == "*** End of File" {
            rows.push(DiffDisplayRow::Elision {
                text: sanitize_for_terminal(line),
            });
            continue;
        }
        if !in_file {
            return None;
        }
        let row = if let Some(text) = line.strip_prefix('+') {
            DiffDisplayRow::Added {
                line_no,
                text: sanitize_for_terminal(text),
            }
        } else if let Some(text) = line.strip_prefix('-') {
            DiffDisplayRow::Removed {
                line_no,
                text: sanitize_for_terminal(text),
            }
        } else {
            DiffDisplayRow::Context {
                line_no,
                text: sanitize_for_terminal(line.strip_prefix(' ').unwrap_or(line)),
            }
        };
        rows.push(row);
        line_no += 1;
    }
    None
}

pub fn produce_patch_diff(rows: &[DiffDisplayRow]) -> (String, String) {
    let full = with_tree_connectors(rows);
    if rows.len() <= DIFF_COLLAPSED_LINES {
        return (full.clone(), full);
    }
    let mut collapsed = rows[..DIFF_COLLAPSED_LINES].to_vec();
    collapsed.push(DiffDisplayRow::Elision {
        text: format!(
            "... +{} lines (select turn to /expand)",
            rows.len() - DIFF_COLLAPSED_LINES
        ),
    });
    (with_tree_connectors(&collapsed), full)
}
