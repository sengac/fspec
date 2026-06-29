// Feature: spec/features/agentview-turn-content-modal.feature
//
//! RPC-382 — AgentView turn content modal (Enter on selected turn).
//!
//! Feature: spec/features/agentview-turn-content-modal.feature
//!
//! Builds on RPC-381 (turn-select / SELECT mode). Drives the AgentView
//! dispatch + App reducer end-to-end for the modal port described in the
//! RPC-382 design doc:
//!   - Enter on the selected turn emits `Action::OpenTurnModal(seq)`;
//!     the App reducer sets `AgentView.turn_modal_seq = Some(seq)`.
//!   - The `TurnContentModal` overlay renders the selected turn's FULL
//!     `ChunkSource.text`, even when the scrollback view of that turn is
//!     collapsed (top scrolled off a too-small viewport).
//!   - Esc closes the modal but STAYS in select mode (the `[SELECT]`
//!     badge remains); a second Esc then exits select mode.
//!   - Tab while the modal is open closes the modal AND exits select
//!     mode (tear-down parity with the TS reference).
//!   - While the modal is open, ↑/↓ are gated and do NOT move the
//!     underlying turn selection.
//!
//! These tests target the INTENDED public surface
//! (`Action::OpenTurnModal` / `Action::CloseTurnModal`,
//! `AgentView.turn_modal_seq`) and are EXPECTED to fail to COMPILE until
//! RPC-382 lands — the correct red state for ACDD.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::KeyCode;

mod common;

#[path = "common/turn_content_modal_rpc382_helpers.rs"]
mod helpers;
use helpers::{
    agent_app_with_collapsed_last_turn, agent_app_with_text_turns, drain_app, joined, key,
    modal_seq, render_rows, selected_seq, sid, tab,
};

// ─────────────────────────────────────────────────────────────────────
// Scenario: Enter on the selected turn opens the turn content modal
// ─────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_the_selected_turn_opens_the_turn_content_modal() {
    // @step Given an AgentView in turn-selection mode with the second of three turns selected
    let mut app = agent_app_with_text_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Up)); // last (3rd) -> 2nd
    drain_app(&mut app);
    assert_eq!(
        selected_seq(&app, &id),
        Some(1),
        "second turn must be selected"
    );

    // @step When I press the Enter key
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(&mut app);

    // @step Then the turn content modal is open
    assert_eq!(
        modal_seq(&app),
        Some(1),
        "Enter must open the modal for the selected turn's seq"
    );

    // @step And the modal shows the full text of the second turn
    let rows = render_rows(&mut app, 80, 20);
    assert!(
        joined(&rows).contains("SECONDBODY"),
        "modal must render the selected turn's body; got: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: The modal shows the full content of a collapsed turn
// ─────────────────────────────────────────────────────────────────────

#[test]
fn the_modal_shows_the_full_content_of_a_collapsed_turn() {
    // @step Given an AgentView in turn-selection mode with a collapsed turn selected
    let mut app = agent_app_with_collapsed_last_turn();
    let id = sid("s-1");
    let _ = app.handle_event(&tab()); // auto-selects the tall last turn
    drain_app(&mut app);
    assert_eq!(selected_seq(&app, &id), Some(2));
    // The turn is collapsed in the scrollback: its top (TOPMARKER) is
    // scrolled off a too-small viewport before the modal opens.
    let plain = render_rows(&mut app, 80, 12);
    assert!(
        !joined(&plain).contains("TOPMARKER"),
        "precondition: the tall turn's top must be collapsed/off-screen; got: {plain:?}"
    );

    // @step When I press the Enter key
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(&mut app);

    // @step Then the turn content modal is open
    assert_eq!(modal_seq(&app), Some(2), "Enter must open the modal");

    // @step And the modal shows the complete content of the turn
    let rows = render_rows(&mut app, 80, 12);
    assert!(
        joined(&rows).contains("TOPMARKER"),
        "modal must show the full body including the collapsed top; got: {rows:?}"
    );

    // @step And the modal does not show the collapsed placeholder
    assert!(
        !joined(&rows).contains("select turn to"),
        "modal must not render the collapsed '/expand' placeholder hint; got: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Esc closes the modal but stays in turn-selection mode
// ─────────────────────────────────────────────────────────────────────

#[test]
fn esc_closes_the_modal_but_stays_in_turn_selection_mode() {
    // @step Given an AgentView in turn-selection mode with the turn content modal open
    let mut app = agent_app_with_text_turns();
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open before Esc");
    assert!(app.navigator().agent.turn_select_mode);

    // @step When I press the Esc key
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain_app(&mut app);

    // @step Then the turn content modal is closed
    assert_eq!(modal_seq(&app), None, "first Esc must close the modal");

    // @step And turn-selection mode is still active
    assert!(
        app.navigator().agent.turn_select_mode,
        "first Esc must NOT exit select mode"
    );

    // @step And the session header still shows the [SELECT] badge
    let rows = render_rows(&mut app, 120, 24);
    assert!(
        rows[0].contains("[SELECT]"),
        "header must still show [SELECT]; got: {:?}",
        rows[0]
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A second Esc after closing the modal exits turn-selection mode
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_second_esc_after_closing_the_modal_exits_turn_selection_mode() {
    // @step Given an AgentView in turn-selection mode with the turn content modal closed
    let mut app = agent_app_with_text_turns();
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    assert!(app.navigator().agent.turn_select_mode);
    assert_eq!(modal_seq(&app), None, "modal must be closed in this state");

    // @step When I press the Esc key
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain_app(&mut app);

    // @step Then turn-selection mode becomes inactive
    assert!(
        !app.navigator().agent.turn_select_mode,
        "Esc with the modal closed must exit select mode"
    );

    // @step And the session header does not show the [SELECT] badge
    let rows = render_rows(&mut app, 120, 24);
    assert!(
        !rows[0].contains("[SELECT]"),
        "header must NOT show [SELECT] after exiting; got: {:?}",
        rows[0]
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Tab while the modal is open closes the modal and exits
//           select mode
// ─────────────────────────────────────────────────────────────────────

#[test]
fn tab_while_the_modal_is_open_closes_the_modal_and_exits_select_mode() {
    // @step Given an AgentView in turn-selection mode with the turn content modal open
    let mut app = agent_app_with_text_turns();
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open before Tab");
    assert!(app.navigator().agent.turn_select_mode);

    // @step When I press the Tab key
    let _ = app.handle_event(&tab());
    drain_app(&mut app);

    // @step Then the turn content modal is closed
    assert_eq!(modal_seq(&app), None, "Tab tear-down must close the modal");

    // @step And turn-selection mode becomes inactive
    assert!(
        !app.navigator().agent.turn_select_mode,
        "Tab must also exit select mode"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Scrollback keys are gated while the modal is open
// ─────────────────────────────────────────────────────────────────────

#[test]
fn scrollback_keys_are_gated_while_the_modal_is_open() {
    // @step Given an AgentView in turn-selection mode with the second of three turns selected
    let mut app = agent_app_with_text_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Up)); // last -> 2nd
    drain_app(&mut app);
    assert_eq!(
        selected_seq(&app, &id),
        Some(1),
        "second turn must be selected"
    );

    // @step And the turn content modal is open
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(&mut app);
    assert_eq!(modal_seq(&app), Some(1), "modal must be open");

    // @step When I press the Up arrow key
    let _ = app.handle_event(&key(KeyCode::Up));
    drain_app(&mut app);

    // @step Then the selected turn is still the second turn
    assert_eq!(
        selected_seq(&app, &id),
        Some(1),
        "Up must be gated while the modal is open — selection must not move"
    );
}
