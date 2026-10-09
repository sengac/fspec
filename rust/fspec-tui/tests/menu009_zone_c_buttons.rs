//! MENU-009 — the menu bar's Zone C: the right-aligned New Agent /
//! Close Agent action buttons (the agent semantics were superseded by
//! BUG-199: start-a-new-agent + the Esc exit gesture, then by BUG-204:
//! 'Close Agent' ALWAYS mounts the 'Exit Session?' dialog — the
//! state-dependent interrupt / draft-clear branches moved to the
//! physical Esc key only).
//!
//! Feature: spec/features/menubar-zone-c-right-aligned-new-agent-close-agent-buttons.feature
//!          spec/features/bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics.feature
//!          spec/features/bug204-close-agent-button-always-shows-exit-confirmation-dialog.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert).
//!
//! Harnesses:
//! - **Component-level** (scenarios A, B, C, I, L, M, O):
//!   `paint_menu_bar` / `menu_bar_layout` over a `MenuSnapshot` built from
//!   the per-surface `&'static` Zone C slices (`BOARD_ZONE_C`,
//!   `AGENT_ZONE_C`, the mux empty slice) — the shared painter stays
//!   surface-agnostic, so the snapshot IS the surface.
//! - **App-level** (scenarios D, E, F, G, H, J, K, N): App + MockBackend
//!   (the `menu002_board_surface.rs` pattern) — `fresh_app` +
//!   `drain_pending` + full-App render into a TestBackend at 120x24.
//!   Observation points: `app.board_store().menu_focus()` /
//!   `app.navigator().agent.menu_focus()` (the ring),
//!   `app.compositor().contains(CREATE_SESSION_DIALOG_ID)` (the
//!   None-target path + BUG-199 R1), `app.compositor().contains(
//!   EXIT_DIALOG_ID)` (BUG-199 R2), `app.active_view()` /
//!   `app.agent_view_store().navigation_target_session()` (OpenAgentView
//!   resolution), `mock.destroy_session_calls()` (the teardown only runs
//!   after a dialog commit).
//!
//! Ring geometry (the continuous wrap, R3):
//! - board: 7 columns → 4 items → chips → `New Agent` (the LAST stop)
//! - agent: `Board View` → chips → `New Agent` → `Close Agent` → wrap
//!   (MENU-011: the bar paints the items bracketed — `[ New Agent ]` /
//!   `[ Close Agent ]` — and the legacy `[esc]` hint is removed)
//! - mux:  byte-identical to pre-MENU-009 (empty `zone_c` slice)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::layout::menu_bar_layout;
use codelet_fspec_tui::components::menu_bar::{
    build_chips, paint_menu_bar, ChipInput, MenuAction, MenuCategory, MenuChip, MenuFocus,
    MenuLayout, MenuSnapshot, ZoneBCell, ZoneCButton, CATEGORIES,
};
use codelet_fspec_tui::{AgentView, App, FspecBackend, Theme, ViewMode, CREATE_SESSION_DIALOG_ID};
use codelet_rpc_types::{SessionId, SessionStatus, WorkUnitInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;

mod common;
use common::MockBackend;

/// The agent bar's row in the full-App render (the SessionHeader is row 0,
/// the 2-zone bar row 1 — the `menu007_agent_board_view_item.rs` constant).
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

/// True when the cell carries the inverse-video highlight (bg Cyan).
fn is_inverse(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].bg == Color::Cyan
}

fn sid(id: &str) -> SessionId {
    SessionId::new(id.to_string())
}

/// Seed `n` open sessions (s-1..s-n, Idle by default).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(sid(&format!(
            "s-{i}"
        ))));
    }
    drain_pending(app).await;
}

fn make_unit(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
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

/// Seed a small board so the focused column has a selectable card.
async fn seed_units(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog"),
        make_unit("AUTH-002", "testing"),
        make_unit("AUTH-003", "blocked"),
    ]));
    drain_pending(app).await;
}

/// Focus the named column via the existing store seam.
fn focus_column(app: &mut App, name: &str) {
    app.board_store_mut().set_focused_column(name);
}

/// Enter the Agent view on the given session (the RPC-097
/// jump-into-existing-session path).
async fn enter_agent_view(app: &mut App, session: &str) {
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(sid(session))));
    drain_pending(app).await;
}

/// The header's row 3 — the board's 2-zone menu bar row (3 rows below the
/// `Checkpoints:` status row; the `menu002_board_surface.rs` helper).
fn bar_row(buf: &Buffer) -> u16 {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .expect("the checkpoint status row must render");
    status_y + 3
}

// ─────────────────────────────────────────────────────────────────────────
// Component-level helpers (scenarios A, B, C, I, L, M, O)
// ─────────────────────────────────────────────────────────────────────────

/// One Idle `#n` chip with the given WU id suffix (a 1-input
/// `build_chips` — the real builder, so the glyph is live).
fn chip(i: usize, wu: Option<&'static str>) -> MenuChip {
    build_chips(
        &[ChipInput {
            index: (i + 1, 3),
            status: SessionStatus::Idle,
            wu_id: wu.map(str::to_string),
            active: false,
        }],
        0,
    )
    .pop()
    .expect("a single-input build_chips must produce one chip")
}

/// The board-shaped snapshot: the full `CATEGORIES` Zone A + one
/// `Chip(i)` Zone B cell per chip + the board's single `New Agent` Zone C
/// button (the surface's `&'static` slice, the `AGENT_ZONE_A` precedent).
fn board_snapshot(chips: Vec<MenuChip>, focus: Option<MenuFocus>) -> MenuSnapshot {
    let zone_b = chips
        .iter()
        .enumerate()
        .map(|(i, _)| ZoneBCell::Chip(i))
        .collect();
    MenuSnapshot {
        zone_a: CATEGORIES,
        focus,
        open_menu: None,
        zone_b,
        chips,
        zone_c: codelet_fspec_tui::views::board::menu_snapshot::BOARD_ZONE_C,
        clock_ms: 0,
    }
}

/// The agent-shaped snapshot: the single `Board View` Zone A item plus
/// the chips plus the agent's two Zone C buttons (New Agent, then
/// Close Agent — MENU-011 R3: the labels are stored BRACKETED, the
/// legacy `[esc]` hint removed). The `&'static` slices mirror the
/// `AGENT_ZONE_A` / `AGENT_ZONE_C` constants the production snapshot
/// builder reads (the component-level test addresses the labels
/// directly).
fn agent_snapshot(chips: Vec<MenuChip>, focus: Option<MenuFocus>) -> MenuSnapshot {
    const AGENT_ZONE_A: &[MenuCategory] = &[MenuCategory {
        id: "board-view",
        label: "Board View",
        entries: &[],
    }];
    static AGENT_ZONE_C: &[ZoneCButton] = &[
        ZoneCButton {
            label: "[ New Agent ]",
            action: MenuAction::NewAgent,
        },
        ZoneCButton {
            label: "[ Close Agent ]",
            action: MenuAction::CloseAgent,
        },
    ];
    let zone_b = chips
        .iter()
        .enumerate()
        .map(|(i, _)| ZoneBCell::Chip(i))
        .collect();
    MenuSnapshot {
        zone_a: AGENT_ZONE_A,
        focus,
        open_menu: None,
        zone_b,
        chips,
        zone_c: AGENT_ZONE_C,
        clock_ms: 0,
    }
}

/// The mux-shaped snapshot: the full `CATEGORIES` Zone A + the pane view
/// labels (Board active, Files) + chips + the EMPTY Zone C (Q1).
fn mux_snapshot(chips: Vec<MenuChip>) -> MenuSnapshot {
    let zone_b = vec![
        ZoneBCell::View {
            label: "Board",
            active: true,
        },
        ZoneBCell::View {
            label: "Files",
            active: false,
        },
    ]
    .into_iter()
    .chain(chips.iter().enumerate().map(|(i, _)| ZoneBCell::Chip(i)))
    .collect();
    MenuSnapshot {
        zone_b,
        chips,
        // Q1: the mux top bar carries NO Zone C — the empty slice keeps
        // the ring / layout / paint byte-identical to pre-MENU-009.
        ..Default::default()
    }
}

/// Paint a 1-row bar into a `w`-column buffer and return the layout.
fn render_bar(snap: &MenuSnapshot, w: u16) -> (Buffer, MenuLayout) {
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    let layout = paint_menu_bar(area, &mut buf, snap, &Theme::default())
        .expect("layout for a non-zero area");
    (buf, layout)
}

/// The trimmed row 0 of a 1-row bar buffer.
fn bar_line(buf: &Buffer, w: u16) -> String {
    (0..w)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Render the view-level agent pane into a `w`-column TestBackend buffer
/// (the `menu007_agent_board_view_item.rs` zero-sessions pattern — the
/// App cannot enter the Agent view with an empty open-session list).
fn render_agent_pane(w: u16) -> Buffer {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut view = AgentView::new(tx);
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    let mut term = Terminal::new(TestBackend::new(w, 24)).expect("Terminal::new");
    term.draw(|frame| view.render_with_store(frame.area(), frame.buffer_mut(), &mut store))
        .expect("draw");
    term.backend().buffer().clone()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The board bar paints a right-aligned "New Agent" button and no "Close Agent"
#[test]
fn scenario_the_board_bar_paints_a_right_aligned_new_agent_button_and_no_close_agent() {
    // @step Given a board menu bar snapshot with 1 open session and no ring focus
    let snap = board_snapshot(vec![chip(0, None)], None);

    // @step When the bar is painted into a 120-column row
    let (buf, layout) = render_bar(&snap, 120);
    let row = bar_line(&buf, 120);

    // @step Then the row ends with the right-aligned "[ New Agent ]" Zone C button
    assert!(
        row.ends_with("[ New Agent ]"),
        "the row must end with the right-aligned New Agent button (MENU-011: bracketed): {row:?}"
    );
    assert_eq!(
        layout.zone_c_rects.len(),
        1,
        "exactly one Zone C rect (the board's single button)"
    );
    // The button is right-aligned at the inner edge (1-cell R1 padding):
    // the last Zone C cell ends at x = width - 1.
    let last = &layout.zone_c_rects[0];
    assert_eq!(
        last.x + last.width,
        119,
        "New Agent must sit at the inner right edge"
    );

    // @step And the row contains no "Close Agent" button
    assert!(
        !row.contains("Close Agent"),
        "the board bar must not paint Close Agent: {row:?}"
    );

    // @step And the left-anchored Zone A items and the Zone B chip are unchanged
    assert!(
        row.starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"),
        "the Zone A items keep their left-anchored order (MENU-011: bracketed): {row:?}"
    );
    assert!(row.contains('│'), "the dim zone separator: {row:?}");
    assert!(row.contains("[ #1 ● ]"), "the session chip: {row:?}");
}

/// Scenario: The agent bar paints "New Agent" then "Close Agent" right-aligned
/// (MENU-011: both buttons paint bracketed; the legacy `[esc]` hint on
/// the Close Agent label was removed)
#[test]
fn scenario_the_agent_bar_paints_new_agent_then_close_agent_esc_right_aligned() {
    // @step Given an agent menu bar snapshot with 0 open sessions and no ring focus
    let snap = agent_snapshot(Vec::new(), None);

    // @step When the agent bar is painted into a 100-column row
    let (buf, layout) = render_bar(&snap, 100);
    let row = bar_line(&buf, 100);

    // @step Then the row ends with "New Agent" followed by "Close Agent" right-aligned
    // (MENU-011: the buttons paint BRACKETED — the 2-cell gap is kept,
    // the legacy `[esc]` hint is REMOVED)
    assert!(
        row.ends_with("[ New Agent ]  [ Close Agent ]"),
        "the row must end with both Zone C buttons (2-cell gap; MENU-011 bracketed): {row:?}"
    );
    assert_eq!(layout.zone_c_rects.len(), 2, "two Zone C rects");
    let (na, ca) = (&layout.zone_c_rects[0], &layout.zone_c_rects[1]);
    assert_eq!(
        na.x + na.width + 2,
        ca.x,
        "the 2-cell inter-button gap (Zone B visual parity)"
    );
    assert_eq!(
        ca.x + ca.width,
        99,
        "Close Agent ends at the inner right edge (MENU-011: 15-cell bracketed label)"
    );

    // @step And the left-anchored "Board View" item is unchanged
    assert!(
        row.starts_with(" [ Board View ]"),
        "the agent bar's single Zone A item stays left-anchored (MENU-011: bracketed): {row:?}"
    );
    assert!(
        !row.contains('│') && !row.contains('#'),
        "no separator and no chips without sessions: {row:?}"
    );
}

/// Scenario: The mux bar paints no Zone C buttons — the row is byte-identical to pre-MENU-009
#[test]
fn scenario_the_mux_bar_paints_no_zone_c_buttons_the_row_is_byte_identical_to_pre_menu_009() {
    // @step Given a mux menu bar snapshot with 2 panes and 1 open session
    let snap = mux_snapshot(vec![chip(0, None)]);
    assert!(
        snap.zone_c.is_empty(),
        "Q1: the mux Zone C is the empty slice"
    );

    // @step When the mux bar is painted into a 120-column row
    let (buf, layout) = render_bar(&snap, 120);
    let row = bar_line(&buf, 120);

    // @step Then the row contains no Zone C button
    assert!(
        !row.contains("New Agent") && !row.contains("Close Agent"),
        "no Zone C button text on the mux bar: {row:?}"
    );
    assert!(layout.zone_c_rects.is_empty(), "no Zone C rects: {row:?}");

    // @step And the Zone A items, the pane labels and the chip paint exactly as before
    assert!(
        row.starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"),
        "the Zone A items (MENU-011: bracketed): {row:?}"
    );
    assert!(row.contains("Board"), "the active pane label: {row:?}");
    assert!(row.contains("Files"), "the second pane label: {row:?}");
    assert!(row.contains("[ #1 ● ]"), "the global session chip: {row:?}");
}

/// Scenario: Enter on the board's "New Agent" button activates the "."-key semantics
#[tokio::test]
async fn scenario_enter_on_the_board_new_agent_button_mounts_the_create_session_dialog_when_no_open_sessions(
) {
    // @step Given the board bar is painted with the "New Agent" Zone C button and no open sessions
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    render_app(&mut app); // paint the bar (the ring feeds at dispatch tick)
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Left from the first column must land on the New Agent button (the ring's last stop)"
    );

    // @step When the ring focuses "New Agent" and I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the board (the "."-key None-target path)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "with no open sessions the '.'-key None-target path must mount the CreateSessionDialog"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the user stays on the board until the dialog is confirmed (RPC-097 reopen #1)"
    );

    // @step And the board bar's ring focus clears
    assert!(
        app.board_store().menu_focus().is_none(),
        "executing the button is a 'leave the bar' gesture — the ring focus clears"
    );
}

/// Scenario: A left click on the board's "New Agent" button activates it
#[tokio::test]
async fn scenario_a_left_click_on_the_board_new_agent_button_activates_it() {
    // @step Given the board bar is painted with the "New Agent" Zone C button
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    let buf = render_app(&mut app); // cache the bar geometry
    let y = bar_row(&buf);
    let x = find_x(&buf, y, "New Agent").expect("New Agent on the bar row");

    // @step When I left-click the "New Agent" button
    app.handle_event(&click(x, y));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the board (BUG-203: the button
    // ALWAYS starts a new agent — it never jumps into the selected unit's session)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the click must mount the CreateSessionDialog (BUG-199 agent-bar parity)"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the user stays on the board until the dialog is confirmed (RPC-097 reopen #1)"
    );
    assert!(
        app.agent_view_store().navigation_target_session().is_none(),
        "no session may be resumed / re-entered (BUG-203)"
    );

    // @step And the board bar's ring focus clears
    assert!(
        app.board_store().menu_focus().is_none(),
        "executing the button must clear the board bar's ring focus"
    );
}

/// Scenario: Enter on the agent's "New Agent" button starts a new agent (BUG-199 R1)
/// (the board-button parity — the MENU-009 "jump into / resume the current
/// session" semantics are superseded).
#[tokio::test]
async fn scenario_enter_on_the_agents_new_agent_button_mounts_the_create_session_dialog() {
    // @step Given the agent bar is painted with "New Agent" and "Close Agent" and 1 open session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the agent bar geometry (chips + Zone C rects)
                          // Ring: Board View → chip #1 → New Agent.
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Right x2 from 'Board View' (past the chip) must land on 'New Agent'"
    );

    // @step When the ring focuses the agent bar's "New Agent" and I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the Agent view (the current session is untouched)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "'New Agent' must mount the CreateSessionDialog (start a NEW agent)"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the user stays on the Agent view until the dialog is confirmed"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1")),
        "the current session must be untouched by the button"
    );

    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus"
    );
}

/// Scenario: Enter on the agent's "Close Agent" button shows the same exit
/// confirmation as Esc (MENU-011: the `[esc]` hint was removed) (BUG-204
/// supersedes BUG-199 R2 — the idle-session dialog behavior is kept;
/// the state-dependent branches moved to the physical Esc key).
#[tokio::test]
async fn scenario_enter_on_the_agents_close_agent_button_shows_the_esc_exit_confirmation() {
    // @step Given the agent view shows 1 open idle session and the agent bar is painted
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    render_app(&mut app); // cache the agent bar geometry (chips + Zone C rects)
                          // Ring: Board View → chip #1 → New Agent → Close Agent.
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    for _ in 0..3 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        drain_pending(&mut app).await;
    }
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "Left then Right x3 must land on the 'Close Agent' button"
    );
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "no destroy before the activation"
    );

    // @step When the ring focuses "Close Agent" and I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the "Exit Session?" confirmation dialog (Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "'Close Agent' must show the Esc exit confirmation dialog"
    );
    // @step And the session is not destroyed until an option is committed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the button must never destroy the session directly"
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

/// Scenario: A left click on the agent's "Close Agent" button shows the
/// same exit confirmation as Esc (MENU-011: the label paints bracketed —
/// `[ Close Agent ]` — and the legacy `[esc]` hint was removed) (BUG-199 R2 — the MENU-009 direct
/// teardown is superseded).
#[tokio::test]
async fn scenario_a_left_click_on_the_agents_close_agent_button_shows_the_esc_exit_confirmation() {
    // @step Given the agent view shows 1 open idle session and the agent bar is painted
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    let buf = render_app(&mut app); // cache the agent bar geometry
    let x = find_x(&buf, AGENT_BAR_ROW, "Close Agent")
        .expect("Close Agent on the bar row (MENU-011: bracketed, [esc] hint removed)");

    // @step When I left-click the "Close Agent" button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the "Exit Session?" confirmation dialog (Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "the click must show the Esc exit confirmation dialog (BUG-199 R2)"
    );
    // @step And the session is not destroyed until an option is committed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the button must never destroy the session directly"
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

/// Scenario: A focused Zone C button carries the inverse-video ring highlight
#[test]
fn scenario_a_focused_zone_c_button_carries_the_inverse_video_ring_highlight() {
    // @step Given a board menu bar snapshot with the ring focus on "New Agent"
    let snap = board_snapshot(vec![chip(0, None)], Some(MenuFocus::ZoneC(0)));

    // @step When the bar is painted into a 120-column row
    let (buf, layout) = render_bar(&snap, 120);

    // @step Then the "New Agent" button's cells are styled bg Cyan, fg Black, bold
    let rect = &layout.zone_c_rects[0];
    for cx in rect.x..rect.x + rect.width {
        let cell = &buf[(cx, 0)];
        assert_eq!(cell.bg, Color::Cyan, "cell {cx} must be inverse bg");
        assert_eq!(cell.fg, Color::Black, "cell {cx} must be inverse fg");
        assert!(
            cell.modifier.contains(ratatui::style::Modifier::BOLD),
            "cell {cx} must be bold"
        );
    }

    // @step And no other Zone C or Zone A cell is inverted
    for rect in layout.item_rects.iter().chain(layout.cell_rects.iter()) {
        for cx in rect.x..rect.x + rect.width {
            assert_ne!(
                buf[(cx, 0)].bg,
                Color::Cyan,
                "an unfocused Zone A/Zone B cell at {cx} must not be inverted"
            );
        }
    }
}

/// Scenario: The ring walks last chip -> "New Agent" -> columns on the board
#[tokio::test]
async fn scenario_the_ring_walks_last_chip_to_new_agent_to_columns_on_the_board() {
    // @step Given the board ring has 7 columns, 4 items and 1 chip
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "blocked"); // the last column (ring slot 6)
    render_app(&mut app);
    // Right x5 from the last column: 4 items + 1 chip → the last chip.
    for _ in 0..5 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        drain_pending(&mut app).await;
    }
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Right x5 from the last column must land on chip #1 (the last chip)"
    );

    // @step When the ring walks Right from the last chip
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then "New Agent" (the only Zone C button) holds the ring focus
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Right from the last chip must land on New Agent"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let na_x = find_x(&buf, y, "New Agent").expect("New Agent on the bar row");
    assert!(
        is_inverse(&buf, na_x + 10, y), // MENU-011: bracketed "[ New Agent ]"
        "the New Agent button must paint inverse-video on row {y}:\n{}",
        row_text(&buf, y)
    );

    // @step When the ring walks Right again
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the focus wraps to the first kanban column (no bar highlight)
    assert!(
        app.board_store().menu_focus().is_none(),
        "Right from New Agent must wrap back to a column"
    );
    assert_eq!(app.board_store().focused_column_index(), 0);

    // @step When the ring walks Left from the first item
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then "New Agent" holds the ring focus
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Left from the first column must land on the New Agent button (the reverse wrap)"
    );
}

/// Scenario: The ring wraps through both Zone C buttons on the agent bar
#[tokio::test]
async fn scenario_the_ring_wraps_through_both_zone_c_buttons_on_the_agent_bar() {
    // @step Given the agent bar has the "Board View" item, 1 chip and 2 Zone C buttons
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the agent bar geometry (chips + Zone C rects)
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE)); // Item(0) = 'Board View'
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)); // → chip #1
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Right from 'Board View' must land on the last chip"
    );

    // @step When the ring walks Right from the last chip
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then "New Agent" holds the ring focus
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Right from the last chip must land on 'New Agent'"
    );

    // @step When the ring walks Right once more
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then "Close Agent" holds the ring focus
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "Right from 'New Agent' must land on 'Close Agent'"
    );

    // @step When the ring walks Right once more
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then "Board View" holds the ring focus (the wrap)
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::Item(0)),
        "Right from 'Close Agent' must wrap back to 'Board View'"
    );

    // @step When the ring walks Left from "Board View"
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then "Close Agent" holds the ring focus (the reverse wrap)
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "Left from 'Board View' must wrap to the last Zone C button"
    );
}

/// Scenario: Zone C drops before Zone B content when the row cannot afford it
#[test]
fn scenario_zone_c_drops_before_zone_b_content_when_the_row_cannot_afford_it() {
    // @step Given a board menu bar snapshot with 1 chip bound to a long work-unit id
    let snap = board_snapshot(vec![chip(0, Some("MENU-001"))], None);

    // @step When the bar is painted into a 60-column row
    // (MENU-011: the bracketed Zone A is 42 cells — a 40-col area no
    // longer fits even the absolute minimum; 60 cols = 58 inner: the
    // level-0 row (42 + 3 separator + 17 chip = 62) does not fit, so
    // the WU id suffix is the first Zone B ladder step dropped; the
    // level-1 row (53) plus the 2-cell gap plus the 13-cell bracketed
    // button (68) still does not fit — Zone C drops BEFORE any further
    // Zone B truncation, R5.)
    let (buf, layout) = render_bar(&snap, 60);
    let row = bar_line(&buf, 60);

    // @step Then the row contains no Zone C button
    assert!(
        !row.contains("New Agent"),
        "Zone C must drop when the row cannot afford it: {row:?}"
    );
    assert!(
        layout.zone_c_rects.is_empty(),
        "no Zone C rects when the zone dropped: {row:?}"
    );

    // @step And the chip still paints (Zone B ladder step 1, WU id dropped)
    assert_eq!(
        layout.level, 1,
        "the ladder must have stopped at step 1 (WU id dropped), Zone C before any further drop"
    );
    assert!(
        row.contains("[ #1 ● ]"),
        "the chip must still paint: {row:?}"
    );
    assert!(
        !row.contains("MENU-001"),
        "the WU id suffix is the step-1 truncation: {row:?}"
    );

    // @step And the painted row fits within the area width
    if let Some(last) = layout.cell_rects.last() {
        assert!(
            last.x + last.width <= 60,
            "the painted row must fit within the area width"
        );
    }
}

// Scenario: The Zone C truncation bound holds for any width and chip count (proptest)
proptest::proptest! {
    #[test]
    fn scenario_the_zone_c_truncation_bound_holds_for_any_width_and_chip_count(
        width in 3u16..=400,
        chip_count in 0usize..=24,
    ) {
        // @step Given a board menu bar snapshot with a Zone C "New Agent" button and any number of chips
        let chips: Vec<MenuChip> = (0..chip_count)
            .map(|i| chip(i, Some("A-BIG-UNIT-ID")))
            .collect();
        let snap = board_snapshot(chips, None);

        // @step When the bar is painted into any render area width
        // (the geometry pass — the painted row is the layout's rects)
        if let Some(layout) = menu_bar_layout(Rect::new(0, 0, width, 1), &snap) {
            // @step Then the painted row (Zone A items, Zone B cells AND Zone C rects) never exceeds the area width
            // Too-tight areas legitimately paint nothing — only assert the
            // bound when a layout was produced.
            for rect in layout
                .item_rects
                .iter()
                .chain(layout.cell_rects.iter())
                .chain(layout.zone_c_rects.iter())
            {
                assert!(
                    rect.x + rect.width <= width,
                    "a Zone C rect (or A/B cell) overflows: x {} w {} at width {}",
                    rect.x,
                    rect.width,
                    width
                );
            }
        }
    }
}

/// Scenario: A Zone C click with an open dropdown does NOT fire the click-away dismiss
#[tokio::test]
async fn scenario_a_zone_c_click_with_an_open_dropdown_does_not_fire_the_click_away_dismiss() {
    // @step Given the board bar's Kanban dropdown is open and the "New Agent" button is painted
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)); // → Item(0) = Kanban
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE)); // open the Kanban dropdown
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the Kanban dropdown must be open at row 0"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let x = find_x(&buf, y, "New Agent").expect("New Agent on the bar row");

    // @step When I left-click the "New Agent" button
    app.handle_event(&click(x, y));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the board (BUG-203: the button
    // ALWAYS starts a new agent — it never resumes the first open session)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the click must mount the CreateSessionDialog (BUG-199 agent-bar parity)"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the user stays on the board until the dialog is confirmed (RPC-097 reopen #1)"
    );

    // @step And NO MenuDismissBar gesture fires (the click is not "outside")
    // Observation: the click is ON the bar row (the button owns its rect),
    // so the BUG-196/197 `close_outside` gesture — which only fires for
    // clicks OFF the bar row and outside the panel/item/Zone C rects —
    // must not have run: the board's own state transitions show a SINGLE
    // close (the execute path's `dismiss_menu`), and the click did NOT
    // fall through to a content selection (the card selection is intact).
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        0,
        "a click-away fall-through must not have re-selected a card"
    );

    // @step And the dropdown closes and the bar de-selects exactly once (the execute path)
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "the dropdown must be closed (the execute path closed it)"
    );
    assert!(
        app.board_store().menu_focus().is_none(),
        "the bar must be de-selected (the execute path de-selected it)"
    );
}

/// Scenario: With no open sessions the agent bar still shows both Zone C buttons
#[test]
fn scenario_with_no_open_sessions_the_agent_bar_still_shows_both_zone_c_buttons() {
    // @step Given an agent menu bar snapshot with zero open sessions
    // (view-level pane — the App cannot enter the Agent view with an
    // empty open-session list; the zero-chip snapshot is the same shape)

    // @step When the agent bar is painted into a 80-column row
    let buf = render_agent_pane(80);
    let row = row_text(&buf, AGENT_BAR_ROW);

    // @step Then the row ends with "[ New Agent ]" followed by
    // "[ Close Agent ]" right-aligned (MENU-011: bracketed, the
    // `[esc]` hint removed)
    assert!(
        row.ends_with("[ New Agent ]  [ Close Agent ]"),
        "both Zone C buttons must paint even with an empty chip list: {row:?}"
    );

    // @step And the buttons are independent of the (empty) chip list
    assert!(
        row.contains("Board View"),
        "the left-anchored 'Board View' item: {row:?}"
    );
    assert!(
        !row.contains('│') && !row.contains('#'),
        "no separator and no chips without sessions: {row:?}"
    );
}
