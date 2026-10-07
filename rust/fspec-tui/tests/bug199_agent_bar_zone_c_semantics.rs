//! BUG-199 — agent-bar Zone C button semantics: 'New Agent' starts a new
//! agent (the Create Session dialog); 'Close Agent' is the Esc exit
//! gesture (the RPC-098 cascade), not a direct teardown.
//!
//! Feature: spec/features/bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert).
//!
//! Harness: App + MockBackend (the `menu009_zone_c_buttons.rs` pattern) —
//! `fresh_app` + `drain_pending` + full-App render into a TestBackend at
//! 120x24. Observation points:
//! - `app.compositor().contains(CREATE_SESSION_DIALOG_ID)` (R1: New Agent
//!   mounts the dialog over the Agent view);
//! - `mock.interrupt_calls()` / `mock.last_interrupt()` (R2 L4/L5: the
//!   cascade's interrupt branch), `app.navigator().agent.input.value()`
//!   (R2 L6: the draft-clear branch),
//!   `app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID)` (R2 L7: the
//!   'Exit Session?' dialog branch), `mock.destroy_session_calls()`
//!   (the button NEVER destroys directly);
//! - `app.agent_view_store().current_session()` / `app.active_view()`
//!   (the current session is never disturbed by 'New Agent');
//! - `app.navigator().agent.menu_focus()` (the 'leave the bar' gesture).
//!
//! Pane geometry (120x24, single agent view): the SessionHeader is row 0,
//! the 2-zone agent bar row 1 (the `menu009` AGENT_BAR_ROW constant).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::{AgentView, App, FspecBackend, Theme, ViewMode, CREATE_SESSION_DIALOG_ID};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

/// The agent bar's row in the full-App render (the SessionHeader is row 0,
/// the 2-zone bar row 1 — the `menu009_zone_c_buttons.rs` constant).
const AGENT_BAR_ROW: u16 = 1;

/// The stable compositor id of the RPC-098 'Exit Session?' dialog.
const EXIT_DIALOG_ID: &str = "exit-confirmation-dialog";

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Render the full App (navigator + compositor) into a 120x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The cell x of the first occurrence of `needle` on row `y`.
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

fn sid(id: &str) -> SessionId {
    SessionId::new(id.to_string())
}

/// Seed `n` open sessions (s-1..s-n, Idle by default — `append_session`
/// focuses the LAST, so the current session is s-n).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(sid(&format!(
            "s-{i}"
        ))));
    }
    drain_pending(app).await;
}

/// Enter the Agent view on the given session (the RPC-097
/// jump-into-existing-session path).
async fn enter_agent_view(app: &mut App, session: &str) {
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(sid(session))));
    drain_pending(app).await;
}

/// Walk the agent ring to Zone C button `button` (0 = New Agent,
/// 1 = Close Agent [esc]) from the empty input: Left enters the bar at
/// 'Board View' (ring pos 0), then Right walks the chips and the
/// buttons (ring pos of ZoneC(i) = 1 + chip_count + i).
async fn ring_to_zone_c(app: &mut App, chip_count: usize, button: usize) {
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(app).await;
    for _ in 0..1 + chip_count + button {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        drain_pending(app).await;
    }
}

/// Await `predicate` for up to a second (the cascade's interrupt branch
/// spawns the backend `interrupt` task — the `menu009` helper).
async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

/// Render the view-level agent pane into a `w`-column TestBackend buffer
/// (the `menu009` zero-sessions pattern — the App cannot enter the Agent
/// view with an empty open-session list).
fn render_agent_pane(w: u16) -> Buffer {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut view = AgentView::new(tx);
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    let mut term = Terminal::new(TestBackend::new(w, 24)).expect("Terminal::new");
    term.draw(|frame| view.render_with_store(frame.area(), frame.buffer_mut(), &mut store))
        .expect("draw");
    term.backend().buffer().clone()
}

/// The board-shaped snapshot: the full `CATEGORIES` Zone A + one
/// `Chip(i)` Zone B cell per chip + the board's single `New Agent`
/// Zone C button (the surface's `&'static` slice).
fn board_snapshot(
    chips: Vec<codelet_fspec_tui::components::menu_bar::MenuChip>,
) -> codelet_fspec_tui::components::menu_bar::MenuSnapshot {
    use codelet_fspec_tui::components::menu_bar::{MenuSnapshot, ZoneBCell, CATEGORIES};
    let zone_b = chips
        .iter()
        .enumerate()
        .map(|(i, _)| ZoneBCell::Chip(i))
        .collect();
    MenuSnapshot {
        zone_a: CATEGORIES,
        focus: None,
        open_menu: None,
        zone_b,
        chips,
        zone_c: codelet_fspec_tui::views::board::menu_snapshot::BOARD_ZONE_C,
        clock_ms: 0,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Enter on the agent's 'New Agent' button mounts the Create Session dialog
#[tokio::test]
async fn scenario_enter_on_the_agents_new_agent_button_mounts_the_create_session_dialog() {
    // @step Given the agent view shows 2 open sessions and the agent bar is painted
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enter_agent_view(&mut app, "s-2").await;
    render_app(&mut app); // paint the bar (the ring feeds at dispatch tick)

    // @step When the ring focuses the agent bar's 'New Agent' and I press Enter
    ring_to_zone_c(&mut app, 2, 0).await;
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "the ring must land on the agent bar's 'New Agent' button"
    );
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the Agent view
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "'New Agent' must mount the CreateSessionDialog (BUG-199 R1)"
    );
    // @step And the current session is unchanged and the view stays the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the user stays on the Agent view until the dialog is confirmed"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-2")),
        "the current session must be untouched by the button"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus"
    );
}

/// Scenario: A left click on the agent's 'New Agent' button mounts the Create Session dialog
#[tokio::test]
async fn scenario_a_left_click_on_the_agents_new_agent_button_mounts_the_create_session_dialog() {
    // @step Given the agent view shows 2 open sessions and the agent bar is painted
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enter_agent_view(&mut app, "s-2").await;
    let buf = render_app(&mut app); // cache the agent bar geometry
    let x = find_x(&buf, AGENT_BAR_ROW, "New Agent").expect("New Agent on the bar row");

    // @step When I left-click the 'New Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the Agent view
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the click must mount the CreateSessionDialog (BUG-199 R1)"
    );
    // @step And the current session is unchanged and the view stays the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the user stays on the Agent view until the dialog is confirmed"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-2")),
        "the current session must be untouched by the button"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus"
    );
}

/// Scenario: 'Close Agent' on a running session interrupts the run like the first Esc
#[tokio::test]
async fn scenario_close_agent_on_a_running_session_interrupts_the_run_like_the_first_esc() {
    // @step Given the agent view shows 1 open session that is RUNNING
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // Deterministic status fold (the `keyboard_cascade_rpc051` pattern —
    // the store seam instead of the broadcast subscriber task).
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = find_x(&buf, AGENT_BAR_ROW, "Close Agent [esc]").unwrap_or_else(|| {
        panic!(
            "the 'Close Agent [esc]' button must paint on the bar row:\n{}",
            row_text(&buf, AGENT_BAR_ROW)
        )
    });

    // @step When I left-click the 'Close Agent [esc]' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await; // dispatch MenuZoneCActivate (the bus)
                                   // @step Then the run is interrupted and the view stays the Agent view
    wait_until(
        || mock.interrupt_calls() == 1,
        "backend.interrupt to fire (the L4/L5 Esc branch)",
    )
    .await;
    assert_eq!(mock.last_interrupt(), Some(sid("s-1")));
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the interrupt branch must keep the Agent view (RPC-051 L4/L5)"
    );
    // @step And no exit confirmation dialog is shown and the session is not destroyed
    assert!(
        !app.compositor().contains(EXIT_DIALOG_ID),
        "the running branch must NOT show the exit confirmation dialog"
    );
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the button must never destroy the session directly (BUG-199 R2)"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus"
    );
}

/// Scenario: 'Close Agent' with a non-empty input clears the draft like Esc level 6
#[tokio::test]
async fn scenario_close_agent_with_a_non_empty_input_clears_the_draft_like_esc_level_6() {
    // @step Given the agent view shows 1 open idle session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // @step And the input buffer contains a draft
    app.navigator_mut().agent.input.set_value("hello world");
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = find_x(&buf, AGENT_BAR_ROW, "Close Agent [esc]").unwrap_or_else(|| {
        panic!(
            "the 'Close Agent [esc]' button must paint on the bar row:\n{}",
            row_text(&buf, AGENT_BAR_ROW)
        )
    });

    // @step When I left-click the 'Close Agent [esc]' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the input buffer is cleared and the view stays the Agent view
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "the L6 Esc branch must clear the draft (BUG-199 R2)"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the L6 branch must keep the Agent view"
    );
    // @step And no exit confirmation dialog is shown
    assert!(
        !app.compositor().contains(EXIT_DIALOG_ID),
        "a non-empty draft short-circuits before the dialog (RPC-095 L6)"
    );
}

/// Scenario: 'Close Agent' on an idle session shows the same exit confirmation as Esc
#[tokio::test]
async fn scenario_close_agent_on_an_idle_session_shows_the_same_exit_confirmation_as_esc() {
    // @step Given the agent view shows 1 open idle session with an empty input
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = find_x(&buf, AGENT_BAR_ROW, "Close Agent [esc]").unwrap_or_else(|| {
        panic!(
            "the 'Close Agent [esc]' button must paint on the bar row:\n{}",
            row_text(&buf, AGENT_BAR_ROW)
        )
    });
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "no destroy before the activation"
    );

    // @step When I left-click the 'Close Agent [esc]' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the 'Exit Session?' confirmation dialog (Detach / Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "the idle branch must push the ExitConfirmationDialog (BUG-199 R2, RPC-098 L7)"
    );
    // @step And the session is not destroyed until an option is committed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the dialog must gate the teardown — no direct destroy"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the user stays on the Agent view behind the dialog"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus"
    );
}

/// Scenario: The agent bar paints 'New Agent' then 'Close Agent [esc]' right-aligned
#[test]
fn scenario_the_agent_bar_paints_new_agent_then_close_agent_esc_right_aligned() {
    // @step Given an agent menu bar snapshot with 0 open sessions and no ring focus
    // (view-level pane — the App cannot enter the Agent view with an
    // empty open-session list; the zero-chip snapshot is the same shape)

    // @step When the agent bar is painted into a 120-column row
    let buf = render_agent_pane(120);
    let row = row_text(&buf, AGENT_BAR_ROW);

    // @step Then the row ends with 'New Agent' followed by 'Close Agent [esc]' right-aligned
    assert!(
        row.ends_with("New Agent  Close Agent [esc]"),
        "both Zone C buttons must paint, the second with the [esc] hint: {row:?}"
    );

    // @step And the left-anchored 'Board View' item is unchanged
    assert!(
        row.contains("Board View"),
        "the left-anchored 'Board View' item: {row:?}"
    );
    assert!(
        !row.contains('│') && !row.contains('#'),
        "no separator and no chips without sessions: {row:?}"
    );
}

/// Scenario: The board bar still paints a single 'New Agent' button
#[test]
fn scenario_the_board_bar_still_paints_a_single_new_agent_button() {
    // @step Given a board menu bar snapshot with 1 open session and no ring focus
    use codelet_fspec_tui::components::menu_bar::{
        build_chips, paint_menu_bar, ChipInput, MenuChip,
    };
    let chip: Vec<MenuChip> = build_chips(
        &[ChipInput {
            index: (1, 1),
            status: SessionStatus::Idle,
            wu_id: None,
            active: false,
        }],
        0,
    );
    let snap = board_snapshot(chip);

    // @step When the bar is painted into a 120-column row
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 120, 1);
    let mut buf = Buffer::empty(area);
    paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout for a non-zero area");
    let row = row_text(&buf, 0);

    // @step Then the row ends with the right-aligned 'New Agent' Zone C button
    assert!(
        row.ends_with("New Agent"),
        "the board's single right-aligned button is unchanged: {row:?}"
    );
    // @step And the row contains no 'Close Agent' button
    assert!(
        !row.contains("Close Agent"),
        "the board bar must not paint a Close Agent button: {row:?}"
    );
}
