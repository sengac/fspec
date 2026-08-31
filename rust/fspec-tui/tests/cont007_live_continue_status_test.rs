#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/live-continue-status-indicator.feature
//!
//! CONT-007: live status-bar updates for the auto-continue / goal nudge
//! counter. A new state-only `StreamChunk::ContinueStateUpdate` is pushed
//! by the engine at every counter transition and folded into the TUI
//! chrome cache so the footer paints the REAL `nudges_used` instead of a
//! literal 0, while the per-nudge chat print is removed from the
//! transcript (CliOutput keeps the stdout line).
//!
//! Test surfaces (cont005/cont006 precedent):
//! - rpc-types serde round-trip + SessionContext state-only behavior,
//! - behavioral tests on the REAL production helpers
//!   (`continue_state_event`, `apply_finish_with_summary`) with a
//!   RecordingOutput callback sink (vi.fn()-style, no module mocks),
//! - App + MockBackend chunk-dispatch tests asserting chrome cache and
//!   `paint_footer` buffer output,
//! - source-shape pins (rpc082/083 precedent) for the stream-loop
//!   emission sites, twin mappings and JSON serializers that cannot be
//!   driven without a live provider stream.

use std::sync::Mutex;

use codelet_cli::interactive::continue_state::continue_state_event;
use codelet_cli::interactive::done_early_exit::apply_finish_with_summary;
use codelet_cli::interactive::output::{ContinueStateReason, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_fspec_tui::store::agent_view::chrome_state::ContinueLiveState;
use codelet_fspec_tui::views::agent::chrome_paint::{paint_footer, ChromeAreas};
use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{ContinueStateInfo, SessionId, StreamChunk};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::sync::Arc;

mod common;
use common::MockBackend;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Recording StreamOutput — callback event sink so tests can assert on the
/// emitted events (vi.fn()-style sink, not a module replacement).
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn events(&self) -> Vec<StreamEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Build a Session without depending on a single-credential environment
/// (mirrors cont006_goal_immediate_termination.rs::fresh_session).
fn fresh_session() -> Session {
    for name in ["claude", "openai", "gemini", "codex", "zai"] {
        if let Ok(pm) = codelet_providers::ProviderManager::with_provider(name) {
            return Session::from_provider_manager(pm);
        }
    }
    Session::new(None).expect("failed to create test session")
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

/// Paint the SessionFooter for `session` into a 60x1 buffer and return the
/// painted row text.
fn painted_footer(app: &App, session: &SessionId) -> String {
    let areas = ChromeAreas {
        header: Rect::new(0, 0, 0, 0),
        role: Rect::new(0, 0, 0, 0),
        scrollback: Rect::new(0, 0, 0, 0),
        footer: Rect::new(0, 0, 60, 1),
        input: Rect::new(0, 0, 0, 0),
    };
    let mut buf = Buffer::empty(areas.footer);
    paint_footer(&areas, &mut buf, app.agent_view_store(), Some(session));
    let mut text = String::new();
    for x in 0..60u16 {
        text.push_str(buf[(x, 0)].symbol());
    }
    // Collapse space runs: the double-width ⏩/🎯 glyphs leave a blank
    // continuation cell in the buffer that reads back as an extra space.
    let mut collapsed = String::new();
    let mut prev_space = false;
    for c in text.chars() {
        if c == ' ' {
            if !prev_space {
                collapsed.push(c);
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    collapsed
}

/// Strip `//` line comments so source-shape pins cannot false-positive on
/// commentary (cont009 precedent).
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut in_str = false;
            let bytes = line.as_bytes();
            let mut i = 0;
            while i + 1 < bytes.len() {
                if bytes[i] == b'"' {
                    in_str = !in_str;
                }
                if !in_str && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                    return &line[..i];
                }
                i += 1;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_source(rel: &str) -> String {
    let path = format!("{}/../{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

// ============================================================================
// Scenario: ContinueStateUpdate chunk round-trips and stays out of the
// transcript
// ============================================================================

#[test]
fn continue_state_update_chunk_round_trips_and_stays_out_of_the_transcript() {
    // @step Given a StreamChunk::ContinueStateUpdate with enabled true, budget 10, nudgesUsed 3, goalActive false, effectiveBudget 10
    let chunk = StreamChunk::continue_state_update(ContinueStateInfo {
        enabled: true,
        budget: 10,
        nudges_used: 3,
        goal_active: false,
        effective_budget: 10,
        goal_cleared: false,
        done_rejections: 0,
    });

    // @step When the chunk is serialized to JSON and deserialized back
    let json = serde_json::to_string(&chunk).expect("serialize ContinueStateUpdate");
    let back: StreamChunk = serde_json::from_str(&json).expect("deserialize ContinueStateUpdate");

    // @step Then the JSON payload uses the camelCase field names nudgesUsed, goalActive and effectiveBudget
    assert!(
        json.contains("nudgesUsed")
            && json.contains("goalActive")
            && json.contains("effectiveBudget"),
        "wire JSON must be camelCase; got: {json}"
    );

    // @step And the deserialized chunk carries the same field values
    match back {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(continue_state.enabled);
            assert_eq!(continue_state.budget, 10);
            assert_eq!(continue_state.nudges_used, 3);
            assert!(!continue_state.goal_active);
            assert_eq!(continue_state.effective_budget, 10);
        }
        other => panic!("expected ContinueStateUpdate after round-trip, got {other:?}"),
    }

    // @step And recording the chunk into a SessionContext adds zero scrollback chunks
    let mut ctx = SessionContext::new(sid("s-1"));
    ctx.record_chunk(&StreamChunk::continue_state_update(ContinueStateInfo {
        enabled: true,
        budget: 10,
        nudges_used: 3,
        goal_active: false,
        effective_budget: 10,
        goal_cleared: false,
        done_rejections: 0,
    }));
    assert_eq!(
        ctx.scrollback.chunk_count(),
        0,
        "ContinueStateUpdate is state-only — it must NEVER land in the transcript"
    );
}

// ============================================================================
// Scenario: Engine snapshot reports the effective goal budget
// ============================================================================

#[test]
fn engine_snapshot_reports_the_effective_goal_budget() {
    // @step Given a session with auto-continue enabled and budget 10 and 2 nudges used
    let mut session = fresh_session();
    session.continue_enabled = true;
    session.continue_budget = 10;
    session.continue_nudges_used = 2;

    // @step When the continue state event is built without an active goal
    let event = continue_state_event(&session, ContinueStateReason::TurnStart);

    // @step Then the event reports enabled true, budget 10, nudgesUsed 2, goalActive false and effectiveBudget 10
    assert!(event.enabled);
    assert_eq!(event.budget, 10);
    assert_eq!(event.nudges_used, 2);
    assert!(!event.goal_active);
    assert_eq!(event.effective_budget, 10);
    assert_eq!(event.reason, ContinueStateReason::TurnStart);

    // @step When a goal is set on the session and the event is rebuilt
    session.set_goal("ship the feature", None);
    let goal_event = continue_state_event(&session, ContinueStateReason::TurnStart);

    // @step Then the event reports goalActive true and effectiveBudget 15
    assert!(goal_event.goal_active);
    assert_eq!(
        goal_event.effective_budget, 15,
        "goal mode effective budget must be max(explicit, 15)"
    );

    // @step When the budget is raised to 20 and the event is rebuilt
    session.continue_budget = 20;
    let raised = continue_state_event(&session, ContinueStateReason::TurnStart);

    // @step Then the event reports effectiveBudget 20 and goalActive true
    assert_eq!(raised.effective_budget, 20);
    assert!(raised.goal_active);
}

// ============================================================================
// Scenario: Accepted done() teardown emits a reset counter state from the
// shared teardown
// ============================================================================

#[test]
fn accepted_done_teardown_emits_a_reset_counter_state_from_the_shared_teardown() {
    // @step Given a session with 4 nudges used and an active goal
    let session_id = uuid::Uuid::new_v4();
    let mut session = fresh_session();
    session.continue_enabled = true;
    session.continue_budget = 10;
    session.continue_nudges_used = 4;
    session.set_goal("make all tests pass", None);
    let output = RecordingOutput::new();

    // @step When apply_finish_with_summary runs for an accepted done() summary
    apply_finish_with_summary(&mut session, session_id, "all tests pass", &output);

    // @step Then a ContinueState event is emitted after the announcement with nudgesUsed 0 and goalActive false
    let events = output.events();
    let status_pos = events
        .iter()
        .position(
            |e| matches!(e, StreamEvent::Status(s) if s.starts_with("\u{1F3AF} goal satisfied:")),
        )
        .expect("goal teardown must announce the satisfied goal");
    let state_pos = events
        .iter()
        .position(|e| matches!(e, StreamEvent::ContinueState(_)))
        .expect("teardown must emit a ContinueState event");
    assert!(
        state_pos > status_pos,
        "ContinueState must be emitted AFTER the announcement"
    );
    match &events[state_pos] {
        StreamEvent::ContinueState(cs) => {
            assert_eq!(cs.nudges_used, 0, "teardown must reset the nudge counter");
            assert!(!cs.goal_active, "goal must be cleared before the emission");
            // [UPDATED BY CONT-008] the goal-clearing teardown now marks
            // its snapshot with the DEDICATED GoalSatisfied reason (wire
            // goalCleared: true) so the twins can write the chrome goal
            // state back; DoneAccepted remains for the non-goal case below.
            assert_eq!(cs.reason, ContinueStateReason::GoalSatisfied);
        }
        _ => unreachable!(),
    }

    // @step When the teardown runs again for a session without an active goal
    let mut plain = fresh_session();
    plain.continue_enabled = true;
    plain.continue_nudges_used = 2;
    let plain_output = RecordingOutput::new();
    apply_finish_with_summary(&mut plain, uuid::Uuid::new_v4(), "done", &plain_output);
    let plain_events = plain_output.events();

    // @step Then a ContinueState event is emitted with nudgesUsed 0
    assert!(
        plain_events
            .iter()
            .any(|e| matches!(e, StreamEvent::Status(s) if s == "\u{2713} done: done")),
        "non-goal teardown must keep the closing line"
    );
    assert!(
        plain_events.iter().any(|e| matches!(
            e,
            StreamEvent::ContinueState(cs)
                if cs.nudges_used == 0 && cs.reason == ContinueStateReason::DoneAccepted
        )),
        "non-goal teardown must emit the reset ContinueState event"
    );
}

// ============================================================================
// Scenario: Footer paints the real nudge counter from a live update
// ============================================================================

#[test]
fn footer_paints_the_real_nudge_counter_from_a_live_update() {
    // @step Given an App with an open session whose cached continue state is enabled with budget 10
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::InputSubmitted("/continue 10".to_string()));
    let before = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 context")
        .scrollback
        .chunk_count();

    // @step When a ContinueStateUpdate chunk with nudgesUsed 2 and effectiveBudget 10 is dispatched for the session
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 2,
            goal_active: false,
            effective_budget: 10,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));

    // @step Then the session scrollback gains no transcript chunk
    let after = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("s-1 context")
        .scrollback
        .chunk_count();
    assert_eq!(
        after, before,
        "state-only chunk must not touch the transcript"
    );

    // @step And the painted footer shows "⏩ auto-continue (2/10)"
    let text = painted_footer(&app, &sid("s-1"));
    assert!(
        text.contains("\u{23E9} auto-continue (2/10)"),
        "footer must paint the live counter; painted: {text:?}"
    );

    // @step When a ContinueStateUpdate chunk with goalActive true, nudgesUsed 1 and effectiveBudget 15 is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 1,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));

    // @step Then the painted footer shows "🎯 goal (1/15)"
    let goal_text = painted_footer(&app, &sid("s-1"));
    assert!(
        goal_text.contains("\u{1F3AF} goal (1/15)"),
        "footer must paint the goal indicator with the effective budget; painted: {goal_text:?}"
    );
}

// ============================================================================
// Scenario: Slash dispatch drops the stale live counter
// ============================================================================

#[test]
fn slash_dispatch_drops_the_stale_live_counter() {
    // @step Given an App session whose live counter cache reports 7 nudges used
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 7,
            goal_active: false,
            effective_budget: 10,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));
    assert_eq!(
        app.agent_view_store()
            .continue_live_for(&sid("s-1"))
            .map(|l| l.nudges_used),
        Some(7),
        "test precondition: the live counter cache must hold 7"
    );

    // @step When the user applies "/continue 20" through the continue dispatch
    app.dispatch(Action::InputSubmitted("/continue 20".to_string()));

    // @step Then the live counter cache for the session is cleared
    assert!(
        app.agent_view_store()
            .continue_live_for(&sid("s-1"))
            .is_none(),
        "slash dispatch must drop the stale live counter"
    );

    // @step And the painted footer shows "⏩ auto-continue (0/20)"
    let text = painted_footer(&app, &sid("s-1"));
    assert!(
        text.contains("\u{23E9} auto-continue (0/20)"),
        "footer must fall back to the cached (enabled, budget) pair; painted: {text:?}"
    );
    // Silence the unused-import lint for ContinueLiveState until the goal
    // dispatch assertion below uses it structurally.
    let _: Option<ContinueLiveState> = app.agent_view_store().continue_live_for(&sid("s-1"));

    // @step When a ContinueStateUpdate chunk repopulates the live counter and the user applies "/goal ship it" through the goal dispatch
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 20,
            nudges_used: 3,
            goal_active: false,
            effective_budget: 20,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));
    app.dispatch(Action::InputSubmitted("/goal ship it".to_string()));

    // @step Then the /goal dispatch also drops the live counter state
    assert!(
        app.agent_view_store()
            .continue_live_for(&sid("s-1"))
            .is_none(),
        "/goal dispatch must also drop the stale live counter"
    );
}

// ============================================================================
// Scenario: Emission sites, twin mappings, and chat-print removal are pinned
// in the sources
// ============================================================================

#[test]
fn emission_sites_twin_mappings_and_chat_print_removal_are_pinned_in_the_sources() {
    // @step Given the codelet workspace sources
    let stream_loop = strip_line_comments(&read_source("cli/src/interactive/stream_loop.rs"));
    let done_early_exit =
        strip_line_comments(&read_source("cli/src/interactive/done_early_exit.rs"));
    let output_rs = strip_line_comments(&read_source("cli/src/interactive/output.rs"));
    let agent_loop_twin = strip_line_comments(&read_source("agent-loop/src/background_output.rs"));
    let napi_twin = strip_line_comments(&read_source("napi/src/agent_loop.rs"));
    let agent_loop_json = strip_line_comments(&read_source("agent-loop/src/stream_chunk_json.rs"));
    let napi_json = strip_line_comments(&read_source("napi/src/types.rs"));

    // @step Then stream_loop.rs emits the continue state at the turn start, refund settle, nudge and exhaustion sites
    for reason in [
        "ContinueStateReason::TurnStart",
        "ContinueStateReason::RefundSettled",
        "ContinueStateReason::NudgeConsumed",
        "ContinueStateReason::BudgetExhausted",
    ] {
        assert!(
            stream_loop.contains(reason),
            "stream_loop.rs must emit the continue state with {reason}"
        );
    }
    assert!(
        stream_loop.matches("emit_continue_state").count() >= 4,
        "stream_loop.rs must call emit_continue_state at all four transition sites"
    );

    // @step And stream_loop.rs no longer emits the "⏩ auto-continue: nudging" status message
    assert!(
        !stream_loop.contains("auto-continue: nudging"),
        "the per-nudge chat print must be REMOVED from stream_loop.rs — the counter belongs in the bar"
    );

    // @step And done_early_exit.rs emits the continue state at the tail of apply_finish_with_summary
    assert!(
        done_early_exit.contains("ContinueStateReason::DoneAccepted")
            && done_early_exit.contains("emit_continue_state"),
        "apply_finish_with_summary must emit the DoneAccepted continue state so BOTH exit sites are covered"
    );

    // @step And both background twins map StreamEvent::ContinueState to StreamChunk::ContinueStateUpdate
    for (name, twin) in [("agent-loop", &agent_loop_twin), ("napi", &napi_twin)] {
        assert!(
            twin.contains("StreamEvent::ContinueState"),
            "{name} twin must match the ContinueState event"
        );
        assert!(
            twin.contains("continue_state_update"),
            "{name} twin must map to StreamChunk::continue_state_update"
        );
    }

    // @step And both stream_chunk_to_json_value copies serialize the variant as type "continueStateUpdate"
    for (name, json_src) in [("agent-loop", &agent_loop_json), ("napi", &napi_json)] {
        assert!(
            json_src.contains("continueStateUpdate"),
            "{name} stream_chunk_to_json_value must serialize the variant as continueStateUpdate"
        );
    }

    // @step And CliOutput renders the nudging line from the ContinueState event using the effective budget
    assert!(
        output_rs.contains("StreamEvent::ContinueState"),
        "CliOutput must have a ContinueState arm"
    );
    assert!(
        output_rs.contains("auto-continue: nudging"),
        "the CLI stdout nudging line moves to the CliOutput ContinueState arm"
    );
    assert!(
        output_rs.contains("effective_budget"),
        "the CLI nudging line must print the effective budget, not continue_budget"
    );
}
