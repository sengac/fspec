//! RPC-096 — Shift+Left/Right end-of-list parity with TS `useSessionNavigation`.
//!
//! Feature: spec/features/agentview-shift-arrow-end-of-list-parity.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios. All tests are
//! expected to FAIL before implementation (red phase) — they reference
//! `NavTarget`, `navigate_next`, `navigate_prev`, and
//! `request_create_session_dialog_no_auto` which do not yet exist.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::store::NavTarget;
use codelet_fspec_tui::store::SessionContext;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, AgentViewStore, App, FspecBackend};
use codelet_rpc_types::SessionId;

mod common;
use common::MockBackend;

fn fresh_app() -> Arc<MockBackend> {
    Arc::new(MockBackend::new())
}

fn app() -> App {
    let backend: Arc<dyn FspecBackend> = fresh_app();
    App::new(backend)
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Scenario: navigate_next returns CreateDialog and navigate_prev returns Board when the store is empty
#[test]
fn empty_store_navigate_next_is_create_dialog_and_navigate_prev_is_board() {
    // @step Given an AgentViewStore with zero open sessions
    let store = AgentViewStore::default();
    // @step When I call navigate_next on the store
    let next = store.navigate_next();
    // @step Then the result is NavTarget::CreateDialog
    assert!(matches!(next, NavTarget::CreateDialog));
    // @step When I call navigate_prev on the store
    let prev = store.navigate_prev();
    // @step Then the result is NavTarget::Board
    assert!(matches!(prev, NavTarget::Board));
}

/// Scenario: navigate_next returns CreateDialog at the last session index
#[test]
fn navigate_next_at_last_index_returns_create_dialog() {
    // @step Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 2
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    store.append_session(SessionContext::new(sid("s-3")));
    assert_eq!(store.current_session_index(), 2);
    // @step When I call navigate_next on the store
    let next = store.navigate_next();
    // @step Then the result is NavTarget::CreateDialog
    assert!(matches!(next, NavTarget::CreateDialog));
}

/// Scenario: navigate_prev returns Session(idx-1) when not at the first session index
#[test]
fn navigate_prev_returns_previous_session_when_not_at_first() {
    // @step Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 2
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    store.append_session(SessionContext::new(sid("s-3")));
    // @step When I call navigate_prev on the store
    let prev = store.navigate_prev();
    // @step Then the result is NavTarget::Session(1)
    assert_eq!(prev, NavTarget::Session(1));
}

/// Scenario: navigate_next returns Session(idx+1) when not at the last session index
#[test]
fn navigate_next_returns_next_session_when_not_at_last() {
    // @step Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 0
    let mut app_inst = app();
    app_inst.dispatch(Action::SessionCreated(sid("s-1")));
    app_inst.dispatch(Action::SessionCreated(sid("s-2")));
    app_inst.dispatch(Action::SessionCreated(sid("s-3")));
    // Walk back to index 0 via prev cycles (mid-list dispatch is the
    // only public way to rewind the focus on AgentViewStore).
    app_inst.dispatch(Action::SessionPrev); // 2 -> 1
    app_inst.dispatch(Action::SessionPrev); // 1 -> 0
    assert_eq!(app_inst.agent_view_store().current_session_index(), 0);
    // @step When I call navigate_next on the store
    let next = app_inst.agent_view_store().navigate_next();
    // @step Then the result is NavTarget::Session(1)
    assert_eq!(next, NavTarget::Session(1));
}

/// Scenario: navigate_prev returns Board at the first session index
#[test]
fn navigate_prev_at_first_index_returns_board() {
    // @step Given an AgentViewStore with three open sessions s-1, s-2, s-3 and current_session_index 0
    let mut app_inst = app();
    app_inst.dispatch(Action::SessionCreated(sid("s-1")));
    app_inst.dispatch(Action::SessionCreated(sid("s-2")));
    app_inst.dispatch(Action::SessionCreated(sid("s-3")));
    app_inst.navigator_mut().active_view = ViewMode::Agent;
    app_inst.dispatch(Action::SessionPrev); // 2 -> 1
    app_inst.dispatch(Action::SessionPrev); // 1 -> 0
    assert_eq!(app_inst.agent_view_store().current_session_index(), 0);
    // @step When I call navigate_prev on the store
    let prev = app_inst.agent_view_store().navigate_prev();
    // @step Then the result is NavTarget::Board
    assert!(matches!(prev, NavTarget::Board));
}

/// Scenario: Shift+Right with a single attached session opens the Create Session dialog
#[test]
fn single_session_shift_right_opens_create_session_dialog() {
    // @step Given an App with one open session s-1 and current_session_index 0
    let mut app = app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And navigator.active_view is ViewMode::Agent
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step When the App dispatches Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then agent_view_store.show_create_session_dialog() returns true
    assert!(app.agent_view_store().show_create_session_dialog());
    // @step And agent_view_store.should_auto_create_session() returns false
    assert!(!app.agent_view_store().should_auto_create_session());
    // @step And agent_view_store.current_session_index stays 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And navigator.active_view stays ViewMode::Agent
    assert_eq!(app.active_view(), ViewMode::Agent);
}

/// Scenario: Shift+Left with a single attached session exits AgentView back to the Board
#[test]
fn single_session_shift_left_exits_to_board_view() {
    // @step Given an App with one open session s-1 and current_session_index 0
    let mut app = app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And navigator.active_view is ViewMode::Agent
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step When the App dispatches Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then navigator.active_view is ViewMode::Board
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And agent_view_store.current_session_index stays 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And agent_view_store.show_create_session_dialog() returns false
    assert!(!app.agent_view_store().show_create_session_dialog());
}

/// Scenario: Shift+Right at the last index preserves the typed draft when opening the Create Session dialog
#[test]
fn shift_right_at_last_index_preserves_typed_draft() {
    // @step Given an App with three open sessions s-1, s-2, s-3 and current_session_index 2
    let mut app = app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionCreated(sid("s-3")));
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And the MultiLineInput value is "goodbye"
    app.navigator_mut().agent.input.set_value("goodbye");
    // @step When the App dispatches Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then agent_view_store.show_create_session_dialog() returns true
    assert!(app.agent_view_store().show_create_session_dialog());
    // @step And the MultiLineInput value is still "goodbye"
    assert_eq!(app.navigator().agent.input.value(), "goodbye");
    // @step And agent_view_store.current_session_index stays 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
}

/// Scenario: Shift+Left at the first index snapshots the outgoing draft before exiting to the Board
#[test]
fn shift_left_at_first_index_snapshots_draft_then_exits_to_board() {
    // @step Given an App with three open sessions s-1, s-2, s-3 and current_session_index 0
    let mut app = app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionCreated(sid("s-3")));
    // Walk back to index 0.
    app.dispatch(Action::SessionPrev); // 2 -> 1
    app.dispatch(Action::SessionPrev); // 1 -> 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And the MultiLineInput value is "pending"
    app.navigator_mut().agent.input.set_value("pending");
    // @step When the App dispatches Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then navigator.active_view is ViewMode::Board
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And open_sessions[0].input_draft is "pending"
    assert_eq!(
        app.agent_view_store().open_sessions()[0].input_draft,
        "pending"
    );
    // @step And agent_view_store.current_session_index stays 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
}

/// Scenario: Mid-list Shift+Right preserves the RPC-024 draft round-trip and session switch
#[test]
fn mid_list_shift_right_preserves_rpc024_draft_round_trip() {
    // @step Given an App with three open sessions s-1, s-2, s-3 and current_session_index 1
    let mut app = app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionCreated(sid("s-3")));
    app.dispatch(Action::SessionPrev); // 2 -> 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And the MultiLineInput value is "midway"
    app.navigator_mut().agent.input.set_value("midway");
    // @step And open_sessions[2].input_draft is "third"
    app.agent_view_store_mut()
        .set_input_draft(2, "third".to_string());
    // @step When the App dispatches Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then agent_view_store.current_session_index is 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    // @step And open_sessions[1].input_draft is "midway"
    assert_eq!(
        app.agent_view_store().open_sessions()[1].input_draft,
        "midway"
    );
    // @step And the MultiLineInput value is "third"
    assert_eq!(app.navigator().agent.input.value(), "third");
    // @step And agent_view_store.show_create_session_dialog() returns false
    assert!(!app.agent_view_store().show_create_session_dialog());
    // @step And navigator.active_view stays ViewMode::Agent
    assert_eq!(app.active_view(), ViewMode::Agent);
}

/// Scenario: cycle_session is removed from the AgentViewStore public API
#[test]
fn cycle_session_is_removed_from_public_api() {
    // @step Given the source file rust/fspec-tui/src/store/agent_view.rs
    let source = std::fs::read_to_string("src/store/agent_view.rs").expect("read agent_view.rs");
    // @step Then it does not contain the substring "pub fn cycle_session"
    assert!(
        !source.contains("pub fn cycle_session"),
        "agent_view.rs still exposes pub fn cycle_session — RPC-096 rule [3] violated"
    );
    // @step And it does contain the substring "pub fn navigate_next"
    let navigation_file = "src/store/agent_view/navigation.rs";
    let nav_source = std::fs::read_to_string(navigation_file).unwrap_or_default();
    let combined_has_next =
        source.contains("pub fn navigate_next") || nav_source.contains("pub fn navigate_next");
    assert!(
        combined_has_next,
        "navigate_next not found in agent_view.rs or agent_view/navigation.rs"
    );
    // @step And it does contain the substring "pub fn navigate_prev"
    let combined_has_prev =
        source.contains("pub fn navigate_prev") || nav_source.contains("pub fn navigate_prev");
    assert!(
        combined_has_prev,
        "navigate_prev not found in agent_view.rs or agent_view/navigation.rs"
    );
    // @step And the file has fewer than 300 lines, OR navigate_next/navigate_prev live in the sibling module rust/fspec-tui/src/store/agent_view/navigation.rs
    let line_count = source.lines().count();
    let nav_exists = std::path::Path::new(navigation_file).exists();
    assert!(
        line_count < 300 || nav_exists,
        "agent_view.rs is {line_count} lines and no sibling navigation.rs exists"
    );
}
