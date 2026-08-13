// Feature: spec/features/agent-input-bracketed-paste-routing.feature
//! RPC-403 — bracketed-paste routing into the agent input (red phase).
//!
//! Seams under test (all EXISTING public APIs whose behavior the fix
//! changes — no to-be-created signatures, so this file always compiles):
//!
//!   1. `App::handle_paste(&str)` (app/events.rs:157-167) — today it
//!      forwards ONLY to `Compositor::handle_paste`, whose char-splitting
//!      stub (compositor.rs:188-209) drops the paste when no modal layer
//!      is open. The fix must fall back through Navigator → AgentView →
//!      `MultiLineInput::handle_event` (Event::Paste branch).
//!   2. `Compositor::handle_paste` — today explodes the paste into
//!      per-char synthetic `KeyCode::Char` events; the fix must deliver
//!      ONE real `Event::Paste(String)` to the top modal layer.
//!   3. `MultiLineInput::handle_event(Event::Paste)` — today inserts
//!      raw text including `\r`; the fix must normalize CRLF → LF.
//!   4. The paste path must respect the RPC-095 `block_edits`
//!      (Compacting) gate. Today `AgentView::handle_event` routes
//!      `Event::Paste` through `input.handle_event(other)` which
//!      BYPASSES the gate (dispatch.rs:247-250) — pinned via
//!      `App::handle_event(&Event::Paste)` (Navigator path exists today).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use codelet_fspec_tui::views::agent::multiline_input::MultiLineInput;
use codelet_fspec_tui::{Component, EventResult, Priority, ViewMode};
use codelet_rpc_types::SessionStatus;
use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;

use common::harness::{seed_sid, AppTestHarness};

/// Minimal modal layer that records every event it receives and
/// consumes them all (modal semantics). Shared `Arc<Mutex<Vec<Event>>>`
/// lets the test inspect what the compositor actually delivered.
struct RecordingModal {
    events: Arc<Mutex<Vec<Event>>>,
}

impl Component for RecordingModal {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }
    fn id(&self) -> &str {
        "recording-modal"
    }
    fn handle_event(&mut self, event: &Event) -> EventResult {
        self.events.lock().unwrap().push(event.clone());
        EventResult::consumed()
    }
    fn render(&mut self, _area: Rect, _buf: &mut Buffer) {}
}

/// Render the App once into an off-screen buffer so per-frame caches
/// (`AgentView::last_is_compacting` — the RPC-095 gate input) refresh.
fn render_once(h: &mut AppTestHarness) {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    h.app.render(area, &mut buf);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Multi-line paste with no modal open is inserted verbatim and
//           grows the input
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn multi_line_paste_with_no_modal_open_is_inserted_verbatim_and_grows_the_input() {
    // @step Given the agent view is active with no modal open
    let mut h = AppTestHarness::new();
    assert_eq!(h.app.active_view(), ViewMode::Agent);
    assert_eq!(h.app.compositor().len(), 0, "no modal must be open");

    // @step And the agent input is empty
    assert_eq!(h.app.navigator().agent.input.value(), "");

    // @step When I paste a 3-line snippet
    let _ = h.app.handle_paste("line one\nline two\nline three");

    // @step Then the input buffer contains all 3 lines separated by newlines
    assert_eq!(
        h.app.navigator().agent.input.value(),
        "line one\nline two\nline three",
        "RPC-403: paste with no modal open must fall through to the agent input"
    );

    // @step And the input area reports 3 visible rows
    assert_eq!(h.app.navigator().agent.input.visible_rows(), 3);

    // @step And the cursor is at the end of the pasted text
    assert_eq!(
        h.app.navigator().agent.input.cursor(),
        (2, "line three".len()),
        "cursor must land at the end of the pasted text"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CRLF line endings are normalized to LF on paste
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn crlf_line_endings_are_normalized_to_lf_on_paste() {
    // @step Given the agent view is active with no modal open
    // Full routed path: App::handle_paste → (fallback) Navigator →
    // AgentView → MultiLineInput Event::Paste branch. Note: at the
    // widget level tui-textarea's `insert_str` already normalizes CRLF
    // (verified while writing this test) — the fix must preserve that
    // through whatever normalization/sanitization it adds, and this
    // test fails today because the paste never reaches the input.
    let mut h = AppTestHarness::new();
    assert_eq!(h.app.active_view(), ViewMode::Agent);
    assert_eq!(h.app.compositor().len(), 0, "no modal must be open");

    // @step When I paste text containing Windows CRLF line endings
    let _ = h.app.handle_paste("foo\r\nbar");

    // @step Then the input buffer contains the lines separated by LF only
    assert_eq!(
        h.app.navigator().agent.input.value(),
        "foo\nbar",
        "CRLF must be normalized to LF before insertion"
    );

    // @step And the input buffer contains no carriage return characters
    assert!(
        !h.app.navigator().agent.input.value().contains('\r'),
        "no '\\r' may enter the buffer; got {:?}",
        h.app.navigator().agent.input.value()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Paste is inserted at the cursor preserving existing text
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn paste_is_inserted_at_the_cursor_preserving_existing_text() {
    // @step Given the agent input contains "prefix" with the cursor at the end
    let mut input = MultiLineInput::new();
    input.set_value("prefix");
    assert_eq!(input.cursor(), (0, 6), "precondition: cursor at end");

    // @step When I paste "pasted"
    let _ = input.handle_event(&Event::Paste("pasted".to_string()));

    // @step Then the input buffer contains "prefixpasted"
    assert_eq!(input.value(), "prefixpasted");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Paste while a modal is open is delivered to the modal intact
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn paste_while_a_modal_is_open_is_delivered_to_the_modal_intact() {
    // @step Given a modal layer with a paste handler is open above the agent view
    let mut h = AppTestHarness::new();
    h.app.navigator_mut().agent.input.set_value("agent draft");
    let recorded: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
    h.app.compositor_mut().push(Box::new(RecordingModal {
        events: recorded.clone(),
    }));

    // @step When I paste a multi-line string
    let _ = h.app.handle_paste("multi\nline\npaste");

    // @step Then the modal receives the full paste string as a single paste event
    let events = recorded.lock().unwrap();
    let pastes: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::Paste(_)))
        .collect();
    assert_eq!(
        pastes.len(),
        1,
        "modal must receive exactly ONE Event::Paste (got {} paste events \
         among {} total events — the char-splitting stub sends synthetic keys)",
        pastes.len(),
        events.len()
    );
    match pastes[0] {
        Event::Paste(s) => assert_eq!(s, "multi\nline\npaste", "paste payload must be intact"),
        _ => panic!("filtered event must be Event::Paste"),
    }
    assert!(
        !events.iter().any(|e| matches!(e, Event::Key(_))),
        "the paste must NOT be exploded into synthetic key events"
    );
    drop(events);

    // @step And the agent input buffer is unchanged
    assert_eq!(
        h.app.navigator().agent.input.value(),
        "agent draft",
        "a consumed modal paste must never leak into the agent input"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Paste while the session is compacting is suppressed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn paste_while_the_session_is_compacting_is_suppressed() {
    // @step Given the agent input contains "draft" and the compacting edit gate is active
    let mut h = AppTestHarness::new();
    h.app.navigator_mut().agent.input.set_value("draft");
    h.app
        .agent_view_store_mut()
        .set_session_status(seed_sid(), SessionStatus::Compacting);
    // A frame render caches the gate (AgentView::last_is_compacting).
    render_once(&mut h);

    // @step When I paste "more text"
    // Both delivery paths must be gated: the App paste entry point AND
    // the Navigator → AgentView::handle_event(Event::Paste) path (the
    // route the fallback fix uses; it exists today and bypasses the
    // gate via dispatch.rs:247-250 → runtime FAIL expected).
    let _ = h.app.handle_paste("more text");
    let _ = h.app.handle_event(&Event::Paste("more text".to_string()));

    // @step Then the input buffer still contains only "draft"
    assert_eq!(
        h.app.navigator().agent.input.value(),
        "draft",
        "paste must be suppressed by the block_edits (Compacting) gate"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Large paste keeps the full buffer but caps the input at 6
//           visible rows
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn large_paste_keeps_the_full_buffer_but_caps_the_input_at_6_visible_rows() {
    // @step Given the agent view is active with no modal open
    let mut h = AppTestHarness::new();
    assert_eq!(h.app.active_view(), ViewMode::Agent);
    assert_eq!(h.app.compositor().len(), 0, "no modal must be open");

    // @step And the agent input is empty
    assert_eq!(h.app.navigator().agent.input.value(), "");

    // @step When I paste a 10-line snippet
    let snippet = (1..=10)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = h.app.handle_paste(&snippet);

    // @step Then the input buffer contains all 10 lines
    let value = h.app.navigator().agent.input.value();
    assert_eq!(
        value.split('\n').count(),
        10,
        "all 10 pasted lines must be in the buffer; got {value:?}"
    );
    assert_eq!(value, snippet);

    // @step And the input area reports 6 visible rows
    assert_eq!(
        h.app.navigator().agent.input.visible_rows(),
        6,
        "visible rows must clamp at the 6-row cap"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Supplementary test (RPC-403 review, warning 2 — documented, no @step
// comments): RPC-411 replaced the HitlDialog modal with the inline HITL
// prompt. Options mode must CONSUME Event::Paste (swallow it) so a
// paste can never leak into the composer draft; freeform/Other mode
// inserts the paste into the SHARED composer input (TS parity — the
// real MultiLineInput is active). See
// tests/inline_hitl_prompt_rpc411.rs for the full key/paste matrix.
// ─────────────────────────────────────────────────────────────────────────

fn rpc411_hitl_request() -> codelet_rpc_types::HitlRequest {
    use codelet_rpc_types::{HitlOption, HitlQuestion, HitlRequest};
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "q-1".to_string(),
            header: "Apply?".to_string(),
            question: "Run tool?".to_string(),
            options: vec![
                HitlOption {
                    label: "Yes".to_string(),
                    description: "run".to_string(),
                },
                HitlOption {
                    label: "No".to_string(),
                    description: "skip".to_string(),
                },
            ],
        }],
    }
}

#[test]
fn paste_while_hitl_options_prompt_is_active_is_swallowed_and_never_reaches_the_agent_input() {
    use codelet_fspec_tui::Action;
    use codelet_rpc_types::SessionId;

    // Given: the agent input holds a draft and the focused session has
    // an active options-mode HITL slot (rendered once so the AgentView
    // caches the prompt for paste routing).
    let mut h = AppTestHarness::new();
    h.app
        .dispatch(Action::SessionCreated(SessionId::new("s-1")));
    h.app.navigator_mut().active_view = ViewMode::Agent;
    h.app.navigator_mut().agent.input.set_value("agent draft");
    h.app.dispatch(Action::HitlPromptFetched {
        session_id: SessionId::new("s-1"),
        request: rpc411_hitl_request(),
    });
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    h.app.render(area, &mut buf);

    // When: a multi-line paste arrives through the real App entry point.
    let _ = h.app.handle_paste("clipboard\ncontents");

    // Then: the paste is NOT inserted into the composer draft hidden
    // behind the options prompt (consumed/ignored)…
    assert_eq!(
        h.app.navigator().agent.input.value(),
        "agent draft",
        "paste while the options-mode HITL prompt shows must never reach the agent input"
    );

    // …and the prompt stays active (a swallowed paste must not answer
    // or cancel the request).
    assert!(
        h.app
            .agent_view_store()
            .hitl_prompt_for(&SessionId::new("s-1"))
            .is_some(),
        "swallowed paste must leave the HITL slot active and unchanged"
    );
}
