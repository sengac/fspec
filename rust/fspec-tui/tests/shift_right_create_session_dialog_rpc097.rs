//! RPC-097 — AgentView Shift+Right must mount CreateSessionDialog AND
//! the rendered dialog must EXACTLY match TS Ink styling.
//!
//! Feature: spec/features/agentview-shift-right-create-session-dialog.feature
//!
//! Tests are written BEFORE the fix (red phase). They reference the
//! observable, end-to-end contract: after dispatching `Action::SessionNext`
//! or feeding a `Shift+Right` crossterm key through `App::handle_event`,
//! the Compositor must contain `CREATE_SESSION_DIALOG_ID`. The visual
//! parity tests then assert per-cell bg/fg/modifier on the styled cells.

#![allow(clippy::too_many_lines)]

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::components::create_session_dialog::CreateSessionDialog;
use codelet_fspec_tui::components::create_session_dialog::CreateSessionOption;
use codelet_fspec_tui::components::create_session_dialog::CREATE_SESSION_DIALOG_ID;
use codelet_fspec_tui::store::SessionContext;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, Component, EventResult, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitContext};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> App {
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    App::new(backend)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Render a fresh CreateSessionDialog with the given preselect/workunit
/// into an 80x24 TestBackend and return the resulting Buffer.
fn render_dialog_buffer(
    preselect: Option<CreateSessionOption>,
    work_unit: Option<WorkUnitContext>,
) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut dialog = CreateSessionDialog::new(preselect, work_unit);
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).or_fail("Terminal::new")?;
    term.draw(|frame| {
        Component::render(&mut dialog, frame.area(), frame.buffer_mut());
    })
    .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

/// Render an entire App (Navigator + Compositor) onto an 80x24
/// TestBackend and return the buffer. Used for end-to-end Shift+Right
/// visual assertions.
fn render_app_buffer(app: &mut App) -> Result<Buffer, codelet_test_helpers::TestError> {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).or_fail("Terminal::new")?;
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

/// Collect a buffer into a flat String for substring assertions.
fn buffer_to_string(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Find the (x, y) of the first column where N consecutive cells'
/// symbols, concatenated, equal `needle`. Walks each row left-to-right
/// matching by **column index**, not byte offset (so the rounded
/// border's multi-byte glyphs do not shift the returned x).
/// Returns None if `needle` is not present.
fn find_text_cell(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let needle_cols: Vec<&str> = needle.split("").filter(|s| !s.is_empty()).collect();
    let n = needle_cols.len();
    if n == 0 {
        return None;
    }
    for y in 0..buf.area.height {
        if buf.area.width as usize <= n {
            continue;
        }
        for x in 0..=(buf.area.width as usize - n) {
            let mut hit = true;
            for (i, want) in needle_cols.iter().enumerate() {
                if buf[(x as u16 + i as u16, y)].symbol() != *want {
                    hit = false;
                    break;
                }
            }
            if hit {
                return Some((x as u16, y));
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────
// Mount-via-Shift+Right (the bug being fixed)
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right with a single open session mounts CreateSessionDialog
#[test]
fn shift_right_single_session_mounts_create_session_dialog() -> TestResult {
    // @step Given an App in AgentView with one open session at current_session_index 0
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(app.agent_view_store().current_session_index(), 0);

    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);

    // @step Then the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right must push CreateSessionDialog onto the compositor"
    );
    // @step And the dialog is at Priority::Foreground
    assert_eq!(
        app.compositor().topmost_id().as_deref(),
        Some(CREATE_SESSION_DIALOG_ID)
    );
    // @step And current_session_index is still 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    Ok(())
}

/// Scenario: Shift+Right at the last index of three open sessions mounts CreateSessionDialog
#[test]
fn shift_right_at_last_index_of_three_mounts_create_session_dialog() -> TestResult {
    // @step Given an App in AgentView with three open sessions s-1, s-2, s-3 and current_session_index 2
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionCreated(sid("s-3")));
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(app.agent_view_store().current_session_index(), 2);

    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);

    // @step Then the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right at last index must push CreateSessionDialog onto the compositor"
    );
    // @step And current_session_index is still 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    Ok(())
}

/// Scenario: Shift+Right with zero open sessions mounts CreateSessionDialog with generic title
#[test]
fn shift_right_zero_sessions_mounts_with_generic_title() -> TestResult {
    // @step Given an App in AgentView with zero open sessions
    let mut app = fresh_app();
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);

    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);

    // @step Then the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right on empty store must push CreateSessionDialog"
    );

    // @step And the rendered dialog title is "Start New Agent?"
    // @step And the rendered description is "Starts a new agent, not linked to any work unit."
    let painted = buffer_to_string(&render_app_buffer(&mut app)?);
    assert!(
        painted.contains("Start New Agent?"),
        "expected 'Start New Agent?' in rendered buffer, got:\n{painted}"
    );
    assert!(
        painted.contains("Starts a new agent, not linked to any work unit."),
        "expected unattached description in rendered buffer, got:\n{painted}"
    );
    Ok(())
}

#[path = "shift_right_create_session_dialog/generic.rs"]
mod generic;

// ─────────────────────────────────────────────────────────────────────
// Visual parity with TS Ink (the styling drift being fixed)
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Freshly mounted dialog paints the Start button blue/white/bold
/// and the other two buttons gray
#[test]
fn freshly_mounted_dialog_paints_yes_button_blue_white_bold() -> TestResult {
    // @step Given an App in AgentView with one open session
    // @step When the user presses Shift+Right
    // @step And I render the App onto an 80x24 TestBackend
    let buf = render_dialog_buffer(None, None)?;

    // @step Then the cells covering " Start " have background Color::Blue and foreground Color::White and Modifier::BOLD
    let (x, y) = find_text_cell(&buf, " Start    Start in Worktree ")
        .or_fail("the button row must appear in the rendered dialog buffer")?;
    // Inspect every cell in the 7-cell span ' Start ' — bg/fg/modifier on
    // each must match the TS contract.
    for dx in 0..7 {
        let cell = &buf[(x + dx, y)];
        let style = cell.style();
        assert_eq!(
            style.bg,
            Some(Color::Blue),
            "selected button cell ({},{}) bg must be Blue, got {:?} (char {:?})",
            x + dx,
            y,
            style.bg,
            cell.symbol()
        );
        assert_eq!(
            style.fg,
            Some(Color::White),
            "selected button cell ({},{}) fg must be White, got {:?} (char {:?})",
            x + dx,
            y,
            style.fg,
            cell.symbol()
        );
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "selected button cell ({},{}) must include BOLD modifier, got {:?}",
            x + dx,
            y,
            style.add_modifier
        );
    }

    // @step And the cells covering " Start in Worktree " have foreground Color::Gray
    let (ix, iy) = find_text_cell(&buf, " Start in Worktree    Cancel ")
        .or_fail("the button row must appear in the rendered dialog buffer")?;
    for dx in 0..19 {
        let cell = &buf[(ix + dx, iy)];
        let style = cell.style();
        assert_eq!(
            style.fg,
            Some(Color::Gray),
            "unselected ' Start in Worktree ' cell ({},{}) fg must be Gray, got {:?} (char {:?})",
            ix + dx,
            iy,
            style.fg,
            cell.symbol()
        );
        assert_ne!(
            style.bg,
            Some(Color::Blue),
            "unselected ' Start in Worktree ' cell ({},{}) must NOT have Blue bg",
            ix + dx,
            iy
        );
    }

    // @step And the cells covering " Cancel " have foreground Color::Gray
    let (cx, cy) = find_text_cell(&buf, " Cancel ")
        .or_fail("' Cancel ' must appear in the rendered dialog buffer")?;
    for dx in 0..8 {
        let cell = &buf[(cx + dx, cy)];
        let style = cell.style();
        assert_eq!(
            style.fg,
            Some(Color::Gray),
            "unselected ' Cancel ' cell ({},{}) fg must be Gray, got {:?} (char {:?})",
            cx + dx,
            cy,
            style.fg,
            cell.symbol()
        );
        assert_ne!(
            style.bg,
            Some(Color::Blue),
            "unselected ' Cancel ' cell ({},{}) must NOT have Blue bg",
            cx + dx,
            cy
        );
    }

    // @step And no cell in the buffer contains the glyph "▸"
    // @step And no cell in the buffer contains the glyph "○"
    let painted = buffer_to_string(&buf);
    assert!(
        !painted.contains('\u{25b8}'),
        "rendered dialog must not contain ▸ marker glyph, got:\n{painted}"
    );
    assert!(
        !painted.contains('\u{25cb}'),
        "rendered dialog must not contain ○ marker glyph, got:\n{painted}"
    );
    Ok(())
}

/// Scenario: Right arrow cycles selection forward with wrap-around
#[test]
fn right_arrow_cycles_forward_with_wrap_around() -> TestResult {
    // @step Given the CreateSessionDialog is mounted with default selection Start
    let mut dialog = CreateSessionDialog::new(None, None);
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    // @step When the user presses Right
    let _ = dialog.handle_event(&key(KeyCode::Right));
    // @step Then the selected option is Start in Worktree
    assert_eq!(dialog.selected_option(), CreateSessionOption::Isolated);
    // @step When the user presses Right
    let _ = dialog.handle_event(&key(KeyCode::Right));
    // @step Then the selected option is Cancel
    assert_eq!(dialog.selected_option(), CreateSessionOption::Cancel);
    // @step When the user presses Right
    let _ = dialog.handle_event(&key(KeyCode::Right));
    // @step Then the selected option is Start
    assert_eq!(dialog.selected_option(), CreateSessionOption::Yes);
    Ok(())
}

/// Scenario: Left arrow wraps from Start back to Cancel
#[test]
fn left_arrow_wraps_from_yes_to_cancel() -> TestResult {
    // @step Given the CreateSessionDialog is mounted with default selection Start
    let mut dialog = CreateSessionDialog::new(None, None);
    // @step When the user presses Left
    let _ = dialog.handle_event(&key(KeyCode::Left));
    // @step Then the selected option is Cancel
    assert_eq!(dialog.selected_option(), CreateSessionOption::Cancel);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Confirmation / cancellation behaviour
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Enter on Start emits CreateSessionSubmitted with isolated false
/// and dismisses the dialog
#[test]
fn enter_on_yes_emits_submitted_non_isolated_and_dismisses() -> TestResult {
    // @step Given the CreateSessionDialog is mounted with selection Start
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.dispatch(Action::SessionNext);
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));

    // @step When the user presses Enter
    let _ = app.handle_event(&key(KeyCode::Enter));

    // @step Then Action::CreateSessionSubmitted with isolated false is emitted
    // (drained via try_recv_action — the dialog's emit_action both pushes
    // through action_tx and stashes pending_action; after the dialog is
    // popped from the compositor we read the action bus.)
    let mut saw_submitted_non_isolated = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::CreateSessionSubmitted { isolated: false }) {
            saw_submitted_non_isolated = true;
        }
    }
    assert!(
        saw_submitted_non_isolated,
        "Enter on Start must emit Action::CreateSessionSubmitted{{ isolated: false }}"
    );
    // @step And the Compositor no longer contains CREATE_SESSION_DIALOG_ID
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    Ok(())
}

/// Scenario: Enter on Start in Worktree emits CreateSessionSubmitted with isolated true
#[test]
fn enter_on_yes_isolated_emits_submitted_isolated_and_dismisses() -> TestResult {
    // @step Given the CreateSessionDialog is mounted with selection Start in Worktree
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.dispatch(Action::SessionNext);
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // Advance selection Start → Start in Worktree
    let _ = app.handle_event(&key(KeyCode::Right));

    // @step When the user presses Enter
    let _ = app.handle_event(&key(KeyCode::Enter));

    // @step Then Action::CreateSessionSubmitted with isolated true is emitted
    let mut saw_submitted_isolated = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::CreateSessionSubmitted { isolated: true }) {
            saw_submitted_isolated = true;
        }
    }
    assert!(
        saw_submitted_isolated,
        "Enter on Start in Worktree must emit Action::CreateSessionSubmitted{{ isolated: true }}"
    );
    // @step And the Compositor no longer contains CREATE_SESSION_DIALOG_ID
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    Ok(())
}

/// Scenario: Enter on Cancel emits CreateSessionCancelled and leaves the
/// MultiLineInput buffer untouched
#[test]
fn enter_on_cancel_emits_cancelled_and_preserves_input() -> TestResult {
    // @step Given an App in AgentView with one open session and MultiLineInput value "hello"
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.navigator_mut().agent.input.set_value("hello");
    // @step And the CreateSessionDialog is mounted with selection Cancel
    app.dispatch(Action::SessionNext);
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    let _ = app.handle_event(&key(KeyCode::Right));
    let _ = app.handle_event(&key(KeyCode::Right));

    // @step When the user presses Enter
    let _ = app.handle_event(&key(KeyCode::Enter));

    // @step Then Action::CreateSessionCancelled is emitted
    let mut saw_cancelled = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::CreateSessionCancelled) {
            saw_cancelled = true;
        }
    }
    assert!(
        saw_cancelled,
        "Enter on Cancel must emit Action::CreateSessionCancelled"
    );
    // @step And the Compositor no longer contains CREATE_SESSION_DIALOG_ID
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step And the MultiLineInput value is still "hello"
    assert_eq!(app.navigator().agent.input.value(), "hello");
    Ok(())
}

/// Scenario: Esc emits CreateSessionCancelled and dismisses the dialog
#[test]
fn esc_emits_cancelled_and_dismisses() -> TestResult {
    // @step Given the CreateSessionDialog is mounted
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.dispatch(Action::SessionNext);
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));

    // @step When the user presses Esc
    let _ = app.handle_event(&key(KeyCode::Esc));

    // @step Then Action::CreateSessionCancelled is emitted
    let mut saw_cancelled = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::CreateSessionCancelled) {
            saw_cancelled = true;
        }
    }
    assert!(
        saw_cancelled,
        "Esc must emit Action::CreateSessionCancelled"
    );
    // @step And the Compositor no longer contains CREATE_SESSION_DIALOG_ID
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    Ok(())
}

/// Scenario: Typed MultiLineInput draft survives the Shift+Right summon
/// and subsequent Esc
#[test]
fn typed_draft_survives_shift_right_summon_and_esc() -> TestResult {
    // @step Given an App in AgentView with one open session and MultiLineInput value "pending"
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.navigator_mut().agent.input.set_value("pending");

    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);
    // @step Then the MultiLineInput value is still "pending"
    assert_eq!(app.navigator().agent.input.value(), "pending");

    // @step When the user presses Esc
    let _ = app.handle_event(&key(KeyCode::Esc));
    // @step Then the MultiLineInput value is still "pending"
    assert_eq!(app.navigator().agent.input.value(), "pending");
    Ok(())
}

/// Scenario: Shift+Right is idempotent when the dialog is already mounted
#[test]
fn shift_right_is_idempotent_when_dialog_already_mounted() -> TestResult {
    // @step Given an App in AgentView with one open session
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);
    // @step Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    let first_count = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == CREATE_SESSION_DIALOG_ID)
        .count();
    assert_eq!(
        first_count, 1,
        "first Shift+Right must push exactly one dialog"
    );

    // @step When the user presses Shift+Right again
    app.dispatch(Action::SessionNext);
    // @step Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    let second_count = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == CREATE_SESSION_DIALOG_ID)
        .count();
    assert_eq!(
        second_count, 1,
        "second Shift+Right must not duplicate the dialog (idempotency)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Footer + source-shape regression
// ─────────────────────────────────────────────────────────────────────

/// Scenario: Rendered footer uses ASCII pipe separators not box-drawing pipes
#[test]
fn rendered_footer_uses_ascii_pipe_not_box_drawing() -> TestResult {
    // @step Given an App in AgentView with one open session
    // @step When the user presses Shift+Right
    // @step And I render the App onto an 80x24 TestBackend
    let buf = render_dialog_buffer(None, None)?;
    let painted = buffer_to_string(&buf);

    // @step Then the rendered buffer contains the string "← → Select | Enter Confirm | Esc Cancel"
    assert!(
        painted.contains("← → Select | Enter Confirm | Esc Cancel"),
        "rendered buffer must contain the exact ASCII-pipe footer, got:\n{painted}"
    );

    // @step And the rendered buffer does not contain the glyph "│"
    // Box-drawing │ U+2502 must not appear anywhere — the dialog uses
    // rounded border with ╭╮╰╯─│ but the FOOTER text uses ASCII |.
    // We check only the footer line by locating its row and confirming
    // the box-drawing pipe is absent within the same row.
    let (fx, fy) = find_text_cell(&buf, "← → Select").or_fail("footer prefix must be present")?;
    let _ = fx;
    let mut footer_row = String::new();
    for x in 0..buf.area.width {
        footer_row.push_str(buf[(x, fy)].symbol());
    }
    // The line contains the rounded border │ at the row's edges (left
    // and right). Strip those leading/trailing border cells before the
    // assertion so we only test the FOOTER text region.
    let inner = footer_row.trim_matches(|c: char| c == '│' || c == ' ' || c == '─');
    assert!(
        !inner.contains('│'),
        "footer text region must not contain U+2502 (got inner: {inner:?}, full row: {footer_row:?})"
    );

    // @step And the ASCII pipe "|" appears in the footer row exactly two times
    let pipe_count = footer_row.chars().filter(|c| *c == '|').count();
    assert_eq!(
        pipe_count, 2,
        "footer row must contain exactly two ASCII pipes (got {pipe_count} in {footer_row:?})"
    );
    Ok(())
}

/// Scenario: Source-shape budget for the refactored CreateSessionDialog
#[test]
fn source_shape_create_session_dialog_under_300_loc() -> TestResult {
    // @step Given the file rust/fspec-tui/src/components/create_session_dialog.rs
    let source = std::fs::read_to_string("src/components/create_session_dialog.rs")
        .or_fail("read create_session_dialog.rs")?;
    // @step Then it has fewer than 300 lines
    let line_count = source.lines().count();
    assert!(
        line_count < 300,
        "create_session_dialog.rs is {line_count} lines; must stay under 300"
    );
    Ok(())
}

/// Scenario: CreateSessionDialog renders via dialog_theme::render_dialog
/// (base dialog primitive reused)
#[test]
fn source_shape_create_session_dialog_uses_render_dialog() -> TestResult {
    // @step Given the source of rust/fspec-tui/src/components/create_session_dialog.rs
    let source = std::fs::read_to_string("src/components/create_session_dialog.rs")
        .or_fail("read create_session_dialog.rs")?;
    // @step Then it imports render_dialog from super::dialog_theme
    assert!(
        source.contains("dialog_theme::")
            || source.contains("use super::dialog_theme")
            || source.contains("render_dialog"),
        "create_session_dialog.rs must reuse dialog_theme::render_dialog primitive"
    );
    // @step And it does not call ratatui Block or Paragraph directly inside the render function
    // We split the file on the render function signature and scan only
    // its body for the disallowed primitives.
    let body = source
        .split_once("fn render(")
        .map(|(_, after)| after)
        .unwrap_or(&source);
    assert!(
        !body.contains("Block::default"),
        "CreateSessionDialog::render must not call Block::default — delegate to render_dialog"
    );
    assert!(
        !body.contains("Paragraph::new"),
        "CreateSessionDialog::render must not call Paragraph::new — delegate to render_dialog"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Sanity: SessionContext attach helper used in the work-unit test
// ─────────────────────────────────────────────────────────────────────

/// Reusable helper — the work-unit-binding test uses set_work_unit_context
/// directly on AgentViewStore. This sanity test pins the helper's API
/// shape so the rendering test above stays linkable.
#[test]
fn agent_view_store_exposes_set_work_unit_context() -> TestResult {
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut().set_work_unit_context(
        sid("s-1"),
        WorkUnitContext {
            id: "X-1".to_string(),
            title: "t".to_string(),
            status: "backlog".to_string(),
        },
    );
    let ctx = app
        .agent_view_store()
        .work_unit_context_for(&sid("s-1"))
        .or_fail("work unit context must be bound")?;
    assert_eq!(ctx.id, "X-1");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// RPC-097 re-review (2026-05-31): BoardView first-press regression
// ─────────────────────────────────────────────────────────────────────
// The original RPC-097 fix only patched the AgentView dispatch path
// (dispatch_session_cycle.rs). BoardView's Shift+Right goes through a SEPARATE
// path (app/dispatch.rs::Action::OpenAgentView arm) that was untouched
// and still relied on the orphan request_create_session_dialog() flag
// setter — requiring TWO presses (one to enter Agent mode, one to fire
// the working AgentView path) to actually see the dialog.
//
// These tests drive the FROM-BOARD path end-to-end via App::handle_event
// with the App still in ViewMode::Board, so they exercise the BoardView
// keybinding → Action::OpenAgentView → app/dispatch.rs match arm chain.

fn wu(id: &str, status: &str) -> codelet_rpc_types::WorkUnitInfo {
    codelet_rpc_types::WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn shift_right_event() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT))
}

/// Drain every queued Action from the App's action channel and dispatch
/// each one synchronously. `App::handle_event` only emits actions onto
/// the channel; the production event loop drains them via `select!` on
/// `action_rx`. Tests must do this manually to observe state changes.
fn drain_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Construct an App that is sitting in ViewMode::Board, with one work
/// unit focused in the given column. Optionally attach `session` to that
/// work unit. Mirrors the test setup pattern from view_board_unit_rpc012.rs.
fn app_in_boardview(unit_id: &str, status: &str, attach: Option<SessionId>) -> App {
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.board_store_mut()
        .replace_work_units(vec![wu(unit_id, status)]);
    app.board_store_mut().set_focused_column(status);
    app.board_store_mut().set_selected_index_for(status, 0);
    if let Some(sid) = attach {
        app.board_store_mut().attach_session(unit_id, sid);
    }
    // App starts in ViewMode::Board by default (Navigator::new), but make
    // it explicit so the test reads top-to-bottom.
    app.navigator_mut().active_view = ViewMode::Board;
    app
}

/// Scenario: BoardView first Shift+Right with unattached work unit mounts
/// CreateSessionDialog OVER BoardView on a single press — and DOES NOT
/// switch active_view to Agent.
///
/// This is the user-reported regression: "doesn't create a new session
/// properly the first time you go shift+right from the board" — the
/// canonical TS behaviour (BoardView.tsx onConfirm callback) is that
/// `setViewMode('agent')` only fires AFTER the dialog is confirmed. The
/// dialog overlays the BoardView in-place; cancelling must leave the
/// user on the Kanban board, not in an empty AgentView.
#[test]
fn boardview_first_shift_right_unattached_mounts_dialog_over_board() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(app.board_store().session_for("AUTH-001").is_none());
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is still ViewMode::Board
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "BoardView Shift+Right with unattached work unit must NOT switch to Agent — \
         dialog overlays BoardView; view switch happens on confirm only"
    );
    // @step And the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "BoardView first Shift+Right must mount CreateSessionDialog on first press"
    );
    // @step And the dialog overlays the BoardView
    assert_eq!(
        app.compositor().topmost_id().as_deref(),
        Some(CREATE_SESSION_DIALOG_ID),
        "dialog must be the topmost compositor layer (overlay on BoardView)"
    );
    Ok(())
}

/// Scenario: BoardView first Shift+Right with attached session jumps
/// directly into AgentView without dialog.
///
/// Guard against over-correction: when a session IS attached, the dialog
/// must NOT appear — the user is navigating into the existing session.
#[test]
fn boardview_first_shift_right_attached_session_no_dialog() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has an attached session "sid-1"
    let mut app = app_in_boardview("AUTH-001", "backlog", Some(SessionId::new("sid-1")));
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        Some(&SessionId::new("sid-1"))
    );

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is ViewMode::Agent
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step And agent_view_store.navigation_target is Some("sid-1")
    assert_eq!(
        app.agent_view_store().navigation_target_session(),
        Some(&SessionId::new("sid-1"))
    );
    // @step And the Compositor does not contain CREATE_SESSION_DIALOG_ID
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "with attached session, Shift+Right must NOT push the dialog — \
         the user is jumping into the existing session"
    );
    Ok(())
}

/// Scenario: Two Shift+Rights from BoardView with unattached work unit
/// remain idempotent (only one CreateSessionDialog instance in the
/// compositor at any time) — and active_view stays Board the WHOLE time.
#[test]
fn boardview_double_shift_right_unattached_is_idempotent_and_stays_on_board() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    let count_after_first = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == CREATE_SESSION_DIALOG_ID)
        .count();
    assert_eq!(
        count_after_first, 1,
        "after first Shift+Right exactly one CreateSessionDialog must be in the compositor"
    );
    // @step And navigator.active_view is still ViewMode::Board
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "first Shift+Right must NOT switch to Agent — dialog overlays BoardView"
    );

    // @step When the user presses Shift+Right again
    // (After the fix, BoardView's input handler is gated off while the
    //  dialog is mounted — but even if a Shift+Right event slips through
    //  the dialog itself or any future path, idempotency must hold.)
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    let count_after_second = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == CREATE_SESSION_DIALOG_ID)
        .count();
    assert_eq!(
        count_after_second, 1,
        "after second Shift+Right exactly one CreateSessionDialog must still be in the compositor \
         (idempotent on CREATE_SESSION_DIALOG_ID)"
    );
    // @step And navigator.active_view is still ViewMode::Board
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "second Shift+Right must also leave the view on Board"
    );
    Ok(())
}

/// Scenario: BoardView Shift+Right then Esc cancels and leaves the user
/// on BoardView (not stranded in an empty AgentView).
#[test]
fn boardview_shift_right_then_esc_returns_to_board() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step And navigator.active_view is still ViewMode::Board
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When the user presses Esc
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain_actions(&mut app);

    // @step Then the Compositor does not contain CREATE_SESSION_DIALOG_ID
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Esc must pop the dialog"
    );
    // @step And navigator.active_view is still ViewMode::Board
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Esc must leave the user on BoardView — not strand them in an empty AgentView"
    );
    Ok(())
}

/// Scenario: BoardView Shift+Right then Enter on Start switches to AgentView
/// and submits create-session — the view switch happens on CONFIRM, not
/// on dialog open.
#[test]
fn boardview_shift_right_then_enter_yes_switches_to_agentview_and_submits() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step And navigator.active_view is still ViewMode::Board
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When the user presses Enter
    let _ = app.handle_event(&key(KeyCode::Enter));

    // @step Then Action::CreateSessionSubmitted with isolated false is emitted
    let mut saw_submitted = false;
    while let Some(action) = app.try_recv_action() {
        let is_submit = matches!(action, Action::CreateSessionSubmitted { isolated: false });
        app.dispatch(action);
        if is_submit {
            saw_submitted = true;
        }
    }
    assert!(
        saw_submitted,
        "Enter on Start must emit Action::CreateSessionSubmitted{{ isolated: false }}"
    );
    // @step And navigator.active_view is ViewMode::Agent
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "view must switch to Agent ON CONFIRM (matches TS BoardView onConfirm callback)"
    );
    // @step And the Compositor does not contain CREATE_SESSION_DIALOG_ID
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// RPC-097 reopen #2 (2026-05-31): BoardView Shift+Right ignores the
// global open-session list and unconditionally opens CreateSessionDialog
// whenever the focused work unit has no attachment. TS canonical
// (BoardView.tsx → useSessionNavigation::handleShiftRight →
// navigateRight → sessionGetNext) consults the global session list
// first. If any open session exists, Shift+Right resumes it.
//
// User report: "it asks me if i want to create a new agent from the
// board when i hit shift+right after I go back to the board with
// shift left and i already have an agent open - so it's not checking
// the active agent list properly."
//
// See spec/attachments/RPC-097/reopen2-active-session-list-not-checked.md
// ─────────────────────────────────────────────────────────────────────

fn shift_left_event() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT))
}

/// Scenario: BoardView Shift+Right with an already-open session
/// resumes that session instead of showing the dialog.
///
/// The user has an open session sid-A in agent_view_store.open_sessions.
/// They focus a work unit on BoardView that has NO attached session and
/// press Shift+Right. The TS canonical behavior is to consult the
/// global session list (sessionGetNext), find sid-A, and navigate to
/// it. The Rust port must mirror that — NO dialog must appear.
#[test]
fn boardview_shift_right_with_open_session_resumes_existing_session_rpc097_reopen2() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(app.board_store().session_for("AUTH-001").is_none());

    // @step And the agent_view_store has one open session "sid-A"
    app.dispatch(Action::SessionCreated(sid("sid-A")));
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // App::dispatch(SessionCreated) may have side-effects on active_view;
    // force it back to Board to model "user pressed Shift+Left to return
    // to BoardView while sid-A remains open".
    app.navigator_mut().active_view = ViewMode::Board;
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is ViewMode::Agent
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "with an open session in agent_view_store, Shift+Right must \
         resume that session — switching active_view to Agent — \
         NOT mount the CreateSessionDialog"
    );
    // @step And agent_view_store.navigation_target is Some("sid-A")
    assert_eq!(
        app.agent_view_store().navigation_target_session(),
        Some(&sid("sid-A")),
        "navigation_target must be set to the resumed session"
    );
    // @step And the Compositor does not contain CREATE_SESSION_DIALOG_ID
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the CreateSessionDialog MUST NOT be mounted when an open \
         session is available to resume — this is the user-reported bug"
    );
    Ok(())
}

/// Scenario: BoardView Shift+Right with two open sessions resumes
/// the first one (sessionGetFirst semantics).
#[test]
fn boardview_shift_right_with_two_open_sessions_resumes_first_rpc097_reopen2() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);

    // @step And the agent_view_store has two open sessions "sid-A" and "sid-B"
    app.dispatch(Action::SessionCreated(sid("sid-A")));
    app.dispatch(Action::SessionCreated(sid("sid-B")));
    assert_eq!(app.agent_view_store().open_sessions().len(), 2);
    app.navigator_mut().active_view = ViewMode::Board;

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is ViewMode::Agent
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step And agent_view_store.navigation_target is Some("sid-A")
    assert_eq!(
        app.agent_view_store().navigation_target_session(),
        Some(&sid("sid-A")),
        "with multiple open sessions, BoardView Shift+Right must resume the FIRST \
         (matches TS sessionGetFirst semantics when no cursor is active)"
    );
    // @step And the Compositor does not contain CREATE_SESSION_DIALOG_ID
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    Ok(())
}

/// Scenario: Full round-trip — user opens agent, returns to board,
/// then Shift+Right resumes the open session (the canonical user
/// flow that surfaced the bug).
#[test]
fn shift_left_then_shift_right_resumes_open_session_rpc097_reopen2() -> TestResult {
    // @step Given an App in AgentView with one open session "sid-A" focused
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.board_store_mut()
        .replace_work_units(vec![wu("AUTH-001", "backlog")]);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    app.dispatch(Action::SessionCreated(sid("sid-A")));
    app.navigator_mut().active_view = ViewMode::Agent;
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When the user presses Shift+Left
    let _ = app.handle_event(&shift_left_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is ViewMode::Board
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Shift+Left at the start of the open-session list must exit to BoardView"
    );

    // @step When the user presses Shift+Right
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is ViewMode::Agent
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Shift+Right from BoardView with an already-open session must resume that session"
    );
    // @step And agent_view_store.navigation_target is Some("sid-A")
    assert_eq!(
        app.agent_view_store().navigation_target_session(),
        Some(&sid("sid-A"))
    );
    // @step And the Compositor does not contain CREATE_SESSION_DIALOG_ID
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "this is the exact user-reported failure mode — round-tripping \
         Shift+Left → Shift+Right must NOT re-prompt the create dialog"
    );
    Ok(())
}

/// Scenario: BoardView Shift+Right with zero open sessions still
/// mounts CreateSessionDialog. Regression guard for RPC-097 reopen #1
/// — ensure the new global-session probe doesn't break the empty
/// state.
#[test]
fn boardview_shift_right_zero_open_sessions_still_mounts_dialog_rpc097_reopen2() -> TestResult {
    // @step Given an App in BoardView with a selected work unit that has no attached session
    let mut app = app_in_boardview("AUTH-001", "backlog", None);
    // @step And the agent_view_store has zero open sessions
    assert!(app.agent_view_store().open_sessions().is_empty());

    // @step When the user presses Shift+Right once
    let _ = app.handle_event(&shift_right_event());
    drain_actions(&mut app);

    // @step Then navigator.active_view is still ViewMode::Board
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "with zero open sessions, RPC-097 reopen #1 contract must hold — \
         dialog overlays BoardView; active_view stays Board"
    );
    // @step And the Compositor contains CREATE_SESSION_DIALOG_ID
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "with zero open sessions, BoardView Shift+Right must still mount the dialog"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Silence unused warnings from EventResult import in older Rust versions
// ─────────────────────────────────────────────────────────────────────
#[allow(dead_code)]
fn _force_event_result_use(_: EventResult) {}
#[allow(dead_code)]
fn _force_session_context_use(_: SessionContext) {}
