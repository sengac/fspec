//! MENU-003 + MENU-007 — Agent view surface: the 2-zone bar row under the
//! SessionHeader (Zone A is the single 'Board View' item, MENU-007).
//!
//! Feature: spec/features/agent-view-surface-2-zone-bar-row-under-session-header.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert).
//!
//! MENU-007 supersession: the agent bar's Zone A is a SINGLE 'Board View'
//! item (no Actions/Help, no dropdowns — the agent view is
//! dropdown-free). The agent-dropdown scenarios of MENU-003 were removed
//! from the feature file; the dropdown machinery is covered by the
//! board/mux feature files (MENU-001/002/004) instead.
//!
//! Harness: `AgentView` + `AgentViewStore` pair (the rpc405 pattern) —
//! sessions seeded through `AgentViewStore::append_session`, one frame
//! rendered into a 100x24 TestBackend, keys/mouse fed through
//! `AgentView::handle_event`. Observation points:
//! - `AgentView::menu_focus()` (the `Option<MenuFocus>` getter);
//! - the live bar: inverse-video (bg Cyan) cells on the row directly
//!   below the SessionHeader (`menu_row(buf)` = header row + 1);
//! - emitted actions (the view's `action_tx` sink):
//!   `Action::BackToBoard` (activating 'Board View'),
//!   `Action::MenuChipActivate` (chip activation),
//!   `Action::AgentEscPressed` (Esc with the bar unfocused);
//! - chip activation: `store.current_session_index()` /
//!   `store.current_session()`.
//!
//! Pane geometry (100x24, single agent pane, menu row ON):
//!   y0  SessionHeader
//!   y1  the 2-zone menu bar ("Board View" left edge x1, dim `│`
//!       separator, then the chips; positions located by scan)
//!   y2+ RoleBanner / scrollback / footer / input
//!
//! The flag-OFF pane (mux shape) keeps the pinned 5-row layout:
//! header y0, input row = last row (y23), footer directly above.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::store::SessionContext;
use codelet_fspec_tui::{Action, AgentView, AgentViewStore};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn click(col: u16, row: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), col, row)
}

fn wheel(kind: MouseEventKind, col: u16, row: u16) -> Event {
    mouse(kind, col, row)
}

/// The shared session store: the view's chip-activation test seam
/// (the `store_handle` field) + the tests' store assertions.
type StoreHandle = std::sync::Arc<std::sync::Mutex<codelet_fspec_tui::AgentViewStore>>;

/// Build an `AgentView` + shared `AgentViewStore` pair plus the action
/// receiver (the view's `action_tx` sink) so emitted actions are
/// observable. The view is bound to the shared store so a chip
/// activation can focus the session directly (the production seam is
/// the App's `MenuChipActivate` resolution instead).
fn harness() -> (
    AgentView,
    StoreHandle,
    UnboundedReceiver<codelet_fspec_tui::Action>,
) {
    let (tx, rx) = unbounded_channel();
    let mut view = AgentView::new(tx);
    let store = std::sync::Arc::new(std::sync::Mutex::new(AgentViewStore::default()));
    view.store_handle = Some(store.clone());
    (view, store, rx)
}

/// Seed `n` open sessions (s-1..s-n, Idle by default) into `store`.
fn seed_sessions(store: &StoreHandle, n: usize) {
    let mut guard = store.lock().unwrap();
    for i in 1..=n {
        guard.append_session(SessionContext::new(SessionId::new(format!("s-{i}"))));
    }
}

fn sid(id: &str) -> SessionId {
    SessionId::new(id.to_string())
}

/// Render one frame on a 100x24 TestBackend (single-view pane, menu row
/// ON via `PaneSession::current_session()`).
fn render_agent(view: &mut AgentView, store: &StoreHandle) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), &mut store.lock().unwrap()))
        .unwrap();
    term.backend().buffer().clone()
}

/// Render one frame for a MUX-style pane: the flag-off shape (pinned
/// 5-row layout, no menu row).
fn render_mux_pane(view: &mut AgentView, store: &StoreHandle) -> Buffer {
    use codelet_fspec_tui::views::agent::pane_render::PaneSession;
    let pane = PaneSession {
        session: None,
        is_focused: true,
        menu_row: false,
    };
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| {
        view.render_session_pane(f.area(), f.buffer_mut(), &mut store.lock().unwrap(), pane)
    })
    .unwrap();
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The 2-zone menu bar's row: one row directly below the SessionHeader
/// (the header is the pane's top row — the single-view pane has no
/// rows above it, so the bar row is a constant).
fn menu_row(_buf: &Buffer) -> u16 {
    1
}

/// The x of the first occurrence of `needle` on `y` (None when absent).
/// `String::find` returns a BYTE index — the row contains multi-byte
/// glyphs (`│`, `●`), so convert to the CHAR (cell) index the buffer
/// addresses by.
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

/// The 'Board View' item's x on the bar row (MENU-007: the agent bar's
/// single Zone A item).
fn board_view_x(buf: &Buffer) -> u16 {
    find_x(buf, menu_row(buf), "Board View")
        .expect("the 'Board View' item must paint on the bar row")
}

/// The x of the first occurrence of `needle` on the bar row (MENU-009
/// Zone C buttons: "New Agent" / "Close Agent").
fn zone_c_button_x(buf: &Buffer, needle: &str) -> u16 {
    find_x(buf, menu_row(buf), needle)
        .unwrap_or_else(|| panic!("the Zone C button '{needle}' must paint on the bar row"))
}

/// The x of chip `n` (1-based) on the bar row.
fn chip_x(buf: &Buffer, n: usize) -> u16 {
    find_x(buf, menu_row(buf), &format!("#{n}")).expect("chip #{n} must paint on the bar row")
}

/// True when the cell carries the inverse-video highlight (bg Cyan).
fn is_inverse(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].bg == Color::Cyan
}

/// Arrange: the bar is focused on the FIRST menu item — the 'Board View'
/// item (MENU-007): bare Left on an empty input.
fn focus_first_item(view: &mut AgentView) {
    view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
}

/// Arrange: focus the bar on chip `n` (1-based) by walking the ring from
/// Item(0) — MENU-007: ONE item, then the chips (`n` Right steps).
fn focus_chip(view: &mut AgentView, n: usize) {
    focus_first_item(view);
    // Item(0) → chip #1 → … → chip #n: `n` Right steps (MENU-007).
    for _ in 0..n {
        view.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The agent pane row under the header paints the 2-zone menu bar
#[test]
fn scenario_the_agent_pane_row_under_the_header_paints_the_2_zone_menu_bar() {
    // @step Given the agent pane has 2 open sessions (one running, one idle) and an empty input
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 2);
    store
        .lock()
        .unwrap()
        .set_session_status(sid("s-1"), SessionStatus::Running);

    // @step When the agent pane renders
    let buf = render_agent(&mut view, &store);
    let y = menu_row(&buf);

    // @step Then the row directly below the SessionHeader shows the 'Board View' item a separator and the two session chips and no other chrome row shifts
    let row = row_text(&buf, y);
    assert!(
        row.contains("Board View"),
        "bar row must show the 'Board View' item: {row:?}"
    );
    assert!(
        !row.contains("Actions") && !row.contains("Help"),
        "the board-only Actions/Help items must NOT paint in the agent view (MENU-007): {row:?}"
    );
    assert!(
        row.contains("│"),
        "bar row must show the zone separator: {row:?}"
    );
    assert!(row.contains("#1"), "bar row must show chip #1: {row:?}");
    assert!(row.contains("#2"), "bar row must show chip #2: {row:?}");
    // The SessionHeader itself (row 0) still renders above the bar, and
    // the input row still renders on the last row (RPC-029: no "Enter=send"
    // footer hint — the placeholder hint is the stable input anchor).
    assert!(
        !row_text(&buf, 0).contains("Board View"),
        "the SessionHeader (row 0) must not carry the menu bar: {:?}",
        row_text(&buf, 0)
    );
    let last = buf.area.height - 1;
    assert!(
        row_text(&buf, last).contains("Type a message"),
        "the input row must still render on the last row after the row shift: {:?}",
        row_text(&buf, last)
    );
}

/// Scenario: The agent pane row is gated by the menu row flag
#[test]
fn scenario_the_agent_pane_row_is_gated_by_the_menu_row_flag() {
    // @step Given a mux agent pane with the menu row flag off
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);

    // @step When the pane renders
    let buf = render_mux_pane(&mut view, &store);

    // @step Then the pane layout is the pinned 5-row shape (header role scrollback footer input) and no menu bar row appears
    let last = buf.area.height - 1;
    assert!(
        !row_text(&buf, 1).contains("Board View"),
        "no menu bar row may appear when the flag is off: {:?}",
        row_text(&buf, 1)
    );
    assert!(
        !buf_text(&buf).contains("Board View"),
        "the 2-zone bar must not paint anywhere with the flag off"
    );
    // Pinned shape: input row is the LAST row, footer directly above it
    // (RPC-029: the footer paints no hint text without a workspace — its
    // dark-grey row background is the stable observation).
    assert!(
        row_text(&buf, last).starts_with(">") || row_text(&buf, last).contains("Type a message"),
        "the input row must be the last row of the pane: {:?}",
        row_text(&buf, last)
    );
    assert_eq!(
        buf[(0, last - 1)].bg,
        ratatui::style::Color::Rgb(0x33, 0x33, 0x33),
        "the footer row must sit directly above the input row: {:?}",
        row_text(&buf, last - 1)
    );
}

/// Scenario: Bare Left on an empty input enters the bar at the first item
#[test]
fn scenario_bare_left_on_an_empty_input_enters_the_bar_at_the_first_item() {
    // @step Given the agent pane has an empty input
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);

    // @step When I press bare Left once
    view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));

    // @step Then the 'Board View' item paints inverse-video with no dropdown open
    assert!(
        view.menu_focus()
            .as_ref()
            .is_some_and(|f| matches!(f, MenuFocus::Item(0))),
        "bare Left on an empty input must enter the bar at Item(0), got {:?}",
        view.menu_focus()
    );
    assert!(
        view.open_menu().is_none(),
        "the agent bar must never open a dropdown (MENU-007)"
    );
    let buf = render_agent(&mut view, &store);
    assert!(
        is_inverse(&buf, board_view_x(&buf), menu_row(&buf)),
        "the 'Board View' item must paint inverse-video"
    );
}

/// Scenario: Bare Left with a non-empty draft stays the text cursor
#[test]
fn scenario_bare_left_with_a_non_empty_draft_stays_the_text_cursor() {
    // @step Given the input draft is the text "hello"
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);
    view.input.set_value("hello");

    // @step When I press bare Left once
    let result = view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));

    // @step Then the text cursor moves left and the menu bar does not gain focus
    assert!(
        view.menu_focus().is_none(),
        "bare Left on a non-empty draft must NOT enter the bar"
    );
    assert_eq!(view.input.value(), "hello", "the draft must be untouched");
    let _ = result; // the input consumes the cursor move
}

/// Scenario: Bare Right never enters the bar
#[test]
fn scenario_bare_right_never_enters_the_bar() {
    // @step Given the agent pane has an empty input
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);

    // @step When I press bare Right once
    view.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));

    // @step Then the menu bar does not gain focus and the text cursor position is unchanged
    assert!(
        view.menu_focus().is_none(),
        "bare Right must never enter the bar"
    );
    assert!(view.input.is_empty(), "the draft must stay empty");
}

/// Scenario: The ring walks items then chips then the Zone C buttons then wraps to the first item
#[test]
fn scenario_the_ring_walks_items_then_chips_then_wraps_to_the_first_item() {
    // @step Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 2);
    render_agent(&mut view, &store); // the bar's paint caches the chip count + Zone C rects
    focus_first_item(&mut view);

    // @step When I press Right five times
    for _ in 0..5 {
        view.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }

    // @step Then the focus lands on chip #1 then chip #2 then 'New Agent' then 'Close Agent' and finally back on 'Board View'
    // The walk is observed step-by-step: re-run the sequence and check
    // every landing (MENU-007: one item, so Right #1 already leaves the
    // item — MENU-009 R3: the two Zone C buttons are the ring's last
    // stops).
    let (mut v2, s2, _rx2) = harness();
    seed_sessions(&s2, 2);
    render_agent(&mut v2, &s2); // the bar's paint caches the chip count + Zone C rects
    focus_first_item(&mut v2);
    v2.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert!(
        v2.menu_focus().as_ref() == Some(&MenuFocus::ZoneB(0)),
        "1st Right must land on chip #1 (no second Zone A item, MENU-007)"
    );
    v2.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert!(
        v2.menu_focus().as_ref() == Some(&MenuFocus::ZoneB(1)),
        "2nd Right must land on chip #2"
    );
    v2.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert!(
        v2.menu_focus().as_ref() == Some(&MenuFocus::ZoneC(0)),
        "3rd Right must land on the 'New Agent' Zone C button (MENU-009)"
    );
    v2.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert!(
        v2.menu_focus().as_ref() == Some(&MenuFocus::ZoneC(1)),
        "4th Right must land on the 'Close Agent' Zone C button (MENU-009)"
    );
    v2.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert!(
        v2.menu_focus().as_ref() == Some(&MenuFocus::Item(0)),
        "5th Right must wrap back to 'Board View'"
    );
    // And the primary walk ended at 'Board View' too:
    assert!(
        view.menu_focus().as_ref() == Some(&MenuFocus::Item(0)),
        "the focus must wrap back to the 'Board View' item"
    );
}

/// Scenario: The ring wraps left from the first item to the last Zone C button
#[test]
fn scenario_the_ring_wraps_left_from_the_first_item_to_the_last_chip() {
    // @step Given the agent pane has 3 open sessions and the menu bar is focused on the 'Board View' item
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 3);
    render_agent(&mut view, &store); // the bar's paint caches the chip count + Zone C rects
    focus_first_item(&mut view);

    // @step When I press Left once
    view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));

    // @step Then the 'Close Agent' button paints inverse-video (MENU-009:
    // the left wrap lands on the LAST Zone C button, not the last chip)
    assert!(
        view.menu_focus().as_ref() == Some(&MenuFocus::ZoneC(1)),
        "Left from Item(0) must wrap to the last Zone C button ('Close Agent')"
    );
    let buf = render_agent(&mut view, &store);
    assert!(
        is_inverse(&buf, zone_c_button_x(&buf, "Close Agent"), menu_row(&buf)),
        "'Close Agent' must paint inverse-video"
    );
    // And Left once more lands on the 'New Agent' button:
    view.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    assert!(
        view.menu_focus().as_ref() == Some(&MenuFocus::ZoneC(0)),
        "Left from 'Close Agent' must land on the 'New Agent' button"
    );
}

/// Scenario: Enter on a chip jumps to that session in the agent view
#[test]
fn scenario_enter_on_a_chip_jumps_to_that_session_in_the_agent_view() {
    // @step Given the agent pane has 3 open sessions and the menu bar is focused on chip #2
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 3);
    render_agent(&mut view, &store); // the bar's paint caches the chip count
    focus_chip(&mut view, 2);

    // @step When I press Enter once
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then the agent pane shows session #2 and the bar highlight clears
    assert_eq!(
        store.lock().unwrap().current_session_index(),
        1,
        "Enter on chip #2 must focus open_sessions[1] (s-2)"
    );
    assert_eq!(store.lock().unwrap().current_session(), Some(&sid("s-2")));
    assert!(
        view.menu_focus().is_none(),
        "the bar highlight must clear after chip activation"
    );
}

/// Scenario: Enter on the 'Board View' item returns to the Board view
#[test]
fn scenario_enter_on_the_board_view_item_returns_to_the_board_view() {
    // @step Given the agent pane is focused on the 'Board View' menu item
    let (mut view, store, mut rx) = harness();
    seed_sessions(&store, 1);
    focus_first_item(&mut view);

    // @step When I press Enter once
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then the active view is the Board view and the bar highlight clears
    assert!(
        rx.try_recv()
            .is_ok_and(|a| matches!(a, Action::BackToBoard)),
        "Enter on the 'Board View' item must emit BackToBoard (the app flips to the Board view)"
    );
    assert!(
        view.menu_focus().is_none(),
        "the bar highlight must clear after activating 'Board View'"
    );
}

/// Scenario: A character key dismisses the bar and is typed into the input
#[test]
fn scenario_a_character_key_dismisses_the_bar_and_is_typed_into_the_input() {
    // @step Given the menu bar is focused (no dropdown) and the input is empty
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);
    focus_first_item(&mut view);

    // @step When I press the bare x key
    view.handle_event(&key(KeyCode::Char('x'), KeyModifiers::NONE));

    // @step Then the bar highlight clears and the input draft is the text "x"
    assert!(
        view.menu_focus().is_none(),
        "a character key must clear the bar focus"
    );
    assert_eq!(
        view.input.value(),
        "x",
        "the character must be typed into the input"
    );
}

/// Scenario: Up and Down from a closed bar drop focus back into the input
#[test]
fn scenario_up_and_down_from_a_closed_bar_drop_focus_back_into_the_input() {
    // @step Given the agent pane has a draft of "hi" and the menu bar is focused on chip #1
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);
    focus_chip(&mut view, 1); // Item(0) -> chip #1 (MENU-007: one item)
    view.input.set_value("hi");

    // @step When I press Up once
    view.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));

    // @step Then the bar highlight clears the text cursor is active in the input and the draft is unchanged
    assert!(
        view.menu_focus().is_none(),
        "Up on a closed bar must drop focus back into the input"
    );
    assert_eq!(view.input.value(), "hi", "the draft must be unchanged");
}

/// Scenario: Esc with the bar unfocused is unchanged
#[test]
fn scenario_esc_with_the_bar_unfocused_is_unchanged() {
    // @step Given the menu bar is not focused and the agent pane input is focused
    let (mut view, store, mut rx) = harness();
    seed_sessions(&store, 1);
    assert!(view.menu_focus().is_none());

    // @step When I press Esc once
    let result = view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));

    // @step Then the app-level cascade (back to board) runs exactly as before the menu bar existed
    assert!(result.is_consumed());
    assert!(
        rx.try_recv()
            .is_ok_and(|a| matches!(a, Action::AgentEscPressed)),
        "Esc with the bar unfocused must emit AgentEscPressed (the app-level cascade)"
    );
}

/// Scenario: Clicking 'Board View' returns to the Board view
#[test]
fn scenario_clicking_board_view_returns_to_the_board_view() {
    // @step Given the agent pane has 2 open sessions
    let (mut view, store, mut rx) = harness();
    seed_sessions(&store, 2);
    let buf = render_agent(&mut view, &store); // refresh the cached bar geometry
    let x = board_view_x(&buf);

    // @step When I click the 'Board View' item in the bar
    view.handle_event(&click(x, menu_row(&buf)));

    // @step Then the active view is the Board view and no dropdown opens
    assert!(
        rx.try_recv()
            .is_ok_and(|a| matches!(a, Action::BackToBoard)),
        "clicking 'Board View' must emit BackToBoard (the app flips to the Board view)"
    );
    assert!(
        view.open_menu().is_none(),
        "clicking 'Board View' must not open a dropdown (MENU-007)"
    );
}

/// Scenario: Clicking a chip activates that session
#[test]
fn scenario_clicking_a_chip_activates_that_session() {
    // @step Given the agent pane has 2 open sessions
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 2);
    let buf = render_agent(&mut view, &store);
    let (x, y) = (chip_x(&buf, 1), menu_row(&buf));

    // @step When I click chip #1 in the bar
    view.handle_event(&click(x, y));

    // @step Then the agent pane shows session #1
    assert_eq!(
        store.lock().unwrap().current_session_index(),
        0,
        "clicking chip #1 must focus session #1"
    );
    assert_eq!(store.lock().unwrap().current_session(), Some(&sid("s-1")));
}

/// Scenario: Wheel left and right over the bar walk the ring
#[test]
fn scenario_wheel_left_and_right_over_the_bar_walk_the_ring() {
    // @step Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 2);
    focus_first_item(&mut view);
    render_agent(&mut view, &store); // cache the bar geometry
    let buf = render_agent(&mut view, &store);
    let y = menu_row(&buf);

    // @step When I scroll the wheel right once over the bar row
    view.handle_event(&wheel(MouseEventKind::ScrollRight, 50, y));

    // @step Then chip #1 is focused
    assert!(
        view.menu_focus().as_ref()
            == Some(&codelet_fspec_tui::components::menu_bar::MenuFocus::ZoneB(
                0
            )),
        "wheel right over the bar must walk the ring to chip #1 (MENU-007: one item)"
    );

    // @step When I scroll the wheel right once over the bar row
    view.handle_event(&wheel(MouseEventKind::ScrollRight, 50, y));

    // @step Then chip #2 is focused
    assert!(
        view.menu_focus().as_ref()
            == Some(&codelet_fspec_tui::components::menu_bar::MenuFocus::ZoneB(
                1
            )),
        "a second wheel right must land on chip #2"
    );
}

/// Scenario: Wheel left and right over the bar with no dropdown and no bar focus are ignored
#[test]
fn scenario_wheel_left_and_right_over_the_bar_with_no_dropdown_and_no_bar_focus_are_ignored() {
    // @step Given the agent pane has an empty input and the menu bar is not focused
    let (mut view, store, _rx) = harness();
    seed_sessions(&store, 1);
    let buf = render_agent(&mut view, &store);
    assert!(view.menu_focus().is_none());

    // @step When I scroll the wheel right once over the bar row
    view.handle_event(&wheel(MouseEventKind::ScrollRight, 50, menu_row(&buf)));

    // @step Then the menu bar does not gain focus
    assert!(
        view.menu_focus().is_none(),
        "wheel over the bar must be ignored while the bar is unfocused"
    );
}

/// Scenario: The bar omits the separator and chips when no sessions are open
#[test]
fn scenario_the_bar_omits_the_separator_and_chips_when_no_sessions_are_open() {
    // @step Given the agent pane has no open sessions
    let (mut view, store, _rx) = harness();

    // @step When the agent pane renders
    let buf = render_agent(&mut view, &store);
    let row = row_text(&buf, menu_row(&buf));

    // @step Then the bar row shows only the 'Board View' item with no separator and no chips
    assert!(
        row.contains("Board View"),
        "the bar must still show 'Board View': {row:?}"
    );
    assert!(
        !row.contains("│"),
        "no separator may paint without chips: {row:?}"
    );
    assert!(
        !row.contains("#1"),
        "no chips may paint without sessions: {row:?}"
    );
}

/// Scenario: With no chips the ring wraps from the item to itself
/// (MENU-009: through the two Zone C buttons)
#[test]
fn scenario_with_no_chips_the_ring_wraps_from_the_item_to_itself() {
    // @step Given the agent pane has no open sessions and the menu bar is focused on the 'Board View' item
    let (mut view, _store, _rx) = harness();
    focus_first_item(&mut view);
    assert!(
        view.menu_focus()
            .as_ref()
            .is_some_and(|f| matches!(f, MenuFocus::Item(0))),
        "the 'Board View' item must be focused first"
    );

    // @step When I press Right three times
    // (MENU-009: with no chips the ring is 1 item + 2 Zone C buttons —
    // 'New Agent' → 'Close Agent' → wrap back to 'Board View'.)
    for _ in 0..3 {
        view.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }

    // @step Then the 'Board View' item is focused again (the walk passed through both Zone C buttons)
    assert!(
        view.menu_focus()
            .as_ref()
            .is_some_and(|f| matches!(f, MenuFocus::Item(0))),
        "with no chips the ring must wrap from the item through the \
         Zone C buttons back to itself (MENU-009)"
    );
}
