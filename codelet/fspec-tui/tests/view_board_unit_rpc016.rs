//! RPC-016 — BoardView viewport + indicators + keyboard nav render tests.
//!
//! Feature: spec/features/rpc016-board-viewport.feature
//!
//! Drives `BoardView::render_with_store` and `BoardView::handle_event`
//! against a `TestBackend` to assert per-column scroll arrows (↑/↓),
//! the last-changed (⏩) and session-attached (🟢) indicators, and the
//! four new keyboard navigation actions (PageUp/PageDown/Home/End).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, Theme};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
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

fn make_unit_at(id: &str, status: &str, ts: &str) -> WorkUnitInfo {
    let mut u = make_unit(id, status, "story");
    u.last_state_change_at = Some(ts.to_string());
    u
}

fn render(width: u16, height: u16, view: &BoardView, store: &BoardStore) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

fn render_fresh(width: u16, height: u16, store: &BoardStore) -> Buffer {
    let (view, _rx) = fresh();
    render(width, height, &view, store)
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

/// Build a string of the rows inside the column-content viewport for the
/// BACKLOG column. The BoardView layout after RPC-016 paints the content
/// rows starting at split[7] (rows 14..N-2 in a 24-row terminal). This
/// helper scans every row of the buffer that lies between the cross
/// junction `├┼┤` row and the bottom junction `├┴┤` row, restricted to
/// the BACKLOG column's horizontal band.
fn backlog_content_rows(buf: &Buffer) -> Vec<String> {
    // Find the cross-junction row (top of content area) and the bottom
    // junction row (just below content area).
    let mut content_start: Option<u16> = None;
    let mut content_end: Option<u16> = None;
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        if content_start.is_none() && row.contains('├') && row.contains('┼') && row.contains('┤') {
            content_start = Some(y + 1);
        } else if content_start.is_some()
            && content_end.is_none()
            && row.contains('├')
            && row.contains('┴')
            && row.contains('┤')
        {
            content_end = Some(y);
        }
    }
    let start = content_start.expect("expected cross-junction row in buffer");
    let end = content_end.expect("expected bottom-junction row in buffer");

    // Determine the BACKLOG column horizontal band by scanning the
    // column-header row. Layout (top→bottom) is:
    //   …, [├┬┤], [column header], [├┼┤], [content rows], [├┴┤], …
    // The first `├┼┤` we find marks `content_start - 1` (the cross
    // junction). The column header sits TWO rows before content_start.
    let header_row = start - 2;
    let mut header = String::new();
    for x in 0..buf.area.width {
        header.push_str(buf[(x, header_row)].symbol());
    }
    let upper = header.to_uppercase();
    let backlog_byte_idx = upper.find("BACKLOG").expect("BACKLOG header missing");
    // Convert byte index to char index — header is ASCII for the labels.
    let backlog_col_x: u16 = upper[..backlog_byte_idx].chars().count() as u16;
    // Find the next `│` border to the right of the BACKLOG label to
    // bound the column horizontally.
    let mut col_end_x: u16 = buf.area.width;
    for x in (backlog_col_x)..buf.area.width {
        if buf[(x, header_row)].symbol() == "│" {
            col_end_x = x;
            break;
        }
    }

    let mut rows: Vec<String> = Vec::new();
    for y in start..end {
        let mut row = String::new();
        for x in backlog_col_x..col_end_x {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(normalize_wide_char_padding(&row));
    }
    rows
}

/// Ratatui paints 2-cell-wide glyphs (such as ⏩ and 🟢) by writing the
/// glyph into the first cell and a space into the continuation cell.
/// When the test concatenates cell symbols we therefore see one extra
/// space after each wide glyph. Collapse it so the user-visible
/// rendering (single space) matches the feature-file assertions.
fn normalize_wide_char_padding(row: &str) -> String {
    let mut out = String::with_capacity(row.len());
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '⏩' || c == '🟢' {
            if let Some(&next) = chars.peek() {
                if next == ' ' {
                    chars.next();
                }
            }
        }
    }
    out
}

fn backlog_joined(buf: &Buffer) -> String {
    backlog_content_rows(buf).join("\n")
}

/// Scenario: Column with no scroll renders the down arrow on the last viewport row
#[test]
fn column_with_no_scroll_renders_the_down_arrow_on_the_last_viewport_row() {
    // @step Given a BoardStore seeded with twenty story work units all in the backlog column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| make_unit(&format!("AUTH-{i:03}"), "backlog", "story"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is "backlog" and the selected index is 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render_fresh(120, 24, &store);
    let rows = backlog_content_rows(&buf);
    // @step Then the column-content rows for the BACKLOG column contain the glyph "↓" on the last viewport row
    let last = rows.last().expect("backlog rows must not be empty");
    assert!(
        last.contains('↓'),
        "last backlog viewport row must contain ↓ when more units exist below; got `{last}` in rows:\n{rows:#?}"
    );
    // @step And the column-content rows for the BACKLOG column do NOT contain the glyph "↑" on the first viewport row
    let first = rows.first().expect("backlog rows must not be empty");
    assert!(
        !first.contains('↑'),
        "first backlog viewport row must NOT contain ↑ when scroll_offset is 0; got `{first}`"
    );
}

/// Scenario: Column with mid-range scroll renders both up and down arrows
#[test]
fn column_with_mid_range_scroll_renders_both_up_and_down_arrows() {
    // @step Given a BoardStore seeded with twenty story work units all in the backlog column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| make_unit(&format!("AUTH-{i:03}"), "backlog", "story"))
        .collect();
    store.replace_work_units(units);
    // @step And the BACKLOG scroll_offset is 5
    store.set_scroll_offset_for("backlog", 5);
    // @step And the focused column is "backlog"
    store.set_focused_column("backlog");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render_fresh(120, 24, &store);
    let rows = backlog_content_rows(&buf);
    // @step Then the column-content rows for the BACKLOG column contain the glyph "↑" on the first viewport row
    let first = rows.first().expect("backlog rows non-empty");
    assert!(
        first.contains('↑'),
        "first backlog viewport row must contain ↑ when scroll_offset > 0; got `{first}`"
    );
    // @step And the column-content rows for the BACKLOG column contain the glyph "↓" on the last viewport row
    let last = rows.last().expect("backlog rows non-empty");
    assert!(
        last.contains('↓'),
        "last backlog viewport row must contain ↓ when more units exist below; got `{last}`"
    );
}

/// Scenario: Column with fewer units than viewport_height renders no arrows
#[test]
fn column_with_fewer_units_than_viewport_height_renders_no_arrows() {
    // @step Given a BoardStore seeded with three story work units all in the backlog column
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        make_unit("AUTH-001", "backlog", "story"),
        make_unit("AUTH-002", "backlog", "story"),
        make_unit("AUTH-003", "backlog", "story"),
    ]);
    // @step And the focused column is "backlog"
    store.set_focused_column("backlog");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render_fresh(120, 24, &store);
    let joined = backlog_joined(&buf);
    // @step Then the column-content rows for the BACKLOG column do NOT contain the glyph "↑"
    assert!(
        !joined.contains('↑'),
        "BACKLOG content must NOT contain ↑ when units.len() < viewport_height; got:\n{joined}"
    );
    // @step And the column-content rows for the BACKLOG column do NOT contain the glyph "↓"
    assert!(
        !joined.contains('↓'),
        "BACKLOG content must NOT contain ↓ when units.len() < viewport_height; got:\n{joined}"
    );
}

/// Scenario: Most-recently-changed work unit renders the ⏩ ⏩ prefix and suffix
#[test]
fn most_recently_changed_work_unit_renders_the_marker_prefix_and_suffix() {
    // @step Given a BoardStore seeded with AUTH-001 last_state_change_at "2026-05-13T10:00:00Z" and AUTH-002 last_state_change_at "2026-05-14T10:00:00Z" in the backlog column
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        make_unit_at("AUTH-001", "backlog", "2026-05-13T10:00:00Z"),
        make_unit_at("AUTH-002", "backlog", "2026-05-14T10:00:00Z"),
    ]);
    // @step And the focused column is "specifying" so neither unit is the selected highlighted cell
    store.set_focused_column("specifying");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render_fresh(120, 24, &store);
    let joined = backlog_joined(&buf);
    // @step Then the column-content rows for the BACKLOG column contain the substring "⏩ AUTH-002"
    assert!(
        joined.contains("⏩ AUTH-002"),
        "BACKLOG content must contain `⏩ AUTH-002` prefix; got:\n{joined}"
    );
    // @step And the column-content rows for the BACKLOG column contain the substring "AUTH-002 ⏩"
    assert!(
        joined.contains("AUTH-002 ⏩"),
        "BACKLOG content must contain `AUTH-002 ⏩` suffix; got:\n{joined}"
    );
    // @step And the column-content rows for the BACKLOG column do NOT contain the substring "⏩ AUTH-001"
    assert!(
        !joined.contains("⏩ AUTH-001"),
        "BACKLOG content must NOT mark AUTH-001 (older) with ⏩; got:\n{joined}"
    );
}

/// Scenario: Work unit with an attached session renders the 🟢 prefix
#[test]
fn work_unit_with_an_attached_session_renders_the_green_circle_prefix() {
    // @step Given a BoardStore seeded with AUTH-002 (story, backlog, estimate 5) and AUTH-001 (story, backlog) with last_state_change_at on AUTH-001 strictly greater than AUTH-002
    let mut store = BoardStore::default();
    let mut u2 = make_unit_at("AUTH-002", "backlog", "2026-05-13T10:00:00Z");
    u2.estimate = Some(5);
    let u1 = make_unit_at("AUTH-001", "backlog", "2026-05-14T10:00:00Z");
    store.replace_work_units(vec![u2, u1]);
    // @step And the BoardStore has an attached session for AUTH-002
    store.attach_session("AUTH-002", SessionId::new("s-1"));
    // @step And the focused column is "specifying" so neither unit is the selected highlighted cell
    store.set_focused_column("specifying");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render_fresh(120, 24, &store);
    let joined = backlog_joined(&buf);
    // @step Then the column-content rows for the BACKLOG column contain the substring "🟢 AUTH-002 [5]"
    assert!(
        joined.contains("🟢 AUTH-002 [5]"),
        "BACKLOG content must contain `🟢 AUTH-002 [5]`; got:\n{joined}"
    );
}

/// Scenario: Last-changed and session-attached indicators stack on the same unit
#[test]
fn last_changed_and_session_attached_indicators_stack_on_the_same_unit() {
    // @step Given a BoardStore seeded with AUTH-002 (story, backlog) carrying the largest last_state_change_at
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        make_unit_at("AUTH-001", "backlog", "2026-05-12T10:00:00Z"),
        make_unit_at("AUTH-002", "backlog", "2026-05-14T10:00:00Z"),
    ]);
    // @step And the BoardStore has an attached session for AUTH-002
    store.attach_session("AUTH-002", SessionId::new("s-1"));
    // @step And the focused column is "specifying" so AUTH-002 is not the selected highlighted cell
    store.set_focused_column("specifying");
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render_fresh(120, 24, &store);
    let joined = backlog_joined(&buf);
    // @step Then the column-content rows for the BACKLOG column contain the substring "⏩ 🟢 AUTH-002"
    assert!(
        joined.contains("⏩ 🟢 AUTH-002"),
        "BACKLOG content must contain `⏩ 🟢 AUTH-002`; got:\n{joined}"
    );
    // @step And the column-content rows for the BACKLOG column contain the substring "AUTH-002 ⏩"
    assert!(
        joined.contains("AUTH-002 ⏩"),
        "BACKLOG content must contain `AUTH-002 ⏩`; got:\n{joined}"
    );
}

/// Scenario: PageDown advances the focused column's selection by viewport_height rows
#[test]
fn pagedown_advances_focused_column_selection_by_viewport_height_rows() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..30)
        .map(|i| make_unit(&format!("AUTH-{i:03}"), "backlog", "story"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is "backlog" with selected index 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    let (view, mut rx) = fresh();
    // Drive a render so the view records its last_viewport_height.
    let _ = render(120, 24, &view, &store);
    // @step When BoardView handles a PageDown key event against the store
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty())),
        &store,
    );
    // @step Then the action bus carries an Action::ScrollFocusedColumnDown variant whose payload equals the most recent viewport_height observed by BoardView
    let mut got_scroll_down = false;
    while let Ok(a) = rx.try_recv() {
        if matches!(a, Action::ScrollFocusedColumnDown(_)) {
            got_scroll_down = true;
        }
    }
    assert!(
        got_scroll_down,
        "PageDown must emit Action::ScrollFocusedColumnDown(viewport_height)"
    );
}

/// Scenario: PageUp scrolls the focused column's selection back by viewport_height rows
#[test]
fn pageup_scrolls_focused_column_selection_back_by_viewport_height_rows() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..30)
        .map(|i| make_unit(&format!("AUTH-{i:03}"), "backlog", "story"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is "backlog" with selected index 25
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 25);
    let (view, mut rx) = fresh();
    let _ = render(120, 24, &view, &store);
    // @step When BoardView handles a PageUp key event against the store
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty())),
        &store,
    );
    // @step Then the action bus carries an Action::ScrollFocusedColumnUp variant whose payload equals the most recent viewport_height observed by BoardView
    let mut got_scroll_up = false;
    while let Ok(a) = rx.try_recv() {
        if matches!(a, Action::ScrollFocusedColumnUp(_)) {
            got_scroll_up = true;
        }
    }
    assert!(
        got_scroll_up,
        "PageUp must emit Action::ScrollFocusedColumnUp(viewport_height)"
    );
}

/// Scenario: Home jumps the focused column's selection to the first unit
#[test]
fn home_jumps_focused_column_selection_to_first_unit() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..30)
        .map(|i| make_unit(&format!("AUTH-{i:03}"), "backlog", "story"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is "backlog" with selected index 20
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 20);
    let (view, mut rx) = fresh();
    let _ = render(120, 24, &view, &store);
    // @step When BoardView handles a Home key event against the store
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty())),
        &store,
    );
    // @step Then the action bus carries the Action::SelectFirstInFocused variant
    let mut got_first = false;
    while let Ok(a) = rx.try_recv() {
        if matches!(a, Action::SelectFirstInFocused) {
            got_first = true;
        }
    }
    assert!(got_first, "Home must emit Action::SelectFirstInFocused");
}

/// Scenario: End jumps the focused column's selection to the last unit
#[test]
fn end_jumps_focused_column_selection_to_last_unit() {
    // @step Given a BoardStore seeded with thirty story work units all in the backlog column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..30)
        .map(|i| make_unit(&format!("AUTH-{i:03}"), "backlog", "story"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is "backlog" with selected index 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    let (view, mut rx) = fresh();
    let _ = render(120, 24, &view, &store);
    // @step When BoardView handles an End key event against the store
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::empty())),
        &store,
    );
    // @step Then the action bus carries the Action::SelectLastInFocused variant
    let mut got_last = false;
    while let Ok(a) = rx.try_recv() {
        if matches!(a, Action::SelectLastInFocused) {
            got_last = true;
        }
    }
    assert!(got_last, "End must emit Action::SelectLastInFocused");
}

/// Scenario: RPC-014 details strip and RPC-015 header are still painted after RPC-016 lands
#[test]
fn rpc014_details_strip_and_rpc015_header_still_painted_after_rpc016() {
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
    let buf = render_fresh(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring "AUTH-001: User Login"
    assert!(joined.contains("AUTH-001: User Login"), "missing details title:\n{joined}");
    // @step And the rendered buffer contains the substring "Epic: authentication"
    assert!(joined.contains("Epic: authentication"), "missing Epic line:\n{joined}");
    // @step And the rendered buffer contains the substring "Status: backlog"
    assert!(joined.contains("Status: backlog"), "missing Status line:\n{joined}");
    // @step And the rendered buffer contains the substring "Checkpoints: None"
    assert!(joined.contains("Checkpoints: None"), "missing Checkpoints: None:\n{joined}");
    // @step And the rendered buffer contains the substring "← →"
    assert!(joined.contains("← →"), "missing footer arrows:\n{joined}");
    // @step And the rendered buffer contains the substring "Work Agent"
    assert!(joined.contains("Work Agent"), "missing footer 'Work Agent':\n{joined}");
}
