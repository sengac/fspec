//! RPC-022 — AgentView RoleBanner integration tests.
//!
//! Feature: spec/features/rpc022-role-banner.feature
//!
//! Drives the inline `RoleBanner` widget through `AgentView::render_with_store`
//! so the layout glue (1-row carve-out above scrollback + suppression
//! while resume/search mode views are active) is exercised, not just
//! the standalone widget paint path covered by `role_banner.rs` inline
//! tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::store::{AgentViewStore, SessionContext};
use codelet_fspec_tui::views::AgentView;
use codelet_fspec_tui::{ResumeSessionView, SearchHistoryView};
use codelet_rpc_types::SessionId;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

mod common;

/// Helper: render AgentView at the supplied dimensions and return the
/// resulting buffer as a Vec<String> (one entry per row).
fn render_rows(
    width: u16,
    height: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    let buf: Buffer = term.backend().buffer().clone();
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

fn fresh_view() -> AgentView {
    let (tx, _rx) = unbounded_channel();
    AgentView::new(tx)
}

fn store_with_session(id: &str) -> AgentViewStore {
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(SessionId::new(id)));
    store
}

fn scrollback_block_height(rows: &[String]) -> usize {
    // RPC-029: AgentView no longer wraps scrollback in a border Block,
    // so the old ┌…└ counting strategy no longer applies. The
    // scrollback slot is the contiguous span of blank/text rows
    // between the header (row 0) and the footer/input rows at the
    // bottom. To measure it portably across with-/without-role
    // renders, we count rows from index 1 (skip header) down to the
    // first row containing the input prompt "> " or the cwd marker.
    // The "Role:" row, when present, sits at index 1 — we count it
    // as part of the area that the banner reserves, but exclude it
    // from the scrollback height (the no-role layout has 0 rows of
    // banner so the scrollback height is exactly 1 row larger).
    let mut bottom = rows.len();
    for (i, row) in rows.iter().enumerate().rev() {
        if row.contains("> Type a message") || row.contains("> ") || (i > 0 && row.contains('/')) {
            bottom = i;
            break;
        }
    }
    let mut top = 1usize; // skip header row 0
    if let Some(first) = rows.get(top) {
        if first.trim_start().starts_with("Role:") {
            top += 1;
        }
    }
    bottom.saturating_sub(top)
}

/// Scenario: RoleBanner renders zero rows when no role is set on the focused session
#[test]
fn role_banner_renders_zero_rows_when_no_role_set() {
    // @step Given an AgentViewStore with one open session "s-1" and role_for("s-1") = None
    let mut store = store_with_session("s-1");
    // (intentionally do not call set_role)
    assert!(store.role_for(&SessionId::new("s-1")).is_none());
    let mut view = fresh_view();
    // @step When AgentView.render_with_store paints into a 80x24 area
    let rows = render_rows(80, 24, &mut store, &mut view);
    // @step Then no row in the rendered buffer starts with "Role:"
    assert!(
        !rows.iter().any(|r| r.trim_start().starts_with("Role:")),
        "expected no Role: row, got:\n{}",
        rows.join("\n")
    );
    // @step And the scrollback Block consumes the entire flex region between header and input
    // RPC-029: there is no Block any more; the scrollback area is the
    // span between the header (row 0) and the footer/input rows. A
    // healthy AgentView render in an 80x24 area gives this span at
    // least 14 rows of headroom when no banner is present.
    let block_height = scrollback_block_height(&rows);
    assert!(
        block_height >= 14,
        "scrollback span = {block_height} rows, expected >= 14 with no role banner"
    );
}

/// Scenario: RoleBanner renders one row when a role is set on the focused session
#[test]
fn role_banner_renders_one_row_when_role_is_set() {
    // @step Given an AgentViewStore with one open session "s-1" and role_for("s-1") = Some("You are a security reviewer")
    let mut store = store_with_session("s-1");
    store.set_role(
        SessionId::new("s-1"),
        Some("You are a security reviewer".to_string()),
    );
    let mut view = fresh_view();
    // @step When AgentView.render_with_store paints into a 80x24 area
    let rows = render_rows(80, 24, &mut store, &mut view);
    // @step Then exactly one row in the rendered buffer starts with "Role:"
    let role_rows: Vec<&String> = rows
        .iter()
        .filter(|r| r.trim_start().starts_with("Role:"))
        .collect();
    assert_eq!(
        role_rows.len(),
        1,
        "expected exactly 1 Role: row, got {role_rows:?}"
    );
    // @step And the substring "You are a security reviewer" appears on that row
    assert!(role_rows[0].contains("You are a security reviewer"));
    // @step And the scrollback Block height shrinks by exactly 1 row compared to the no-role layout
    let with_role_block_height = scrollback_block_height(&rows);

    // Render the same layout without a role for the comparison baseline.
    let mut store_no_role = store_with_session("s-1");
    let mut view_no_role = fresh_view();
    let rows_no_role = render_rows(80, 24, &mut store_no_role, &mut view_no_role);
    let no_role_block_height = scrollback_block_height(&rows_no_role);

    assert_eq!(
        no_role_block_height,
        with_role_block_height + 1,
        "scrollback span must shrink by exactly 1 row when banner appears (no-role={no_role_block_height}, with-role={with_role_block_height})"
    );
}

/// Scenario: Multi-line role text is collapsed to a single line
#[test]
fn multi_line_role_text_is_collapsed_to_a_single_line() {
    // @step Given an AgentViewStore with role_for("s-1") = Some("You are a security reviewer.\nAnalyze code for vulnerabilities.")
    let mut store = store_with_session("s-1");
    store.set_role(
        SessionId::new("s-1"),
        Some("You are a security reviewer.\nAnalyze code for vulnerabilities.".to_string()),
    );
    let mut view = fresh_view();
    // @step When AgentView.render_with_store paints into a 100x24 area
    let rows = render_rows(100, 24, &mut store, &mut view);
    // @step Then the rendered "Role:" row contains "You are a security reviewer. Analyze code for vulnerabilities."
    let role_row = rows
        .iter()
        .find(|r| r.trim_start().starts_with("Role:"))
        .expect("Role: row not found");
    assert!(
        role_row.contains("You are a security reviewer. Analyze code for vulnerabilities."),
        "Role: row missing collapsed text, got {role_row:?}"
    );
    // @step And the rendered "Role:" row contains NO newline characters
    assert!(!role_row.contains('\n'));
}

/// Scenario: Long role text is truncated to terminal width
#[test]
fn long_role_text_is_truncated_to_terminal_width() {
    // @step Given an AgentViewStore with role_for("s-1") = Some("X".repeat(500))
    let mut store = store_with_session("s-1");
    store.set_role(SessionId::new("s-1"), Some("X".repeat(500)));
    let mut view = fresh_view();
    // @step When AgentView.render_with_store paints into a 40x24 area
    let rows = render_rows(40, 24, &mut store, &mut view);
    // @step Then exactly one row contains the "Role:" prefix
    let role_rows: Vec<&String> = rows
        .iter()
        .filter(|r| r.trim_start().starts_with("Role:"))
        .collect();
    assert_eq!(role_rows.len(), 1, "expected exactly 1 Role: row");
    // @step And that row occupies exactly 40 columns and does not wrap to a second row
    assert_eq!(role_rows[0].chars().count(), 40);
}

/// Scenario: RoleBanner reflects the focused session only, not background sessions
#[test]
fn role_banner_reflects_focused_session_only() {
    // @step Given an AgentViewStore with two open sessions "s-1" and "s-2"
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(SessionId::new("s-1")));
    store.append_session(SessionContext::new(SessionId::new("s-2")));
    // @step And role_for("s-1") = Some("Reviewer A") and role_for("s-2") = None
    store.set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    // @step And current_session_index = 0
    // append_session pushes and focuses the new entry, so we need to
    // re-focus s-1 (index 0).
    store.focus_session_index(0);
    assert_eq!(store.current_session_index(), 0);
    let mut view = fresh_view();
    // @step When AgentView.render_with_store paints
    let rows = render_rows(80, 24, &mut store, &mut view);
    // @step Then the "Role:" row reads "Role: Reviewer A"
    let role_row = rows
        .iter()
        .find(|r| r.trim_start().starts_with("Role:"))
        .expect("Role: row not found");
    assert!(
        role_row.contains("Role: Reviewer A"),
        "expected `Role: Reviewer A`, got {role_row:?}"
    );
    // @step When current_session_index is set to 1
    store.focus_session_index(1);
    assert_eq!(store.current_session_index(), 1);
    // @step And AgentView.render_with_store paints
    let rows = render_rows(80, 24, &mut store, &mut view);
    // @step Then no "Role:" row appears in the rendered buffer
    assert!(
        !rows.iter().any(|r| r.trim_start().starts_with("Role:")),
        "expected no Role: row when focused on s-2 (which has no role)"
    );
}

/// Scenario: RoleBanner is suppressed while resume_view is active
#[test]
fn role_banner_is_suppressed_while_resume_view_is_active() {
    // @step Given an AgentViewStore with role_for("s-1") = Some("Reviewer A")
    let mut store = store_with_session("s-1");
    store.set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    let mut view = fresh_view();
    // @step And AgentView.resume_view is Some(default ResumeSessionView)
    view.resume_view = Some(ResumeSessionView::default());
    // @step When AgentView.render_with_store paints into a 80x24 area
    let rows = render_rows(80, 24, &mut store, &mut view);
    // @step Then no row in the rendered buffer starts with "Role:"
    assert!(
        !rows.iter().any(|r| r.trim_start().starts_with("Role:")),
        "expected no Role: row when resume_view is active, got:\n{}",
        rows.join("\n")
    );
}

/// Scenario: RoleBanner is suppressed while search_view is active
#[test]
fn role_banner_is_suppressed_while_search_view_is_active() {
    // @step Given an AgentViewStore with role_for("s-1") = Some("Reviewer A")
    let mut store = store_with_session("s-1");
    store.set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    let mut view = fresh_view();
    // @step And AgentView.search_view is Some(default SearchHistoryView)
    view.search_view = Some(SearchHistoryView::default());
    // @step When AgentView.render_with_store paints into a 80x24 area
    let rows = render_rows(80, 24, &mut store, &mut view);
    // @step Then no row in the rendered buffer starts with "Role:"
    assert!(
        !rows.iter().any(|r| r.trim_start().starts_with("Role:")),
        "expected no Role: row when search_view is active, got:\n{}",
        rows.join("\n")
    );
}

/// Scenario: role_banner.rs stays under 300 lines
#[test]
fn role_banner_rs_stays_under_300_lines() {
    // @step Given the file rust/fspec-tui/src/views/agent/role_banner.rs after RPC-022 lands
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("views")
        .join("agent")
        .join("role_banner.rs");
    // @step When a test counts the line-count of the file
    let lines = common::read_to_string_or_panic(&path).lines().count();
    // @step Then the file has fewer than 300 lines
    assert!(lines < 300, "role_banner.rs has {lines} lines (>= 300)");
}
