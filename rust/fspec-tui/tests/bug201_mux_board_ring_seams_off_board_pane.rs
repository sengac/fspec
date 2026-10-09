//! BUG-201 — Mux board-pane ring: two defects in the BUG-200 seam-mirror
//! code, both surfaced by the user's live [Board | Agent] layout with the
//! AGENT pane focused (the persisted `focused_pane: 1` home focus).
//!
//! Feature: spec/features/mux-board-pane-ring-seams-fire-off-the-board-pane-stale-board-bar-state-survives-mux-on.feature
//!
//! (1) The seam crossings in `menu_move_mux` fired even when the Board
//! pane was NOT focused — Right off the last Zone B cell teleported the
//! board's focused_column to col0 and dismissed the bar while keyboard
//! focus sat in the agent pane; Left off Item(0) mirrored onto col6.
//! Post-fix: with the Board pane unfocused the bar is a closed
//! items⇄Zone B loop (no column stops).
//!
//! (2) Entering mux never cleared the board store's OWN bar state
//! (`menu_focus` / `open_menu`) left over from the single Board view —
//! the board pane's bar is suppressed in the grid, but the next
//! Left/Right walked that INVISIBLE ring (the ring "lost its place").
//! Post-fix: every mux entry path clears the board store's bar state
//! (`dismiss_menu` — focus + dropdown; `focused_column` untouched).
//!
//! Harness: full-App + MockBackend, real key events through
//! `App::handle_event` — the BUG-200 pattern, but entering mux through
//! the PRODUCTION entry paths (`/mux on` via `InputSubmitted`,
//! `/mux default`, the MuxConfigDialog commit) instead of the
//! `enable_with_config` test seam (which is what let BUG-200's tests
//! miss both defects: a fresh board store + board-pane focus only).
//!
//! Ring under test (mux bar, [Board | Agent] + 1 session):
//!   Item0 (Kanban) → Item1 (Tools) → Item2 (Settings) → Item3 (Help)
//!   → ZoneB0 ('Board' view label) → ZoneB1 (#1 chip — the last cell).
//! The 7 board columns are ring stops ONLY while the Board pane is
//! focused (BUG-200 seams, kept by the board-pane-focused regression
//! scenario below).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::MenuFocus, App, FspecBackend, MuxOrientation, MuxPaneKind, ViewMode,
};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn plain(code: KeyCode) -> Event {
    key(code, KeyModifiers::NONE)
}

fn shift_left() -> Event {
    key(KeyCode::Left, KeyModifiers::SHIFT)
}

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

fn submit(app: &mut App, text: &str) {
    app.dispatch(codelet_fspec_tui::Action::InputSubmitted(text.to_string()));
}

/// Seed one open session (s-1) and drain the created follow-up actions.
async fn seed_session(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
        "s-1",
    )));
    drain_pending(app).await;
}

/// Arrange: the single Board view with the board's OWN bar engaged at
/// Item(0) — the ring walked Right off the last column (blocked, so the
/// board store's focused_column is 6 and `menu_focus` is `Item(0)`).
async fn engage_board_bar(app: &mut App) {
    app.board_store_mut().set_focused_column("blocked");
    let _ = app.handle_event(&plain(KeyCode::Right));
    drain_pending(app).await;
}

/// Arrange: the [Board | Agent] grid entered through the MuxConfigDialog
/// commit (Enabled Off → On, Enter) — the production OFF→ON path. The
/// committed draft keeps the default preset's `focused_pane: 1` (the
/// agent pane).
fn enter_mux_via_dialog(app: &mut App) {
    submit(app, "/mux");
    let _ = app.handle_event(&plain(KeyCode::Right)); // Enabled: Off → On
    let _ = app.handle_event(&plain(KeyCode::Enter)); // commit (remove)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Right from the last chip with the agent pane focused wraps
// to the first menu item
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_right_from_the_last_chip_with_the_agent_pane_focused_wraps_to_the_first_menu_item(
) {
    // @step Given the mux grid is [Board | Agent] with 1 open session and the AGENT pane is focused (the bar was engaged at the first menu item via the empty-input Left entry rule)
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app).await;
    engage_board_bar(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    enter_mux_via_dialog(&mut app);
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux, "mux must be active");
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the agent pane is focused (the persisted home focus)"
    );
    // The empty-input Left entry rule engages the bar at Item(0).
    app.handle_event(&plain(KeyCode::Left));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the entry rule must engage the bar at the first menu item"
    );
    // The ring: Item(0) → Item1 → Item2 → Item3 → ZoneB0 → ZoneB1.
    for _ in 0..5 {
        app.handle_event(&plain(KeyCode::Right));
        drain_pending(&mut app).await;
    }
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the arrange must land on the last chip (#1)"
    );

    // @step When I walk the ring Right through the menu items and the view labels onto the last session chip
    // (done in the arrange — the walk above lands on ZoneB1)

    // @step Then the highlight wraps to the first menu item (Kanban) in the top bar, the bar stays engaged, and the board's focused column is unchanged (the board pane is not focused, so the ring is the closed items⇄chips loop and no column stop is entered)
    app.handle_event(&plain(KeyCode::Right));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "Right off the last chip with the AGENT pane focused must wrap to Item(0) — \
         the closed loop, NOT the board's first column"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "the seam must not touch the board's focused_column off the board pane (stays on blocked)"
    );

    // @step When I walk the ring Left from the first menu item
    app.handle_event(&plain(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step And the highlight wraps to the last session chip (the ring closes on both ends without any column stop)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "Left from Item(0) with the AGENT pane focused must wrap to the last chip — \
         NOT the board's last column"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "the left seam must not touch the board's focused_column off the board pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Left from the first menu item with the agent pane focused
// wraps to the last chip
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_left_from_the_first_menu_item_with_the_agent_pane_focused_wraps_to_the_last_chip()
{
    // @step Given the mux grid is [Board | Agent] with 1 open session and the AGENT pane is focused with an empty input, and the agent pane's empty-input Left entry rule has engaged the bar at the first menu item (Kanban)
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app).await;
    engage_board_bar(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    enter_mux_via_dialog(&mut app);
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().mux.focus(), 1, "the agent pane is focused");
    app.handle_event(&plain(KeyCode::Left));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the empty-input Left entry rule must engage the bar at Item(0)"
    );

    // @step When I press Left once
    app.handle_event(&plain(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the highlight wraps to the last session chip in the top bar (not onto the board's last column), the bar stays engaged, and the board's focused column is unchanged
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "Left from Item(0) with the AGENT pane focused must wrap to the last chip (#1)"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "the board's focused column must stay on blocked (the seam must not fire off the board pane)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Entering mux via /mux on from a Board view with an engaged
// bar clears the board bar state
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_entering_mux_via_mux_on_from_a_board_view_with_an_engaged_bar_clears_the_board_bar_state(
) {
    // @step Given the single Board view is showing with the board's own bar engaged at the first menu item (Kanban) — the ring was walked Right off the last column (blocked) so the board store's ring focus sits on Item(0) and its focused column is blocked
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app).await;
    engage_board_bar(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.board_store().menu_focus(), Some(MenuFocus::Item(0)));
    assert_eq!(app.board_store().focused_column_index(), 6);

    // @step When I submit the slash command "/mux on"
    submit(&mut app, "/mux on");
    drain_pending(&mut app).await;

    // @step Then mux mode is active and the board store's bar state is cleared — no ring focus and no open dropdown on the board store — while the board's focused column stays on blocked (NOT reset by the entry)
    assert_eq!(app.active_view(), ViewMode::Mux, "mux must be active");
    assert!(
        app.board_store().menu_focus().is_none(),
        "entering mux must clear the board store's ring focus (the suppressed bar must not survive)"
    );
    assert!(
        app.board_store().open_menu().is_none(),
        "entering mux must clear the board store's open dropdown"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "the entry must NOT reset the board's focused column (stays on blocked)"
    );

    // @step When I press Left once (the Board pane is the focused mux pane, the board's fresh entry focus)
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "/mux on focuses the board pane on fresh entry"
    );
    app.handle_event(&plain(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the board's focused column moves to the second-to-last column (done) — the Left walks the visible columns, never the board's invisible (suppressed) bar
    assert_eq!(
        app.board_store().focused_column_index(),
        5,
        "Left must walk the VISIBLE columns (blocked → done) — with the stale Item(0) \
         focus the pre-fix walk went off the board store's hidden ring instead"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Committing the mux config dialog from off to on clears the
// board bar state
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_committing_the_mux_config_dialog_from_off_to_on_clears_the_board_bar_state() {
    // @step Given the single Board view is showing with the board's own bar engaged at the first menu item (Kanban) — the board store's ring focus sits on Item(0) and its focused column is blocked
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app).await;
    engage_board_bar(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.board_store().menu_focus(), Some(MenuFocus::Item(0)));
    assert_eq!(app.board_store().focused_column_index(), 6);

    // @step When I open the mux config dialog ("/mux"), set the Enabled row to On and commit it (the dialog emits MuxConfigApplied with the enabled draft)
    submit(&mut app, "/mux");
    drain_pending(&mut app).await;
    app.handle_event(&plain(KeyCode::Right)); // Enabled: Off → On (cursor 0)
    app.handle_event(&plain(KeyCode::Enter)); // commit → MuxConfigApplied
    drain_pending(&mut app).await;

    // @step Then mux mode is active and the board store's bar state is cleared — no ring focus and no open dropdown on the board store — while the board's focused column stays on blocked (NOT reset by the entry)
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the dialog commit must enter mux"
    );
    assert!(
        app.board_store().menu_focus().is_none(),
        "the OFF→ON dialog commit must clear the board store's ring focus"
    );
    assert!(
        app.board_store().open_menu().is_none(),
        "the OFF→ON dialog commit must clear the board store's open dropdown"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "the entry must NOT reset the board's focused column (stays on blocked)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Entering mux via /mux default clears the board bar state
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_entering_mux_via_mux_default_clears_the_board_bar_state() {
    // @step Given the single Board view is showing with the board's own bar engaged at the first menu item (Kanban) — the board store's ring focus sits on Item(0) and its focused column is blocked
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app).await;
    engage_board_bar(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.board_store().menu_focus(), Some(MenuFocus::Item(0)));
    assert_eq!(app.board_store().focused_column_index(), 6);

    // @step When I submit the slash command "/mux default"
    submit(&mut app, "/mux default");
    drain_pending(&mut app).await;

    // @step Then mux mode is active with the default preset and the board store's bar state is cleared — no ring focus and no open dropdown on the board store — while the board's focused column stays on blocked (NOT reset by the entry)
    assert_eq!(app.active_view(), ViewMode::Mux, "mux must be active");
    let cfg = app.navigator().mux.config();
    assert!(cfg.enabled, "the default preset must enable mux");
    assert_eq!(
        cfg.panes,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent],
        "the default preset is [Board | Agent]"
    );
    assert_eq!(cfg.orientation, MuxOrientation::Horizontal);
    assert!(
        app.board_store().menu_focus().is_none(),
        "/mux default must clear the board store's ring focus"
    );
    assert!(
        app.board_store().open_menu().is_none(),
        "/mux default must clear the board store's open dropdown"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "the entry must NOT reset the board's focused column (stays on blocked)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: With the board pane focused the BUG-200 seam crossings stay
// in effect (regression guard — the gate must not kill the board-pane
// ring)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_with_the_board_pane_focused_the_bug_200_seam_crossings_stay_in_effect() {
    // @step Given the mux grid is [Board | Agent] with 1 open session and the Board pane is focused with the ring on the last session chip
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app).await;
    engage_board_bar(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    enter_mux_via_dialog(&mut app);
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the dialog commit focuses the agent pane (home focus)"
    );
    // Shift+Left: move the pane focus onto the Board pane (App-level
    // intercept — the bar never claims Shift+arrows).
    app.handle_event(&shift_left());
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the board pane is focused now"
    );
    // Enter the bar off the board's last column (the pre-existing edge
    // rule), then walk to the last chip: Item(0) → Items → ZoneB0 → ZoneB1.
    app.handle_event(&plain(KeyCode::Right));
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().mux.menu_focus(), Some(MenuFocus::Item(0)));
    for _ in 0..5 {
        app.handle_event(&plain(KeyCode::Right));
        drain_pending(&mut app).await;
    }
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the arrange must land on the last chip (#1)"
    );

    // @step When I press Right once
    app.handle_event(&plain(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the highlight clears from the top bar and the board's first column (backlog) is focused (the seam crossing stays in effect on the board pane)
    assert_eq!(
        app.board_store().focused_column_index(),
        0,
        "Right from the last chip with the BOARD pane focused must land on col0 (backlog)"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar ring must clear on the column landing"
    );
}
