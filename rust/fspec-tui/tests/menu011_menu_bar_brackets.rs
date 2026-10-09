//! MENU-011 — the menu bar's bracketed clickable elements in all zones.
//!
//! Feature: spec/features/menu-bar-bracketed-clickable-elements-in-all-zones-zone-a-items-zone-b-cells-zone-c-buttons-remove-close-agent-esc-hint.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert).
//!
//! Harnesses:
//! - **Component-level** (scenarios 1, 2, 3, 4, 5, 6, 7, 11, 12, 13):
//!   `paint_menu_bar` / `menu_bar_layout` over `MenuSnapshot`s built from
//!   the per-surface `&'static` slices (`BOARD_ZONE_C`,
//!   `AGENT_ZONE_C`, the mux empty slice) — the shared painter stays
//!   surface-agnostic, so the snapshot IS the surface.
//! - **App-level** (scenarios 8, 9, 10): App + MockBackend (the
//!   `menu009_zone_c_buttons.rs` pattern) — `fresh_app` + `drain_pending`
//!   + full-App render into a TestBackend at 120x24.
//!
//! Bracketing rule (R1/R2/R3): every clickable bar element paints with
//! square brackets and a space between the word and each bracket —
//! `[ Kanban ]`, `[ #1 ● ]`, `[ Board ]`, `[ New Agent ]`,
//! `[ Close Agent ]` — and the agent bar's `[esc]` hint is gone.
//! Brackets are part of the button: width, hit-test rects and the
//! inverse-video focus highlight all cover them (R4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::dropdown::dropdown_rect;
use codelet_fspec_tui::components::menu_bar::dropdown_paint::render_menu_dropdown;
use codelet_fspec_tui::components::menu_bar::layout::menu_bar_layout;
use codelet_fspec_tui::components::menu_bar::{
    build_chips, paint_menu_bar, ChipInput, MenuAction, MenuCategory, MenuChip, MenuFocus,
    MenuLayout, MenuSnapshot, ZoneBCell, ZoneCButton, CATEGORIES,
};
use codelet_fspec_tui::{
    AgentView, App, FspecBackend, Theme, ViewMode, CREATE_SESSION_DIALOG_ID,
};
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
// Component-level helpers (scenarios 1, 2, 3, 4, 5, 6, 7, 11, 12, 13)
// ─────────────────────────────────────────────────────────────────────────

/// One `#n` chip in `status` with the given WU id suffix (a 1-input
/// `build_chips` — the real builder, so the glyph is live).
fn chip(status: SessionStatus, wu: Option<&'static str>) -> MenuChip {
    build_chips(
        &[ChipInput {
            index: (1, 3),
            status,
            wu_id: wu.map(str::to_string),
            active: false,
        }],
        0,
    )
    .pop()
    .expect("a single-input build_chips must produce one chip")
}

/// The board-shaped snapshot: the full `CATEGORIES` Zone A + one
/// `Chip(i)` Zone B cell per chip + the board's single Zone C button
/// (the surface's `&'static` slice, the `AGENT_ZONE_A` precedent).
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

/// The agent-shaped snapshot: the single 'Board View' Zone A item plus
/// the chips plus the agent's two Zone C buttons (R1/R3: both bracketed,
/// the `[esc]` hint REMOVED by MENU-011). The `&'static` slices mirror
/// the `AGENT_ZONE_A` / `AGENT_ZONE_C` constants the production snapshot
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

/// Scenario: The board bar paints its Zone A items bracketed
#[test]
fn scenario_the_board_bar_paints_its_zone_a_items_bracketed() {
    // @step Given a board menu bar snapshot with 1 open session and no ring focus
    let snap = board_snapshot(vec![chip(SessionStatus::Idle, None)], None);

    // @step When the bar is painted into a 120-column row
    let (buf, _layout) = render_bar(&snap, 120);
    let row = bar_line(&buf, 120);

    // @step Then Zone A reads "[ Kanban ] [ Tools ] [ Settings ] [ Help ]"
    assert!(
        row.starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"),
        "the bracketed Zone A items after the 1-cell pad: {row:?}"
    );
    // @step And the Zone B chip paints as "[ #1 ● ]"
    assert!(
        row.contains("[ #1 ● ]"),
        "the bracketed session chip: {row:?}"
    );
}

/// Scenario: The agent bar paints its "Board View" item bracketed
#[test]
fn scenario_the_agent_bar_paints_its_board_view_item_bracketed() {
    // @step Given an agent menu bar snapshot with 0 open sessions and no ring focus
    let snap = agent_snapshot(Vec::new(), None);

    // @step When the agent bar is painted into a 100-column row
    let (buf, _layout) = render_bar(&snap, 100);
    let row = bar_line(&buf, 100);

    // @step Then the left-anchored item reads "[ Board View ]"
    assert!(
        row.starts_with(" [ Board View ]"),
        "the agent bar's bracketed Zone A item: {row:?}"
    );
}

/// Scenario: A session chip paints bracketed with its work-unit id when width allows
#[test]
fn scenario_a_session_chip_paints_bracketed_with_its_work_unit_id_when_width_allows() {
    // @step Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
    let snap = board_snapshot(vec![chip(SessionStatus::Idle, Some("MENU-001"))], None);

    // @step And a 200-column render area
    // @step When the bar is rendered
    let (buf, layout) = render_bar(&snap, 200);
    let row = bar_line(&buf, 200);

    // @step Then the chip reads "[ #1 MENU-001 ● ]"
    assert_eq!(layout.level, 0, "wide area keeps the WU suffix");
    assert!(
        row.contains("[ #1 MENU-001 ● ]"),
        "the bracketed chip with its WU id suffix: {row:?}"
    );
}

/// Scenario: The truncation ladder runs on bracketed cells
#[test]
fn scenario_the_truncation_ladder_runs_on_bracketed_cells() {
    // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    let chips: Vec<MenuChip> = (0..4)
        .map(|i| {
            build_chips(
                &[ChipInput {
                    index: (i + 1, 4),
                    status: SessionStatus::Idle,
                    wu_id: Some(format!("MENU-00{}-LONGID", i + 1)),
                    active: false,
                }],
                0,
            )
            .pop()
            .expect("one chip per input")
        })
        .collect();
    let snap = board_snapshot(chips, None);

    // @step When the bar is rendered into a 84-column area
    // (84 cols = 82 inner: bracketed Zone A = 42; the level-1 row
    // (42 + 3 + 4x8 + 6 = 83) no longer fits, and the level-2 row
    // (42 + 3 + 3x8 + 6 + 6 = 81) fits — so the ladder lands at level
    // 2: WU ids dropped + the 4th chip folded, both applied to the
    // BRACKETED cells (R2/R6).)
    let (buf, layout) = render_bar(&snap, 84);
    let row = bar_line(&buf, 84);

    // @step Then no chip shows a work-unit id suffix and each visible chip reads "[ #n ● ]"
    assert!(
        layout.level >= 1,
        "WU ids dropped first (got level {})",
        layout.level
    );
    assert!(!row.contains("LONGID"), "no WU id suffixes: {row:?}");
    assert!(row.contains("[ #1 ● ]"), "bracketed chip 1: {row:?}");
    // @step And the 4th chip is folded into a single dim "[ +1 ]" marker
    assert!(
        layout.level >= 2,
        "level {} must fold beyond the 3rd chip",
        layout.level
    );
    assert!(
        row.contains("[ +1 ]"),
        "the bracketed +N fold marker: {row:?}"
    );
    // @step And the painted row fits within the area width
    for rect in layout
        .item_rects
        .iter()
        .chain(layout.cell_rects.iter())
        .chain(layout.zone_c_rects.iter())
    {
        assert!(
            rect.x + rect.width <= 84,
            "painted cell overflows: x {} w {}",
            rect.x,
            rect.width
        );
    }
}

/// Scenario: Mux pane view labels paint bracketed
#[test]
fn scenario_mux_pane_view_labels_paint_bracketed() {
    // @step Given a mux menu bar snapshot with [Board, Files] panes (Board focused) and 1 open session
    let snap = mux_snapshot(vec![chip(SessionStatus::Idle, None)]);

    // @step When the mux bar is painted into a 120-column row
    let (buf, layout) = render_bar(&snap, 120);
    let row = bar_line(&buf, 120);

    // @step Then Zone B reads "[ Board ]  [ Files ]  [ #1 ● ]" with the focused pane label bold
    assert!(
        row.contains("[ Board ]"),
        "the bracketed active pane label: {row:?}"
    );
    assert!(
        row.contains("[ Files ]"),
        "the bracketed inactive pane label: {row:?}"
    );
    assert!(
        row.contains("[ #1 ● ]"),
        "the bracketed session chip after the pane labels: {row:?}"
    );
    // The focused (active) pane label stays bold + theme.fg (white) —
    // BUG-202: never a standalone cyan foreground (the inverse-video
    // highlight is the only cyan in the bar row).
    let board_x = find_x(&buf, 0, "[ Board ]").expect("the [ Board ] label");
    let label_x = board_x + 2; // past '[ '
    assert!(
        buf[(label_x, 0)]
            .modifier
            .contains(ratatui::style::Modifier::BOLD),
        "the focused pane label must stay bold"
    );
    assert_eq!(
        buf[(label_x, 0)].fg,
        Color::White,
        "the focused pane label must paint the primary foreground (BUG-202), not cyan"
    );
    // @step And the row contains no Zone C button
    assert!(
        layout.zone_c_rects.is_empty(),
        "no Zone C on the mux bar: {row:?}"
    );
}

/// Scenario: The board bar paints a right-aligned "[ New Agent ]" button
#[test]
fn scenario_the_board_bar_paints_a_right_aligned_new_agent_button_bracketed() {
    // @step Given a board menu bar snapshot with 1 open session and no ring focus
    let snap = board_snapshot(vec![chip(SessionStatus::Idle, None)], None);

    // @step When the bar is painted into a 120-column row
    let (buf, layout) = render_bar(&snap, 120);
    let row = bar_line(&buf, 120);

    // @step Then the row ends with the right-aligned "[ New Agent ]" Zone C button
    assert!(
        row.ends_with("[ New Agent ]"),
        "the row must end with the right-aligned bracketed New Agent button: {row:?}"
    );
    assert_eq!(
        layout.zone_c_rects.len(),
        1,
        "exactly one Zone C rect (the board's single button)"
    );
    // The button is right-aligned at the inner edge (1-cell R1 padding).
    let last = &layout.zone_c_rects[0];
    assert_eq!(
        last.x + last.width,
        119,
        "the bracketed button must sit at the inner right edge"
    );
    // @step And the row contains no "Close Agent" button
    assert!(
        !row.contains("Close Agent"),
        "the board bar must not paint Close Agent: {row:?}"
    );
}

/// Scenario: The agent bar paints "[ New Agent ]" then "[ Close Agent ]" right-aligned
/// without the [esc] hint
#[test]
fn scenario_the_agent_bar_paints_new_agent_then_close_agent_bracketed_without_the_esc_hint() {
    // @step Given an agent menu bar snapshot with 0 open sessions and no ring focus
    let snap = agent_snapshot(Vec::new(), None);

    // @step When the agent bar is painted into a 100-column row
    let (buf, layout) = render_bar(&snap, 100);
    let row = bar_line(&buf, 100);

    // @step Then the row ends with "[ New Agent ]" followed by "[ Close Agent ]" right-aligned
    assert!(
        row.ends_with("[ New Agent ]  [ Close Agent ]"),
        "the row must end with both bracketed Zone C buttons (2-cell gap): {row:?}"
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
        "[ Close Agent ] ends at the inner right edge"
    );
    // @step And the row contains no "[esc]" hint
    assert!(
        !row.contains("[esc]"),
        "the [esc] hint must be gone (MENU-011 R3): {row:?}"
    );
    assert!(
        row.contains("[ Close Agent ]"),
        "the button itself still paints: {row:?}"
    );
}

/// Scenario: Enter on the agent's "[ Close Agent ]" still shows the same
/// exit confirmation as Esc (the BUG-204 semantics — always the dialog —
/// survive the hint removal; BUG-199 R2's state-dependent branches were
/// superseded and moved to the physical Esc key).
#[tokio::test]
async fn scenario_enter_on_the_agents_close_agent_button_still_shows_the_esc_exit_confirmation() {
    // @step Given the agent view shows 1 open idle session and the agent bar is painted
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the agent bar geometry (chips + Zone C rects)
                          // Ring: [ Board View ] → chip #1 → [ New Agent ] → [ Close Agent ].
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    for _ in 0..3 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        drain_pending(&mut app).await;
    }
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "Left then Right x3 must land on the '[ Close Agent ]' button"
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
        "'[ Close Agent ]' must still show the Esc exit confirmation dialog (BUG-199 semantics)"
    );
    // @step And the session is not destroyed until an option is committed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the button must never destroy the session directly"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus"
    );
}

/// Scenario: A left click on the board's "[ New Agent ]" button activates it
/// (R4: the hit rect covers the full bracketed span).
#[tokio::test]
async fn scenario_a_left_click_on_the_board_new_agent_button_activates_it() {
    // @step Given the board bar is painted with the "[ New Agent ]" Zone C button
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    let buf = render_app(&mut app); // cache the bar geometry
    let y = bar_row(&buf);
    // Click the LEADING bracket of the button — the bracketed span's
    // first cell must be inside the hit rect (R4).
    let x = find_x(&buf, y, "[ New Agent ]").expect("[ New Agent ] on the bar row");

    // @step When I left-click the button
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

/// Scenario: A focused bracketed element's inverse-video highlight covers the brackets
#[test]
fn scenario_a_focused_bracketed_elements_inverse_video_highlight_covers_the_brackets() {
    // @step Given a board menu bar snapshot with the ring focus on the "New Agent" button
    let snap = board_snapshot(
        vec![chip(SessionStatus::Idle, None)],
        Some(MenuFocus::ZoneC(0)),
    );

    // @step When the bar is painted into a 120-column row
    let (buf, layout) = render_bar(&snap, 120);
    let row = bar_line(&buf, 120);

    // @step Then every cell of the "[ New Agent ]" button (brackets included) is styled bg Cyan, fg Black, bold
    let rect = &layout.zone_c_rects[0];
    assert_eq!(
        rect.width, 13,
        "the bracketed '[ New Agent ]' span is 13 cells wide (R4: the brackets ARE the button)"
    );
    for cx in rect.x..rect.x + rect.width {
        let cell = &buf[(cx, 0)];
        assert_eq!(
            cell.bg,
            Color::Cyan,
            "cell {cx} must be inverse bg (bracket included)"
        );
        assert_eq!(cell.fg, Color::Black, "cell {cx} must be inverse fg");
        assert!(
            cell.modifier.contains(ratatui::style::Modifier::BOLD),
            "cell {cx} must be bold"
        );
    }
    // The bracket cells themselves carry the text.
    assert_eq!(buf[(rect.x, 0)].symbol(), "[");
    assert_eq!(buf[(rect.x + rect.width - 1, 0)].symbol(), "]");
    let _ = row;

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

// Scenario: The bracketed row never exceeds the area width (proptest)
proptest::proptest! {
    #[test]
    fn scenario_the_bracketed_row_never_exceeds_the_area_width(
        width in 3u16..=400,
        chip_count in 0usize..=24,
    ) {
        // @step Given a board menu bar snapshot with a Zone C "[ New Agent ]" button and any number of chips
        let chips: Vec<MenuChip> = (0..chip_count)
            .map(|i| {
                build_chips(
                    &[ChipInput {
                        index: (i + 1, 24),
                        status: SessionStatus::Idle,
                        wu_id: Some("A-BIG-UNIT-ID".to_string()),
                        active: false,
                    }],
                    0,
                )
                .pop()
                .expect("one chip per input")
            })
            .collect();
        let snap = board_snapshot(chips, None);

        // @step When the bar is painted into any render area width
        // (the geometry pass — the painted row is the layout's rects)
        if let Some(layout) = menu_bar_layout(Rect::new(0, 0, width, 1), &snap) {
            // @step Then the painted row (Zone A items, Zone B cells AND Zone C rects) never exceeds the area width
            // Too-tight areas legitimately paint nothing — only
            // assert the bound when a layout was produced.
            for rect in layout
                .item_rects
                .iter()
                .chain(layout.cell_rects.iter())
                .chain(layout.zone_c_rects.iter())
            {
                assert!(
                    rect.x + rect.width <= width,
                    "a bracketed cell (or A/B/Zone C rect) overflows: x {} w {} at width {}",
                    rect.x,
                    rect.width,
                    width
                );
            }
        }
    }
}

/// Scenario: The dropdown rows and the "u" help dialog keep their plain
/// registry labels (the bracketing is a bar-row-only transform).
#[test]
fn scenario_the_dropdown_rows_and_the_help_dialog_keep_their_plain_registry_labels() {
    // @step Given the Kanban dropdown is open on a board bar whose items paint bracketed
    let snap = board_snapshot(vec![chip(SessionStatus::Idle, None)], None);
    let (bar_buf, _layout) = render_bar(&snap, 80);
    let bar_row = bar_line(&bar_buf, 80);
    assert!(
        bar_row.contains("[ Kanban ]"),
        "the bar item is bracketed: {bar_row:?}"
    );

    // @step When the dropdown panel and the "Menu bar" help dialog are inspected
    let item_x = _layout.item_rects[0].x; // the "[ Kanban ]" item's x
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), item_x, 0, 0).expect("panel");
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
    let dropdown_text = (panel.y..panel.y + panel.height)
        .map(|y| {
            (panel.x..panel.x + panel.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    // The 'u' help dialog's body rows are generated from the FULL
    // registry (MENU-005 R1) — the plain 'key label' strings.
    let registry_rows: Vec<String> = CATEGORIES
        .iter()
        .flat_map(|c| c.entries.iter().map(|e| format!("{} {}", e.key, e.label)))
        .collect();

    // @step Then the dropdown rows and dialog rows list the plain registry labels (". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments")
    for label in [
        ". New Agent",
        "/ Search",
        "D FOUNDATION.md",
        "A Attachments",
    ] {
        let plain_row = registry_rows
            .iter()
            .find(|r| r.ends_with(label))
            .expect("registry row for {label}");
        assert_eq!(
            plain_row, label,
            "the registry row for '{label}' stays plain (no brackets)"
        );
        assert!(
            !plain_row.contains('[') && !plain_row.contains(']'),
            "the registry row must carry no brackets: {plain_row:?}"
        );
    }
    // The dropdown panel never paints a bracketed label: its widest
    // Kanban row is the plain 'D FOUNDATION.md' entry.
    let kanban_labels: Vec<&str> = CATEGORIES[0].entries.iter().map(|e| e.label).collect();
    for label in kanban_labels {
        assert!(
            !dropdown_text.contains(&format!("[ {label} ]")),
            "the dropdown must paint the plain label, not a bracketed one: {label}"
        );
    }

    // @step And brackets appear only on the bar row itself, never in the dropdown panel or the help dialog body
    assert!(
        !dropdown_text.contains("] ") && !dropdown_text.contains(" ["),
        "no bracketed cell in the dropdown panel (key column is a single char): {dropdown_text:?}"
    );
    for row in &registry_rows {
        assert!(
            !row.contains('[') && !row.contains(']'),
            "the help dialog body row must stay plain: {row:?}"
        );
    }
}

/// Scenario: With no open sessions the agent bar still shows both
/// (bracketed) Zone C buttons — the view-level render path.
#[test]
fn scenario_with_no_open_sessions_the_agent_bar_still_shows_both_bracketed_zone_c_buttons() {
    // @step Given an agent menu bar snapshot with zero open sessions
    // (view-level pane — the App cannot enter the Agent view with an
    // empty open-session list; the zero-chip snapshot is the same shape)

    // @step When the agent bar is painted into a 80-column row
    let buf = render_agent_pane(80);
    let row = row_text(&buf, AGENT_BAR_ROW);

    // @step Then the row ends with "[ New Agent ]" followed by "[ Close Agent ]" right-aligned
    assert!(
        row.ends_with("[ New Agent ]  [ Close Agent ]"),
        "both bracketed Zone C buttons must paint even with an empty chip list: {row:?}"
    );
    // @step And the buttons are independent of the (empty) chip list
    assert!(
        row.contains("[ Board View ]"),
        "the left-anchored bracketed 'Board View' item: {row:?}"
    );
    assert!(!row.contains("[esc]"), "no [esc] hint: {row:?}");
    assert!(
        !row.contains('│') && !row.contains('#'),
        "no separator and no chips without sessions: {row:?}"
    );
}
