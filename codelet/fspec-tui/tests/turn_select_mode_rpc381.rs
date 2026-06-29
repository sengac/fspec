// Feature: spec/features/agentview-turn-select-mode.feature
//
//! RPC-381 — AgentView Tab turn-selection (SELECT) mode.
//!
//! Feature: spec/features/agentview-turn-select-mode.feature
//!
//! Drives the AgentView dispatch + App reducer + SessionHeader wiring
//! end-to-end for the SELECT-mode port described in the RPC-381 design
//! doc:
//!   - Tab toggles `AgentView.turn_select_mode` and emits
//!     `Action::ToggleTurnSelectMode`; entering the mode auto-selects
//!     the last turn.
//!   - In select mode ↑/↓ emit `Action::TurnNavUp` / `Action::TurnNavDown`
//!     and clamp at the first/last turn.
//!   - Enter does NOT submit input while selecting.
//!   - Esc exits select mode and consumes the key (no `AgentEscPressed`).
//!   - Selection stays pinned to the same turn (by seq) when new chunks
//!     stream in.
//!   - A `/`-popup consumes Tab, so the popup-Tab path wins over the
//!     turn-select toggle.
//!   - The SessionHeader paints the `[SELECT]` badge while the mode is
//!     active.
//!
//! These tests target the INTENDED public surface (Action variants,
//! `AgentView.turn_select_mode`, `ScrollbackList` SELECT-mode methods)
//! and are EXPECTED to fail (compile error / assertion) until RPC-381
//! lands — the correct red state for ACDD.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    Action, AgentView, App, FspecBackend, RenderedChunk, SearchHistoryView, ViewMode,
};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn tab() -> Event {
    key(KeyCode::Tab, KeyModifiers::NONE)
}

/// A standalone AgentView wired to a fresh Action channel.
fn fresh_view() -> (AgentView, UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    (AgentView::new(tx), rx)
}

fn drain_view(rx: &mut UnboundedReceiver<Action>) -> Vec<Action> {
    let mut out = Vec::new();
    while let Ok(a) = rx.try_recv() {
        out.push(a);
    }
    out
}

/// An App in ViewMode::Agent with one open session seeded with three
/// single-line turns.
fn agent_app_with_three_turns() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    seed_turns(&mut app, &sid("s-1"), 3);
    app
}

fn seed_turns(app: &mut App, id: &SessionId, count: usize) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .expect("SessionContext present");
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            seq: i as u64,
            lines: vec![Line::from(format!("turn-{i}"))],
            source: None,
        });
    }
}

/// Drain the App's action bus, dispatching each queued Action back into
/// the App so reducer side-effects actually run synchronously.
fn drain_app(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

fn selected_index(app: &App, id: &SessionId) -> Option<usize> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.scrollback.selected_index())
}

fn selected_seq(app: &App, id: &SessionId) -> Option<u64> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.scrollback.selected_seq())
}

/// Render the App and return the rows of glyphs.
fn render_rows(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing Tab enters turn-selection mode and selects the last
//           turn
// ─────────────────────────────────────────────────────────────────────

#[test]
fn tab_enters_turn_selection_mode_and_selects_last_turn() {
    // @step Given an AgentView with a conversation of three turns
    let mut app = agent_app_with_three_turns();
    let id = sid("s-1");

    // @step And no popup or mode view is active
    assert!(app.navigator().agent.slash_popup.is_none());
    assert!(app.navigator().agent.file_popup.is_none());
    assert!(app.navigator().agent.resume_view.is_none());
    assert!(app.navigator().agent.search_view.is_none());

    // @step When I press the Tab key
    let _ = app.handle_event(&tab());
    drain_app(&mut app);

    // @step Then turn-selection mode becomes active
    assert!(
        app.navigator().agent.turn_select_mode,
        "Tab must turn ON turn-selection mode"
    );

    // @step And the third turn is the selected turn
    assert_eq!(
        selected_index(&app, &id),
        Some(2),
        "entering select mode must auto-select the last (3rd) turn"
    );
    assert_eq!(selected_seq(&app, &id), Some(2));

    // @step And the session header shows the [SELECT] badge
    let rows = render_rows(&mut app, 120, 24);
    assert!(
        rows[0].contains("[SELECT]"),
        "header row must show [SELECT]; got: {:?}",
        rows[0]
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing Tab again exits turn-selection mode
// ─────────────────────────────────────────────────────────────────────

#[test]
fn pressing_tab_again_exits_turn_selection_mode() {
    // @step Given an AgentView in turn-selection mode
    let mut app = agent_app_with_three_turns();
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    assert!(app.navigator().agent.turn_select_mode);

    // @step When I press the Tab key
    let _ = app.handle_event(&tab());
    drain_app(&mut app);

    // @step Then turn-selection mode becomes inactive
    assert!(
        !app.navigator().agent.turn_select_mode,
        "second Tab must turn OFF turn-selection mode"
    );

    // @step And the session header does not show the [SELECT] badge
    let rows = render_rows(&mut app, 120, 24);
    assert!(
        !rows[0].contains("[SELECT]"),
        "header row must NOT show [SELECT] after exiting; got: {:?}",
        rows[0]
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Up arrow selects the previous turn
// ─────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_selects_the_previous_turn() {
    // @step Given an AgentView in turn-selection mode with the last of three turns selected
    let mut app = agent_app_with_three_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    assert_eq!(selected_index(&app, &id), Some(2));

    // @step When I press the Up arrow key
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_app(&mut app);

    // @step Then the second turn is the selected turn
    assert_eq!(
        selected_index(&app, &id),
        Some(1),
        "Up must move the selection to the previous (2nd) turn"
    );

    // @step And a gray down-arrow bar is rendered above the selected turn
    // @step And a gray up-arrow bar is rendered below the selected turn
    let rows = render_rows(&mut app, 120, 24);
    let sel_y = rows
        .iter()
        .position(|r| r.contains("turn-1"))
        .expect("selected turn row present");
    assert!(sel_y >= 1, "need a row above the selected turn");
    assert!(
        rows[sel_y - 1].contains('\u{25BC}'),
        "expected ▼ bar above selected turn; got: {:?}",
        rows[sel_y - 1]
    );
    assert!(
        rows[sel_y + 1].contains('\u{25B2}'),
        "expected ▲ bar below selected turn; got: {:?}",
        rows[sel_y + 1]
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Up arrow on the first turn clamps the selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_on_first_turn_clamps_the_selection() {
    // @step Given an AgentView in turn-selection mode with the first of three turns selected
    let mut app = agent_app_with_three_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    // Walk up to the first turn (2 -> 1 -> 0).
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_app(&mut app);
    assert_eq!(selected_index(&app, &id), Some(0));

    // @step When I press the Up arrow key
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_app(&mut app);

    // @step Then the first turn is still the selected turn
    assert_eq!(
        selected_index(&app, &id),
        Some(0),
        "Up on the first turn must clamp the selection at index 0"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Down arrow on the last turn clamps the selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn down_arrow_on_last_turn_clamps_the_selection() {
    // @step Given an AgentView in turn-selection mode with the last of three turns selected
    let mut app = agent_app_with_three_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    assert_eq!(selected_index(&app, &id), Some(2));

    // @step When I press the Down arrow key
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_app(&mut app);

    // @step Then the last turn is still the selected turn
    assert_eq!(
        selected_index(&app, &id),
        Some(2),
        "Down on the last turn must clamp the selection at the last index"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Enter does not submit input while in turn-selection mode
// ─────────────────────────────────────────────────────────────────────

#[test]
fn enter_does_not_submit_input_while_in_turn_selection_mode() {
    // @step Given an AgentView in turn-selection mode
    let (mut view, mut rx) = fresh_view();
    view.handle_event(&tab());
    let _ = drain_view(&mut rx);
    assert!(
        view.turn_select_mode,
        "Tab must enable turn-selection mode on the AgentView"
    );

    // @step And the input contains the text "hello"
    view.input.set_value("hello");

    // @step When I press the Enter key
    view.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then no input is submitted
    let actions = drain_view(&mut rx);
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::InputSubmitted(_))),
        "Enter must NOT submit input in select mode; got {actions:?}"
    );

    // @step And the input still contains the text "hello"
    assert_eq!(view.input.value(), "hello");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Esc exits turn-selection mode without triggering the exit
//           cascade
// ─────────────────────────────────────────────────────────────────────

#[test]
fn esc_exits_turn_selection_mode_without_triggering_exit_cascade() {
    // @step Given an AgentView in turn-selection mode
    let (mut view, mut rx) = fresh_view();
    view.handle_event(&tab());
    let _ = drain_view(&mut rx);
    assert!(view.turn_select_mode);

    // @step When I press the Esc key
    let result = view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));

    // @step Then turn-selection mode becomes inactive
    assert!(
        !view.turn_select_mode,
        "Esc must turn OFF turn-selection mode"
    );

    // @step And the session header does not show the [SELECT] badge
    // (turn_select_mode is the source of the badge; asserted off above.)

    // @step And the normal Esc exit cascade is not triggered
    assert!(
        result.is_consumed(),
        "Esc in select mode must be consumed locally"
    );
    let actions = drain_view(&mut rx);
    assert!(
        !actions.iter().any(|a| matches!(a, Action::AgentEscPressed)),
        "Esc in select mode must NOT emit AgentEscPressed; got {actions:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Selection stays pinned to the same turn when new content
//           streams in
// ─────────────────────────────────────────────────────────────────────

#[test]
fn selection_stays_pinned_when_new_content_streams_in() {
    // @step Given an AgentView in turn-selection mode with the second of three turns selected
    let mut app = agent_app_with_three_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE)); // 2 -> 1
    drain_app(&mut app);
    assert_eq!(selected_index(&app, &id), Some(1));
    assert_eq!(selected_seq(&app, &id), Some(1));

    // @step When the agent streams a new turn into the scrollback
    {
        let ctx = app
            .agent_view_store_mut()
            .session_context_mut_for(&id)
            .expect("ctx");
        ctx.scrollback.push(RenderedChunk {
            seq: 3,
            lines: vec![Line::from("turn-3")],
            source: None,
        });
    }

    // @step Then the second turn is still the selected turn
    assert_eq!(
        selected_seq(&app, &id),
        Some(1),
        "selection must stay pinned to the same seq when new content streams in"
    );
    assert_eq!(selected_index(&app, &id), Some(1));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Tab is consumed by an open slash-command popup
// ─────────────────────────────────────────────────────────────────────

#[test]
fn tab_is_consumed_by_an_open_slash_command_popup() {
    // @step Given an AgentView with the slash-command popup open
    let (mut view, mut rx) = fresh_view();
    for ch in "/c".chars() {
        view.handle_event(&key(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    let _ = drain_view(&mut rx);
    assert!(
        view.slash_popup.is_some(),
        "slash popup must be open before Tab"
    );

    // @step When I press the Tab key
    view.handle_event(&tab());

    // @step Then the slash-command popup handles the key
    // (Tab on the highlighted command fills the input — the popup's
    // existing Tab behaviour — and closes the popup.)
    assert!(
        view.slash_popup.is_none(),
        "the popup must handle Tab (fills + closes), not the turn-select toggle"
    );

    // @step And turn-selection mode does not become active
    assert!(
        !view.turn_select_mode,
        "Tab consumed by the popup must NOT toggle turn-selection mode"
    );
    let actions = drain_view(&mut rx);
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::ToggleTurnSelectMode)),
        "no ToggleTurnSelectMode must be emitted while a popup consumes Tab; got {actions:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Supplementary: the standalone AgentView emits the new actions so the
// dispatch surface is pinned independent of the App reducer.
// ─────────────────────────────────────────────────────────────────────

#[test]
fn tab_emits_toggle_action_and_arrows_emit_turn_nav_in_select_mode() {
    let (mut view, mut rx) = fresh_view();

    // Tab emits ToggleTurnSelectMode.
    view.handle_event(&tab());
    let actions = drain_view(&mut rx);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ToggleTurnSelectMode)),
        "Tab must emit Action::ToggleTurnSelectMode; got {actions:?}"
    );

    // In select mode, Up/Down emit TurnNavUp / TurnNavDown.
    view.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    let up_actions = drain_view(&mut rx);
    assert!(
        up_actions.iter().any(|a| matches!(a, Action::TurnNavUp)),
        "Up in select mode must emit Action::TurnNavUp; got {up_actions:?}"
    );

    view.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    let down_actions = drain_view(&mut rx);
    assert!(
        down_actions
            .iter()
            .any(|a| matches!(a, Action::TurnNavDown)),
        "Down in select mode must emit Action::TurnNavDown; got {down_actions:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Supplementary: confirm Esc in select mode does NOT fall through to the
// App-level mode-view path (sanity that a fresh non-select-mode Esc still
// emits AgentEscPressed, so the select-mode suppression is meaningful).
// ─────────────────────────────────────────────────────────────────────

#[test]
fn esc_outside_select_mode_still_emits_agent_esc_pressed() {
    let (mut view, mut rx) = fresh_view();
    // Not in select mode, no popup / mode-view open.
    assert!(!view.turn_select_mode);
    assert!(view.search_view.is_none());
    let _ = view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    let actions = drain_view(&mut rx);
    assert!(
        actions.iter().any(|a| matches!(a, Action::AgentEscPressed)),
        "Esc outside select mode must still emit AgentEscPressed; got {actions:?}"
    );
    // Touch SearchHistoryView so the import is exercised even if the
    // select-mode arm changes the default Esc routing.
    let _ = SearchHistoryView::new();
}
