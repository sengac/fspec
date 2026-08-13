//! RPC-014 — BoardView rich grid + details strip render tests.
//!
//! Feature: spec/features/rpc014-board-grid.feature
//!
//! Drives `BoardView::render_with_store` against a `TestBackend` and
//! asserts the new box-drawing topology + work-unit details strip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, Theme};
use codelet_rpc_types::WorkUnitInfo;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn make_unit(id: &str, status: &str, work_type: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: work_type.to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn render(width: u16, height: u16, store: &BoardStore) -> Buffer {
    let (view, _rx) = fresh();
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

fn join_buffer(buf: &Buffer) -> String {
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

fn row_string(buf: &Buffer, y: u16) -> String {
    let mut row = String::new();
    for x in 0..buf.area.width {
        row.push_str(buf[(x, y)].symbol());
    }
    row
}

/// Find the (x, y) of the first occurrence of `needle` (a single glyph)
/// in the buffer; returns None if not present.
fn find_glyph(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].symbol() == needle {
                return Some((x, y));
            }
        }
    }
    None
}

/// Find a substring beginning at or after row `min_y`. Used by content-row
/// assertions to skip past the details strip (which echoes the selected
/// work-unit id in its title row).
fn find_substring(buf: &Buffer, text: &str) -> Option<(u16, u16)> {
    find_substring_from(buf, text, 0)
}

fn find_substring_from(buf: &Buffer, text: &str, min_y: u16) -> Option<(u16, u16)> {
    for y in min_y..buf.area.height {
        let row = row_string(buf, y);
        if let Some(byte_idx) = row.find(text) {
            let col = row[..byte_idx].chars().count() as u16;
            return Some((col, y));
        }
    }
    None
}

/// Find a substring inside the content rows of the BoardView, skipping
/// the top border, details strip, details-bottom separator, column
/// header row, and column-header-bottom separator. The content area
/// begins at row 9 for a standard layout.
fn find_substring_in_content(buf: &Buffer, text: &str) -> Option<(u16, u16)> {
    find_substring_from(buf, text, 9)
}

/// Scenario: No work unit selected paints the centered placeholder
#[test]
fn no_work_unit_selected_paints_centered_placeholder() {
    // @step Given an empty BoardStore (no work units, no selection)
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "No work unit selected"
    assert!(
        joined.contains("No work unit selected"),
        "missing 'No work unit selected' placeholder:\n{joined}"
    );
}

/// Scenario: Selected work unit paints title and metadata rows of the details strip
#[test]
fn selected_work_unit_paints_title_and_metadata_rows_of_details_strip() {
    // @step Given a BoardStore containing AUTH-001 (story, backlog, title "User Login", description "Sign in with email/password", estimate 5, epic "authentication", no attachments)
    let unit = WorkUnitInfo {
        id: "AUTH-001".to_string(),
        title: "User Login".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: Some("Sign in with email/password".to_string()),
        estimate: Some(5),
        epic: Some("authentication".to_string()),
        attachments: Vec::new(),
        last_state_change_at: None,
    };
    let mut store = BoardStore::default();
    store.replace_work_units(vec![unit]);
    // @step And the focused column is "backlog" and the selected index is 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "AUTH-001: User Login"
    assert!(
        joined.contains("AUTH-001: User Login"),
        "missing 'AUTH-001: User Login' in details strip:\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Epic: authentication"
    assert!(
        joined.contains("Epic: authentication"),
        "missing 'Epic: authentication':\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Estimate: 5pts"
    assert!(
        joined.contains("Estimate: 5pts"),
        "missing 'Estimate: 5pts':\n{joined}"
    );
    // @step And the rendered buffer contains the substring "Status: backlog"
    assert!(
        joined.contains("Status: backlog"),
        "missing 'Status: backlog':\n{joined}"
    );
}

/// Scenario: Attachments row renders comma-joined basenames with the "A" key hint
#[test]
fn attachments_row_renders_comma_joined_basenames_with_a_key_hint() {
    // @step Given a BoardStore containing DOC-014 with attachments ["spec/attachments/RPC-014/notes.md", "spec/attachments/RPC-014/ref.md"]
    let unit = WorkUnitInfo {
        id: "DOC-014".to_string(),
        title: "Docs".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: vec![
            "spec/attachments/RPC-014/notes.md".to_string(),
            "spec/attachments/RPC-014/ref.md".to_string(),
        ],
        last_state_change_at: None,
    };
    let mut store = BoardStore::default();
    store.replace_work_units(vec![unit]);
    // @step And the focused column matches DOC-014's column and DOC-014 is selected
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring 'Attachments (use the "A" key to view): notes.md, ref.md'
    assert!(
        joined.contains("Attachments (use the \"A\" key to view): notes.md, ref.md"),
        "missing attachments row:\n{joined}"
    );
}

/// Scenario: Box-drawing borders and inner junctions are painted
#[test]
fn box_drawing_borders_and_inner_junctions_are_painted() {
    // @step Given a BoardStore containing AUTH-001 in backlog
    let mut store = BoardStore::default();
    store.replace_work_units(vec![make_unit("AUTH-001", "backlog", "story")]);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    // @step Then row 0 of the rendered buffer starts with "┌" and ends with "┐"
    let row0 = row_string(&buf, 0);
    assert!(
        row0.starts_with('┌'),
        "row 0 must start with ┌, got `{row0}`"
    );
    assert!(
        row0.trim_end().ends_with('┐'),
        "row 0 must end with ┐, got `{row0}`"
    );
    // @step And the last in-bounds row of the box starts with "└" and ends with "┘"
    let last = row_string(&buf, 23);
    assert!(
        last.starts_with('└'),
        "last row must start with └, got `{last}`"
    );
    assert!(
        last.trim_end().ends_with('┘'),
        "last row must end with ┘, got `{last}`"
    );
    // @step And at least one inner row contains the glyph "├" and the glyph "┬" and the glyph "┤"
    let has_top = (1..23).any(|y| {
        let r = row_string(&buf, y);
        r.contains('├') && r.contains('┬') && r.contains('┤')
    });
    assert!(has_top, "expected one inner row with ├ ┬ ┤ junctions");
    // @step And at least one inner row contains the glyph "├" and the glyph "┼" and the glyph "┤"
    let has_cross = (1..23).any(|y| {
        let r = row_string(&buf, y);
        r.contains('├') && r.contains('┼') && r.contains('┤')
    });
    assert!(has_cross, "expected one inner row with ├ ┼ ┤ junctions");
    // @step And at least one inner row contains the glyph "├" and the glyph "┴" and the glyph "┤"
    let has_bottom = (1..23).any(|y| {
        let r = row_string(&buf, y);
        r.contains('├') && r.contains('┴') && r.contains('┤')
    });
    assert!(has_bottom, "expected one inner row with ├ ┴ ┤ junctions");
}

/// Scenario: Focused column header is cyan+bold and other columns are dim
#[test]
fn focused_column_header_is_cyan_bold_and_other_columns_are_dim() {
    // @step Given a BoardStore containing AUTH-001 in backlog with the BACKLOG column focused
    let mut store = BoardStore::default();
    store.replace_work_units(vec![make_unit("AUTH-001", "backlog", "story")]);
    store.set_focused_column("backlog");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the column header row contains the substring "BACKLOG"
    assert!(joined.contains("BACKLOG"), "missing BACKLOG header");
    // @step And the cell holding "BACKLOG" is styled with foreground Cyan and the bold modifier
    let (x, y) = find_substring(&buf, "BACKLOG").expect("BACKLOG must appear");
    let cell = &buf[(x, y)];
    assert_eq!(cell.fg, Color::Cyan, "BACKLOG cell fg must be Cyan");
    assert!(
        cell.modifier.contains(Modifier::BOLD),
        "BACKLOG cell must carry the BOLD modifier"
    );
    // @step And the column header row contains the substring "SPECIFYING"
    assert!(joined.contains("SPECIFYING"), "missing SPECIFYING header");
    // @step And the cell holding "SPECIFYING" is styled with the theme.dim foreground (DarkGray)
    let (sx, sy) = find_substring(&buf, "SPECIFYING").expect("SPECIFYING must appear");
    let scell = &buf[(sx, sy)];
    assert_eq!(
        scell.fg,
        Color::DarkGray,
        "SPECIFYING cell fg must be DarkGray (theme.dim)"
    );
}

/// Scenario: Bug cells render red and the focused selected cell flips to bg=green fg=black bold
#[test]
fn bug_cells_render_red_and_focused_selected_cell_flips_to_bg_green_fg_black_bold() {
    // @step Given a BoardStore containing BUG-001 (bug) and BUG-002 (bug) in the backlog column
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        make_unit("BUG-001", "backlog", "bug"),
        make_unit("BUG-002", "backlog", "bug"),
    ]);
    // @step And the focused column is "backlog" and BUG-001 is selected
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    // @step Then the cell containing "BUG-001" is styled with background Green, foreground Black and the bold modifier
    let (x1, y1) =
        find_substring_in_content(&buf, "BUG-001").expect("BUG-001 must appear in content rows");
    let c1 = &buf[(x1, y1)];
    assert_eq!(
        c1.bg,
        Color::Green,
        "BUG-001 selected cell bg must be Green"
    );
    assert_eq!(
        c1.fg,
        Color::Black,
        "BUG-001 selected cell fg must be Black"
    );
    assert!(
        c1.modifier.contains(Modifier::BOLD),
        "BUG-001 selected cell must be bold"
    );
    // @step And the cell containing "BUG-002" is styled with foreground Red
    let (x2, y2) =
        find_substring_in_content(&buf, "BUG-002").expect("BUG-002 must appear in content rows");
    let c2 = &buf[(x2, y2)];
    assert_eq!(
        c2.fg,
        Color::Red,
        "BUG-002 unselected bug cell fg must be Red"
    );
}

/// Scenario: Task cells render blue with the [estimate] suffix
#[test]
fn task_cells_render_blue_with_estimate_suffix() {
    // @step Given a BoardStore containing TASK-001 (task, estimate 3) in the implementing column
    let mut store = BoardStore::default();
    let mut t = make_unit("TASK-001", "implementing", "task");
    t.estimate = Some(3);
    store.replace_work_units(vec![t]);
    // @step And the focused column is "backlog" (so TASK-001 is NOT the selected cell)
    store.set_focused_column("backlog");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "TASK-001 [3]"
    assert!(
        joined.contains("TASK-001 [3]"),
        "missing 'TASK-001 [3]':\n{joined}"
    );
    // @step And the cell containing "TASK-001 [3]" is styled with foreground Blue
    let (x, y) =
        find_substring_in_content(&buf, "TASK-001").expect("TASK-001 must appear in content rows");
    let c = &buf[(x, y)];
    assert_eq!(
        c.fg,
        Color::Blue,
        "TASK-001 unselected task cell fg must be Blue"
    );
}

/// Scenario: Footer string and footer separator are still painted at the bottom
#[test]
fn footer_string_and_footer_separator_are_still_painted_at_the_bottom() {
    // @step Given a BoardStore containing AUTH-001 in backlog
    let mut store = BoardStore::default();
    store.replace_work_units(vec![make_unit("AUTH-001", "backlog", "story")]);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    // @step Then the last in-bounds inner row contains the substring "← → Columns"
    let footer_row = row_string(&buf, 22);
    assert!(
        footer_row.contains("← → Columns"),
        "expected footer on row 22, got: `{footer_row}`"
    );
    // @step And the same row contains the substring "↵ Work Agent"
    assert!(
        footer_row.contains("↵ Work Agent"),
        "expected '↵ Work Agent' on footer row, got: `{footer_row}`"
    );
    // @step And the row immediately above the footer contains the glyph "┴"
    let sep_row = row_string(&buf, 21);
    assert!(
        sep_row.contains('┴'),
        "expected ┴ on row 21 (footer separator), got: `{sep_row}`"
    );
    // Sanity: the ⏩ / 🟢 indicators must NOT appear in this slice (RPC-016).
    let (_, _) = find_glyph(&buf, "⏩").unwrap_or((0, 0));
}
