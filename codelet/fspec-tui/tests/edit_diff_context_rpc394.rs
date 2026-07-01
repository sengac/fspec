//! Feature: spec/features/edit-diff-surrounding-file-context.feature
//!
//! RPC-394 — integration coverage for the context-aware Edit diff builder.
//! `build_edit_diff_rows_with_context` reads the POST-EDIT file and injects up
//! to `CONTEXT_LINES` (3) real unchanged file lines BEFORE and AFTER the
//! changed region as `Context` rows. File-based scenarios use a real temp file
//! (std temp dir + a unique name, no new dependency) and assert on the produced
//! `Vec<DiffDisplayRow>`.
//!
//! Every Gherkin step carries a matching `// @step` comment whose text mirrors
//! the feature file exactly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use codelet_fspec_tui::store::agent_view::diff_format::{
    build_edit_diff_rows_with_context, DiffDisplayRow, DIFF_COLLAPSED_LINES,
};

/// Allocate a unique temp file path (no `tempfile` dep needed) and write
/// `content` to it. Returns the path; the file is left for the OS to reap.
fn write_temp_file(content: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let mut path = std::env::temp_dir();
    path.push(format!("rpc394_ctx_{pid}_{n}.txt"));
    fs::write(&path, content).expect("write temp file");
    path
}

fn line_no(row: &DiffDisplayRow) -> Option<usize> {
    match row {
        DiffDisplayRow::Context { line_no, .. }
        | DiffDisplayRow::Removed { line_no, .. }
        | DiffDisplayRow::Added { line_no, .. } => Some(*line_no),
        DiffDisplayRow::Elision { .. } => None,
    }
}

fn row_text(row: &DiffDisplayRow) -> &str {
    match row {
        DiffDisplayRow::Context { text, .. }
        | DiffDisplayRow::Removed { text, .. }
        | DiffDisplayRow::Added { text, .. }
        | DiffDisplayRow::Elision { text } => text,
    }
}

fn context_rows(rows: &[DiffDisplayRow]) -> Vec<&DiffDisplayRow> {
    rows.iter()
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .collect()
}

fn removed_rows(rows: &[DiffDisplayRow]) -> Vec<&DiffDisplayRow> {
    rows.iter()
        .filter(|r| matches!(r, DiffDisplayRow::Removed { .. }))
        .collect()
}

fn added_rows(rows: &[DiffDisplayRow]) -> Vec<&DiffDisplayRow> {
    rows.iter()
        .filter(|r| matches!(r, DiffDisplayRow::Added { .. }))
        .collect()
}

#[test]
fn mid_file_edit_shows_three_context_lines_above_and_below() {
    // @step Given a fifty-line file whose lines ten and eleven are replaced by two entirely different lines
    // Build a 50-line file; the POST-EDIT file has lines 10 & 11 replaced.
    let old_string = "line10\nline11";
    let new_string = "REPLACED-A\nREPLACED-B";
    let post_edit: String = (1..=50)
        .map(|n| match n {
            10 => "REPLACED-A\n".to_string(),
            11 => "REPLACED-B\n".to_string(),
            _ => format!("line{n}\n"),
        })
        .collect();
    let path = write_temp_file(&post_edit);

    // @step When I build the context-aware edit diff rows for that edit
    let rows = build_edit_diff_rows_with_context(
        old_string,
        new_string,
        Some(path.to_str().unwrap()),
        DIFF_COLLAPSED_LINES,
    );

    // @step Then three unchanged file lines immediately above the change appear as gray context rows
    let before: Vec<&DiffDisplayRow> = rows
        .iter()
        .take_while(|r| {
            !matches!(
                r,
                DiffDisplayRow::Removed { .. } | DiffDisplayRow::Added { .. }
            )
        })
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .collect();
    assert_eq!(before.len(), 3, "three before-context rows");
    assert_eq!(row_text(before[0]), "line7");
    assert_eq!(line_no(before[0]), Some(7));
    assert_eq!(row_text(before[1]), "line8");
    assert_eq!(line_no(before[1]), Some(8));
    assert_eq!(row_text(before[2]), "line9");
    assert_eq!(line_no(before[2]), Some(9));

    // @step And the two old lines appear as removed rows and the two new lines appear as added rows
    let removed = removed_rows(&rows);
    let added = added_rows(&rows);
    assert_eq!(removed.len(), 2);
    assert_eq!(row_text(removed[0]), "line10");
    assert_eq!(row_text(removed[1]), "line11");
    assert_eq!(added.len(), 2);
    assert_eq!(row_text(added[0]), "REPLACED-A");
    assert_eq!(row_text(added[1]), "REPLACED-B");

    // @step And three unchanged file lines immediately below the change appear as gray context rows
    let after: Vec<&DiffDisplayRow> = rows
        .iter()
        .skip_while(|r| !matches!(r, DiffDisplayRow::Added { .. }))
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .collect();
    assert_eq!(after.len(), 3, "three after-context rows");
    assert_eq!(row_text(after[0]), "line12");
    assert_eq!(line_no(after[0]), Some(12));
    assert_eq!(row_text(after[1]), "line13");
    assert_eq!(line_no(after[1]), Some(13));
    assert_eq!(row_text(after[2]), "line14");
    assert_eq!(line_no(after[2]), Some(14));
}

#[test]
fn edit_on_first_line_has_no_before_context_and_trailing_context() {
    // @step Given a file whose first line is replaced by a different line
    let old_string = "line1";
    let new_string = "REPLACED-1";
    let post_edit: String = (1..=10)
        .map(|n| {
            if n == 1 {
                "REPLACED-1\n".to_string()
            } else {
                format!("line{n}\n")
            }
        })
        .collect();
    let path = write_temp_file(&post_edit);

    // @step When I build the context-aware edit diff rows for that edit
    let rows = build_edit_diff_rows_with_context(
        old_string,
        new_string,
        Some(path.to_str().unwrap()),
        DIFF_COLLAPSED_LINES,
    );

    // @step Then no context row appears before the change
    let first_change = rows
        .iter()
        .position(|r| {
            matches!(
                r,
                DiffDisplayRow::Removed { .. } | DiffDisplayRow::Added { .. }
            )
        })
        .expect("a change row");
    let before_context = rows[..first_change]
        .iter()
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .count();
    assert_eq!(before_context, 0, "no before-context on first-line edit");

    // @step And up to three unchanged file lines below the change appear as gray context rows
    let after: Vec<&DiffDisplayRow> = rows
        .iter()
        .skip_while(|r| !matches!(r, DiffDisplayRow::Added { .. }))
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .collect();
    assert_eq!(after.len(), 3);
    assert_eq!(row_text(after[0]), "line2");
    assert_eq!(line_no(after[0]), Some(2));
    assert_eq!(row_text(after[2]), "line4");
    assert_eq!(line_no(after[2]), Some(4));
}

#[test]
fn edit_on_last_line_has_leading_context_and_no_after_context() {
    // @step Given a file whose last line is replaced by a different line
    let old_string = "line10";
    let new_string = "REPLACED-10";
    let post_edit: String = (1..=10)
        .map(|n| {
            if n == 10 {
                "REPLACED-10\n".to_string()
            } else {
                format!("line{n}\n")
            }
        })
        .collect();
    let path = write_temp_file(&post_edit);

    // @step When I build the context-aware edit diff rows for that edit
    let rows = build_edit_diff_rows_with_context(
        old_string,
        new_string,
        Some(path.to_str().unwrap()),
        DIFF_COLLAPSED_LINES,
    );

    // @step Then up to three unchanged file lines above the change appear as gray context rows
    let before: Vec<&DiffDisplayRow> = rows
        .iter()
        .take_while(|r| {
            !matches!(
                r,
                DiffDisplayRow::Removed { .. } | DiffDisplayRow::Added { .. }
            )
        })
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .collect();
    assert_eq!(before.len(), 3);
    assert_eq!(row_text(before[0]), "line7");
    assert_eq!(line_no(before[0]), Some(7));
    assert_eq!(row_text(before[2]), "line9");
    assert_eq!(line_no(before[2]), Some(9));

    // @step And no context row appears after the change
    let last_change = rows
        .iter()
        .rposition(|r| {
            matches!(
                r,
                DiffDisplayRow::Removed { .. } | DiffDisplayRow::Added { .. }
            )
        })
        .expect("a change row");
    let after_context = rows[last_change + 1..]
        .iter()
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .count();
    assert_eq!(after_context, 0, "no after-context on last-line edit");
}

#[test]
fn missing_file_falls_back_to_fragments_only_with_no_panic() {
    // @step Given an edit whose file path does not exist on disk
    let old_string = "alpha\nbeta";
    let new_string = "gamma\ndelta";
    let missing = "/nonexistent/rpc394/does/not/exist.txt";

    // @step When I build the context-aware edit diff rows for that edit
    let rows = build_edit_diff_rows_with_context(
        old_string,
        new_string,
        Some(missing),
        DIFF_COLLAPSED_LINES,
    );

    // @step Then the rows contain only the removed and added fragment lines with no injected context rows
    assert_eq!(context_rows(&rows).len(), 0, "no injected context rows");
    let removed = removed_rows(&rows);
    let added = added_rows(&rows);
    assert_eq!(removed.len(), 2);
    assert_eq!(added.len(), 2);
    assert_eq!(row_text(removed[0]), "alpha");
    assert_eq!(row_text(removed[1]), "beta");
    assert_eq!(row_text(added[0]), "gamma");
    assert_eq!(row_text(added[1]), "delta");

    // @step And no panic occurs
    // (reaching this line proves no panic occurred)
}

#[test]
fn write_of_a_new_three_line_file_shows_only_added_rows_and_no_context() {
    // @step Given a Write of a brand-new three-line file
    // A Write is all-additions: its old_string is empty, so even the
    // context-aware builder must inject no context (no surrounding file lines).
    let content = "one\ntwo\nthree";
    let rows = build_edit_diff_rows_with_context("", content, None, DIFF_COLLAPSED_LINES);

    // @step When I build the diff rows for that write
    // (built above)

    // @step Then the output contains three added rows
    let added = added_rows(&rows);
    assert_eq!(added.len(), 3);
    assert_eq!(row_text(added[0]), "one");
    assert_eq!(row_text(added[1]), "two");
    assert_eq!(row_text(added[2]), "three");

    // @step And no context row appears in the output
    assert_eq!(context_rows(&rows).len(), 0, "no context rows for a write");
}

#[test]
fn shared_boundary_line_is_shown_once_and_not_duplicated() {
    // @step Given an edit whose old and new strings share an unchanged middle line inside a larger file
    // The change block sits around line 10 of a 30-line file: the middle line
    // "KEEP" is unchanged while the lines around it ("X"/"Y" → "A"/"B") change.
    let old_string = "X\nKEEP\nY";
    let new_string = "A\nKEEP\nB";
    let post_edit: String = (1..=30)
        .map(|n| match n {
            10 => "A\n".to_string(),
            11 => "KEEP\n".to_string(),
            12 => "B\n".to_string(),
            _ => format!("line{n}\n"),
        })
        .collect();
    let path = write_temp_file(&post_edit);

    // @step When I build the context-aware edit diff rows for that edit
    let rows = build_edit_diff_rows_with_context(
        old_string,
        new_string,
        Some(path.to_str().unwrap()),
        DIFF_COLLAPSED_LINES,
    );

    // @step Then the shared line appears exactly once as a gray context row
    let keep_context = context_rows(&rows)
        .iter()
        .filter(|r| row_text(r) == "KEEP")
        .count();
    assert_eq!(
        keep_context, 1,
        "the shared 'KEEP' line must appear exactly once as a Context row"
    );
    let removed = removed_rows(&rows);
    let added = added_rows(&rows);
    assert!(removed.iter().any(|r| row_text(r) == "X"));
    assert!(removed.iter().any(|r| row_text(r) == "Y"));
    assert!(added.iter().any(|r| row_text(r) == "A"));
    assert!(added.iter().any(|r| row_text(r) == "B"));

    // @step And no injected after-context row duplicates the shared line
    let last_added = rows
        .iter()
        .rposition(|r| matches!(r, DiffDisplayRow::Added { .. }))
        .expect("an added row");
    let after_context_has_keep = rows[last_added + 1..]
        .iter()
        .filter(|r| matches!(r, DiffDisplayRow::Context { .. }))
        .any(|r| row_text(r) == "KEEP");
    assert!(
        !after_context_has_keep,
        "injected after-context must not duplicate the shared 'KEEP' line"
    );
}
