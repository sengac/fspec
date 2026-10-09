//! BUG-197 — menu bar: click-away must de-select a focused item/chip on ALL
//! three surfaces (board, agent, mux), and a stale ring focus must never
//! leave a chip highlighted after view flips.
//!
//! Feature: spec/features/menu-bar-click-away-de-select-and-no-stale-chip-highlight.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! BUG-196 already made the open-dropdown click-away path de-select the
//! bar (`MenuDismissBar`). BUG-197 extends the de-select gesture to the
//! TWO paths it missed:
//!
//!   R1 — a left click OFF the bar row (on content/panes) must clear the
//!        bar's ring focus whether or not a dropdown is open (today the
//!        `close_outside` arm only fires when `open_menu().is_some()`).
//!   R2 — a left click ON the bar row that hits NO zone (empty bar
//!        space) must clear the ring focus (today the board returns
//!        `EventResult::ignored()` and the mux returns `Swallowed`, both
//!        leaving the focus intact).
//!
//! R3/R4 (stale chip highlight after view flips + the
//! "chip highlights ONLY when the ring is on it, or the surface's
//! snapshot builder marked the current-session chip (Agent view /
//! focused Agent pane in mux — never the board, MENU-010)" invariant)
//! are pinned by asserting the ring-focus holders (board
//! `BoardStore.menu_focus`, agent `AgentView.menu_focus`, mux
//! `MultiplexLayout.menu_focus`) are `None` after the flips — the
//! shared `focused_cell()` painter then only paints ring-on-chip OR the
//! per-chip `active` flag (MENU-010: set by the surface's snapshot
//! builder; the board's builder marks no chip).
//!
//! Harness: App + MockBackend (the `bug196_menu_dropdown_row_click.rs` /
//! `menu002_board_surface.rs` / `menu004_mux_surface.rs` pattern) —
//! `fresh_app` + `drain_pending` + full-App render into a TestBackend
//! (board at 120x24, mux at 240x24); the agent scenarios use the
//! `menu003_agent_surface.rs` `AgentView` + `AgentViewStore` harness.
//!
//! Geometry (120x24, single Board view):
//!   y4  the 2-zone menu bar (`bar_row(buf)`, 3 rows below the
//!       `Checkpoints:` status row) — Zone A "Kanban" x15..21, "Help"
//!       x37..41; Zone B chips start ~x45 (first chip cell x45..47).
//!   Kanban cards: the BLOCKED column's content starts at x103, y14
//!       (menu002 parity).
//!
//! Geometry (240x24, mux active, bar present):
//!   y0  the mux menu bar ("Kanban" x1-6, … "Help" x23-26); the Files
//!       pane is x120..237 (pane 1 of two 120-col panes).
//!
//! Geometry (agent view, menu003 harness): bar row = 1; "Board View"
//! x1..11, the `│` separator ~x13, the first chip x15..17 ("#1 ●").
//!
//! Observation points:
//! - ring focus: `app.board_store().menu_focus()` /
//!   `app.navigator().mux.menu_focus()` / `view.menu_focus()`;
//! - the click still lands: board `focused_column` +
//!   `selected_index_for`, mux `navigator().mux.focus()`,
//!   `compositor()` dialog ids;
//! - emitted actions: the agent harness's `action_tx` receiver.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::MenuFocus, App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind,
    ViewMode,
};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

const CREATE_SESSION_DIALOG_ID: &str = "create-session-dialog";

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers (bug196_menu_dropdown_row_click.rs / menu003 pattern)
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

/// Render the full App into a `w`×`h` buffer.
fn render_app(app: &mut App, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
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

/// The 2-zone menu bar's row (3 rows below the `Checkpoints:` row).
fn bar_row(buf: &Buffer) -> u16 {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .expect("the checkpoint status row must render");
    status_y + 3
}

/// Seed `n` open sessions (s-1..s-n, Idle by default).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await;
}

/// Seed a small board (AUTH-001 backlog, AUTH-002 testing, AUTH-003
/// blocked) so columns have selectable cards.
async fn seed_units(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog"),
        make_unit("AUTH-002", "testing"),
        make_unit("AUTH-003", "blocked"),
    ]));
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

/// Arrange (board): the ring lands on the FIRST Zone A menu item
/// (Kanban, dropdown closed): focus the last column and step Right once
/// (the ring's columns→items edge, MENU-002 R2).
fn focus_first_board_item(app: &mut App) {
    app.board_store_mut().set_focused_column("blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
}

/// Arrange (board): the ring lands on the LAST Zone A item (Help).
fn focus_last_board_item(app: &mut App) {
    focus_first_board_item(app);
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
}

/// Arrange (board): the ring lands on chip `j` (Zone B) — walk from the
/// last item (Help) off the ring's items→chips edge.
fn focus_chip(app: &mut App, j: usize) {
    focus_last_board_item(app);
    for _ in 0..=j {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
}

/// Enable mux with the given pane list (horizontal, equal splits, focus
/// pane 0) — mirrors `bug196_menu_dropdown_row_click.rs::enable_mux`
/// including the BUG-182 R7 initial loads for the rendered lazy panes.
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let n = panes.len();
    let config = MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50; n.saturating_sub(1)],
        panes: panes.clone(),
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
    if panes.contains(&MuxPaneKind::ChangedFiles) {
        app.navigator_mut().changed_files = codelet_fspec_tui::views::ChangedFilesView::new();
        app.dispatch(codelet_fspec_tui::Action::ChangedFilesLoaded(Vec::new()));
    }
    if panes.contains(&MuxPaneKind::Checkpoints) {
        app.navigator_mut().checkpoints = codelet_fspec_tui::views::CheckpointsView::new();
        app.dispatch(codelet_fspec_tui::Action::CheckpointsLoaded(Vec::new()));
    }
}

/// Arrange (mux): the mux bar is focused on the first item (Kanban) via
/// the board ring entry (Right off the last column).
fn focus_mux_bar_first_item(app: &mut App) {
    app.board_store_mut().set_focused_column("blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
}

// ─────────────────────────────────────────────────────────────────────────
// Agent-view harness (menu003_agent_surface.rs pattern)
// ─────────────────────────────────────────────────────────────────────────

type AgentStoreHandle = std::sync::Arc<std::sync::Mutex<codelet_fspec_tui::AgentViewStore>>;

fn agent_harness() -> (
    codelet_fspec_tui::AgentView,
    AgentStoreHandle,
    tokio::sync::mpsc::UnboundedReceiver<codelet_fspec_tui::Action>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut view = codelet_fspec_tui::AgentView::new(tx);
    let store = std::sync::Arc::new(std::sync::Mutex::new(
        codelet_fspec_tui::AgentViewStore::default(),
    ));
    view.store_handle = Some(store.clone());
    (view, store, rx)
}

fn agent_seed_sessions(store: &AgentStoreHandle, n: usize) {
    let mut guard = store.lock().unwrap();
    for i in 1..=n {
        guard.append_session(codelet_fspec_tui::store::SessionContext::new(
            SessionId::new(format!("s-{i}")),
        ));
    }
}

fn agent_render(view: &mut codelet_fspec_tui::AgentView, store: &AgentStoreHandle) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), &mut store.lock().unwrap()))
        .unwrap();
    term.backend().buffer().clone()
}

/// Arrange (agent): the ring lands on the first bar item ('Board View')
/// — bare Left on an EMPTY input enters the bar at Item(0) (MENU-003 R3).
fn agent_focus_first_item(view: &mut codelet_fspec_tui::AgentView) {
    view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Board: clicking away from a focused menu item de-selects it
#[tokio::test]
async fn scenario_board_clicking_away_from_a_focused_menu_item_de_selects_it() {
    // @step Given the board is focused on a kanban column and the Kanban bar item is focused with its dropdown closed
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    app.board_store_mut().set_focused_column("backlog");
    focus_first_board_item(&mut app);
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must be ring-focused (dropdown closed)"
    );
    assert_eq!(app.board_store().open_menu(), None);
    // The bar must have painted (the mouse arms hit-test against the
    // cached geometry from this frame).
    let _buf = render_app(&mut app, 120, 24);

    // @step When I left click a work-unit card in another column
    // BLOCKED is the last column — its content rect starts at x103,
    // y14 (menu002 parity: card click at (103, 14)).
    app.handle_event(&click(103, 14));
    drain_pending(&mut app).await;

    // @step Then the clicked card becomes selected in its column
    assert_eq!(
        app.board_store().focused_column(),
        "blocked",
        "the click must STILL land: the clicked column is focused"
    );
    assert_eq!(
        app.board_store().selected_index_for("blocked"),
        0,
        "the click must STILL land: the clicked card becomes selected"
    );

    // @step And the bar highlight clears (no menu item is focused)
    assert!(
        app.board_store().menu_focus().is_none(),
        "R1: a left click OFF the bar row must clear the ring focus even \
         with the dropdown CLOSED (today only the open-dropdown path \
         de-selects)"
    );
}

/// Scenario: Board: clicking away from a ring-focused chip de-selects it
#[tokio::test]
async fn scenario_board_clicking_away_from_a_ring_focused_chip_de_selects_it() {
    // @step Given the board has 2 open sessions and the ring selector is on chip #2 (chip highlight visible)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    focus_chip(&mut app, 1);
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the ring must be on chip #2 (a 1-based chip #2 = ring index 1)"
    );
    let buf = render_app(&mut app, 120, 24);
    let bar = bar_row(&buf);

    // @step When I left click a card in the BACKLOG column
    app.handle_event(&click(103, 14));
    drain_pending(&mut app).await;

    // @step Then the clicked card becomes selected in the BACKLOG column
    assert_eq!(
        app.board_store().focused_column(),
        "blocked",
        "the click must STILL land: the card is selected in its column"
    );
    assert_eq!(app.board_store().selected_index_for("blocked"), 0);

    // @step And the chip highlight disappears (no chip is ring-focused)
    assert!(
        app.board_store().menu_focus().is_none(),
        "R1: clicking a card must clear the ring focus even when the \
         ring sits on a CHIP (the stale ZoneB focus is what left chips \
         highlighted — the user's bug report)"
    );
    let buf = render_app(&mut app, 120, 24);
    assert_eq!(
        row_text(&buf, bar),
        row_text(&buf, bar_row(&buf)),
        "re-render sanity: the bar row is stable"
    );
}

/// Scenario: Board: key bindings are live again after clicking away from the bar
#[tokio::test]
async fn scenario_board_key_bindings_are_live_again_after_clicking_away_from_the_bar() {
    // @step Given the board has no open sessions and the Kanban bar item is focused
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    app.board_store_mut().set_focused_column("backlog");
    focus_first_board_item(&mut app);
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must be ring-focused with no sessions open"
    );
    let _buf = render_app(&mut app, 120, 24);

    // @step When I left click a work-unit card in a column and then press the '.' key
    // Click away (the bar must de-select so the '.' arm is no longer
    // swallowed by the ring), then press '.' (the new-agent binding).
    app.handle_event(&click(103, 14));
    drain_pending(&mut app).await;
    assert!(
        app.board_store().menu_focus().is_none(),
        "the click away must clear the ring focus before '.' is handled"
    );
    app.handle_event(&key(KeyCode::Char('.'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the create-session dialog opens (the '.' binding ran and was not swallowed by the bar)
    // No sessions are open, so the '.' arm's OpenAgentView(None) falls
    // through to the RPC-097 create-session dialog.
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the '.' binding must be live again after clicking away with the \
         dropdown CLOSED (it is swallowed while the bar ring is focused)"
    );
}

/// Scenario: Board: returning to the board after a chip activation shows no pre-highlighted chip
#[tokio::test]
async fn scenario_board_returning_to_the_board_after_a_chip_activation_shows_no_pre_highlighted_chip(
) {
    // @step Given the board has 2 open sessions and the ring selector is on chip #1
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    focus_chip(&mut app, 0);
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "the ring must be on chip #1 (a 1-based chip #1 = ring index 0)"
    );
    let buf = render_app(&mut app, 120, 24);
    // The first chip's cell: Zone B starts after the bracketed items +
    // separator (MENU-011: `[ #1 ● ]` x61 — the `menu002_board_surface`
    // CHIP1_X geometry).
    let chip1_x = 61u16;

    // @step When I click chip #1 to open its Agent view and then return to the board
    // Click the chip (chip activation flips to the Agent view on
    // session s-1), then BackToBoard (the return-to-board action the
    // exit flow / 'Board View' item emit — with a live session a bare
    // Esc opens the exit confirmation instead, RPC-098).
    app.handle_event(&click(chip1_x, bar_row(&buf)));
    drain_pending(&mut app).await;
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "clicking the chip must flip to the Agent view"
    );
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-1")),
        "the flipped view must host chip #1's session"
    );
    app.dispatch(codelet_fspec_tui::Action::BackToBoard);
    drain_pending(&mut app).await;

    // @step Then the board bar does not pre-highlight any chip with the ring selector (no chip is ring-focused)
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Esc must return to the board"
    );
    assert!(
        app.board_store().menu_focus().is_none(),
        "R3: the chip activation must clear the board's ring focus so \
         returning to the board shows NO stale chip highlight"
    );

    // @step And no chip's cells paint the inverse-video highlight (the board never carries the current-session highlight — MENU-010)
    // The painter's `focused_cell` ORs the ring focus with
    // `MenuChip.active`; the board's snapshot builder marks no chip
    // (MENU-010 R2), so with the ring cleared NO chip may highlight —
    // the invariant holds by construction once the stale ring state is
    // gone (no ring focus may land on chip #2 either).
    assert!(
        app.board_store().menu_focus().is_none(),
        "only the ring selector may paint a chip on the board (MENU-010)"
    );
}

/// Scenario: Agent view: clicking empty bar space clears the bar highlight
#[tokio::test]
async fn scenario_agent_view_clicking_empty_bar_space_clears_the_bar_highlight() {
    // @step Given the agent view shows a session and the ring selector is on a bar item or chip (highlight visible)
    let (mut view, store, _rx) = agent_harness();
    agent_seed_sessions(&store, 1);
    let buf = agent_render(&mut view, &store);
    let bar_y = 1u16; // menu003 geometry: the bar row is header + 1.
    let _ = row_text(&buf, bar_y);
    agent_focus_first_item(&mut view);
    assert_eq!(
        view.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the ring must be on the 'Board View' item (highlight visible)"
    );
    agent_render(&mut view, &store);

    // @step When I left click empty bar-row space (no item, no chip, no dropdown open)
    // x30 sits in the empty bar space past the first chip (the chip
    // cells are x15..17, then a 2-cell gap).
    view.handle_event(&click(30, bar_y));

    // @step Then the bar highlight clears and the input composer regains keyboard focus
    assert!(
        view.menu_focus().is_none(),
        "R2: a left click on the bar row that hits NO zone must clear the \
         ring focus — the composer regains the keys (today the agent \
         arm returns Ignored and the focus lingers)"
    );
}

/// Scenario: Agent view: flipping back to the board leaves no ring-focused chip
#[tokio::test]
async fn scenario_agent_view_flipping_back_to_the_board_leaves_no_ring_focused_chip() {
    // @step Given the agent view shows a session, the ring selector is on a chip, and its session is the current session
    let (mut view, store, _rx) = agent_harness();
    agent_seed_sessions(&store, 2);
    // The agent's ring math reads its chip count from the last focused
    // render — paint a frame first so the 2 seeded chips enter the ring
    // (Item(0) → Right lands on ZoneB(0) = chip #1 = the CURRENT
    // session, s-1, the first appended session is current).
    agent_render(&mut view, &store);
    agent_focus_first_item(&mut view);
    view.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(
        view.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "the ring must be on chip #1 (the current session's chip)"
    );
    agent_render(&mut view, &store);

    // @step When I activate the 'Board View' item (Enter or click) to flip to the board
    // Walk back to the item and press Enter (the in-view activation
    // clears the agent's own bar focus and emits BackToBoard).
    view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(view.menu_focus(), Some(MenuFocus::Item(0)));
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    assert!(
        view.menu_focus().is_none(),
        "activating 'Board View' must clear the AGENT bar's ring focus \
         (in-view)"
    );

    // @step Then the board bar does not show a pre-highlighted chip (no chip is ring-focused)
    // End-to-end flip through the App: walk the board ring onto chip
    // #1 (column 0 → +11 stops = 7 columns + 4 items + chip 0), flip to
    // the Agent view on it, then BackToBoard. The board store's ring
    // focus must NOT survive the round trip — a lingering ZoneB focus
    // would paint a stale chip highlight on the board bar.
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    app.board_store_mut().menu_move(11);
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "the board ring must be on chip #1 before the flip"
    );
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new("s-1"),
    )));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Agent);
    app.dispatch(codelet_fspec_tui::Action::BackToBoard);
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(
        app.board_store().menu_focus().is_none(),
        "R4: after the view flip the board's ring focus must be None — \
         a lingering chip focus would paint a stale highlight"
    );

    // @step And no chip's cells paint the inverse-video highlight (the board never carries the current-session highlight — MENU-010)
    // With the ring cleared, the board's snapshot builder marks no
    // chip (MENU-010 R2), so NO chip may paint (the painter's
    // `focused_cell` ORs ring-on-chip with `MenuChip.active`).
    let _buf = render_app(&mut app, 120, 24);
    assert!(
        app.board_store().menu_focus().is_none(),
        "no ring focus may pre-highlight any chip on return"
    );
}

/// Scenario: Mux: clicking a pane clears the bar highlight
#[tokio::test]
async fn scenario_mux_clicking_a_pane_clears_the_bar_highlight() {
    // @step Given the mux surface shows a board pane and a Files pane, and a bar item is focused with its dropdown closed
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
    );
    focus_mux_bar_first_item(&mut app);
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must be ring-focused (dropdown closed)"
    );
    assert_eq!(app.navigator().mux.open_menu(), None);
    // The bar must have painted (the mouse arms hit-test against the
    // cached geometry from this frame).
    let _buf = render_app(&mut app, 240, 24);
    // The Files pane is pane index 1 (pane 0 x0..118, pane 1 x120..237):
    // click well inside the Files pane (menu004 parity: (150, 10)).
    let files_click_x = 150u16;
    let files_click_y = 10u16;

    // @step When I left click the Files pane
    app.handle_event(&click(files_click_x, files_click_y));
    drain_pending(&mut app).await;

    // @step Then the Files pane gains focus and the bar highlight clears
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the click must STILL land: the Files pane (index 1) is focused"
    );
    assert!(
        app.navigator().mux.menu_focus().is_none(),
        "R1: a left click OFF the bar row (on a pane) must clear the mux \
         bar's ring focus even with the dropdown CLOSED (today only the \
         open-dropdown CloseOutsideThenLand path de-selects)"
    );
}

/// Scenario: Mux: clicking empty bar space clears the chip highlight
#[tokio::test]
async fn scenario_mux_clicking_empty_bar_space_clears_the_chip_highlight() {
    // @step Given the mux surface has a focused pane, and the ring selector is on a chip (chip highlight visible)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
    );
    // Paint a frame before walking: the mux bar's ring walk reads its
    // zone_b (view labels + the GLOBAL chips) from the last rendered
    // snapshot — the dispatch's MenuMove arm builds it fresh, but the
    // entry (Right off the last column) only happens on the board
    // surface, which needs the rendered panes to exist.
    let _buf = render_app(&mut app, 240, 24);
    // Ring entry: Right off the last column lands on the first mux bar
    // item (Kanban). Zone B in the mux bar = the non-agent view labels
    // (Board, Files) then the global chips, so from Item(0) walk right
    // through the 4 items + 2 view labels to land on the FIRST chip
    // (ZoneB(2)).
    //
    // The walk drains after EACH Right (and after the entry Right):
    // the mux bar's key classification reads the layout's ring focus to
    // pick the token (entry rule `MenuMoveToItem(0)` vs the walk
    // `MenuMove(1)`), so every Right must be dispatched before the next
    // is classified — production dispatches between events; batching the
    // keys here would re-fire the board-column entry rule on a stale
    // focus (two entry Rights collapse onto Item(0)).
    focus_mux_bar_first_item(&mut app);
    drain_pending(&mut app).await;
    for _ in 0..6 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        drain_pending(&mut app).await;
    }
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "the ring must sit on the first chip (the chip-highlight path)"
    );
    let _buf = render_app(&mut app, 240, 24);
    let focus_before = app.navigator().mux.focus();
    // Empty bar-row space: past the right edge of every item/cell rect
    // (the bar content ends well before x239).
    let empty_x = 235u16;

    // @step When I left click empty bar-row space (no item, no chip, no cell hit)
    app.handle_event(&click(empty_x, 0));
    drain_pending(&mut app).await;

    // @step Then the chip highlight clears
    assert!(
        app.navigator().mux.menu_focus().is_none(),
        "R2: a left click on the bar row that hits NO zone must clear the \
         mux bar's ring focus (today the Swallowed arm leaves the focus \
         — the stale highlight the user reported)"
    );

    // @step And the focused pane keeps its focus (the click does not land on a pane)
    assert_eq!(
        app.navigator().mux.focus(),
        focus_before,
        "a bar-row click that hits no cell must NOT change the pane focus"
    );
}
