//! RPC-071 — AgentView chunk_to_lines parity with TS Ink frontend.
//!
//! Feature: spec/features/agentview-chunk-to-lines-parity-with-ts-ink-userinput-rendered-as-user-rpc-045-state-chunks-consumed-silently.feature
//!
//! Regression guard for the bug captured in the 2026-05-27 screenshot:
//! AgentView's `chunk_to_lines` had a catch-all `format!("{other:?}")`
//! arm that Debug-printed every unhandled `StreamChunk` variant into
//! scrollback (e.g. `UserInput { text: "..." }`,
//! `SessionStateChange { state: Running }`).
//!
//! These tests drive the full `App::dispatch` path the production binary
//! uses (not just the bare `SessionContext`), feeding chunks the way the
//! real broadcast subscriber does, and assert that:
//!   • Visible variants render with the right prefix
//!   • Silent variants produce NO scrollback line and DO NOT bump seq
//!   • State mutations on silent chunks still fire (status pill, etc.)
//!
//! **RPC-078 updated prefix assertions**: the original RPC-071 ladder
//! used Debug-style literals ("user>", "assistant>", "[done]",
//! "[interrupted] N queued", "supervisor>") that diverged from the
//! TS Ink reference. RPC-078 corrected the ladder to match TS Ink
//! (`You:`, `● `, no Done line, `⚠ Interrupted`, `[W] <role>>`) and
//! these tests now pin the corrected ladder.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{
    CompactionResult, ContextFillInfo, FspecRequest, FspecResult, SessionId, SessionState,
    SessionStatus, StreamChunk, SupervisorPendingInjectionInfo, TokenTracker,
};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Build an App with session `s-1` pre-created and return it plus the
/// SessionId for convenience.
fn app_with_session() -> App {
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app
}

/// Flatten every visible chunk's text in `id`'s scrollback into a Vec<String>.
fn session_scrollback_lines(app: &App, id: &SessionId) -> Vec<String> {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect()
}

fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
}

fn session_next_seq(app: &App, id: &SessionId) -> u64 {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback_next_seq)
        .unwrap_or(0)
}

fn ctx<'a>(app: &'a App, id: &SessionId) -> &'a SessionContext {
    app.agent_view_store()
        .session_context_for(id)
        .expect("SessionContext present")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Screenshot reproduction renders only the user input line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn screenshot_reproduction_renders_only_the_user_input_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards UserInput { text: "please review this card" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("please review this card".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly one line equal to "You: please review this card" (RPC-078 prefix correction)
    let lines = session_scrollback_lines(&app, &sid("s-1"));
    assert_eq!(lines.len(), 1, "expected 1 scrollback line, got {lines:?}");
    assert_eq!(lines[0], "You: please review this card");

    // @step When the chunks subscriber forwards SessionStateChange { state: Running } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::session_state_change(SessionState::Running),
    ));
    // @step When the chunks subscriber forwards SessionStateChange { state: Idle } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::session_state_change(SessionState::Idle),
    ));

    // @step Then no scrollback line contains the literal substring "SessionStateChange" or "UserInput {"
    let after = session_scrollback_lines(&app, &sid("s-1"));
    assert_eq!(
        after,
        vec!["You: please review this card".to_string()],
        "scrollback must contain ONLY the user input line; got {after:?}",
    );
    for line in &after {
        assert!(
            !line.contains("SessionStateChange"),
            "scrollback leaked Debug output: {line}"
        );
        assert!(
            !line.contains("UserInput {"),
            "scrollback leaked Debug output: {line}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: UserInput chunk renders as user-prefix scrollback line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn user_input_chunk_renders_as_user_prefix_scrollback_line() {
    // @step Given a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When record_chunk receives StreamChunk::UserInput { text: "hello" }
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("hello".to_string()),
    ));

    // @step Then the scrollback contains exactly one rendered chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step Then the rendered text equals "You: hello" (RPC-078 prefix correction)
    let lines = session_scrollback_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["You: hello".to_string()]);

    // @step Then the scrollback_next_seq cursor equals 1
    assert_eq!(session_next_seq(&app, &sid("s-1")), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Two SessionStateChange chunks leave scrollback empty and seq at zero
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn two_session_state_change_chunks_leave_scrollback_empty_and_seq_at_zero() {
    // @step Given a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When record_chunk receives StreamChunk::SessionStateChange { state: Running }
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::session_state_change(SessionState::Running),
    ));
    // @step Then the scrollback is empty
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);

    // @step When record_chunk receives StreamChunk::SessionStateChange { state: Idle }
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::session_state_change(SessionState::Idle),
    ));

    // @step Then the scrollback_next_seq cursor remains 0
    assert_eq!(session_next_seq(&app, &sid("s-1")), 0);
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Interleaved visible and silent chunks keep seq monotonic over
// visible chunks only
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn interleaved_visible_and_silent_chunks_keep_seq_monotonic_over_visible_chunks_only() {
    // @step Given a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When record_chunk receives UserInput { text: "hi" }, SessionStateChange(Running), Text { text: "hello back" }, SessionStateChange(Idle), Done in that order
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("hi".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::session_state_change(SessionState::Running),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("hello back".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::session_state_change(SessionState::Idle),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the scrollback contains exactly 2 rendered chunks (RPC-078: Done emits NO line)
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);

    // @step Then the rendered scrollback lines equal ["You: hi", "● hello back"] (RPC-078 prefix correction; Done emits no line)
    assert_eq!(
        session_scrollback_lines(&app, &sid("s-1")),
        vec!["You: hi".to_string(), "\u{25CF} hello back".to_string(),]
    );

    // @step Then the scrollback_next_seq cursor equals 2 (RPC-078: Done does not bump seq)
    assert_eq!(session_next_seq(&app, &sid("s-1")), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: IncomingMessage chunk renders as supervisor-prefix line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn incoming_message_chunk_renders_as_supervisor_prefix_line() {
    // @step Given a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When record_chunk receives StreamChunk::IncomingMessage { text: "supervisor here", images: None }
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::IncomingMessage {
            text: "supervisor here".to_string(),
            images: None,
        },
    ));

    // @step Then the scrollback contains exactly one rendered chunk whose line equals "[W] supervisor> supervisor here" (RPC-078: bare body w/ no envelope falls back to default role "supervisor")
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    assert_eq!(
        session_scrollback_lines(&app, &sid("s-1")),
        vec!["[W] supervisor> supervisor here".to_string()]
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Interrupted chunk renders as interrupted-with-queued-count line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn interrupted_chunk_renders_as_interrupted_with_queued_count_line() {
    // @step Given a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When record_chunk receives StreamChunk::Interrupted { queued_inputs: vec!["a", "b"] }
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::interrupted(vec!["a".to_string(), "b".to_string()]),
    ));

    // @step Then the scrollback contains exactly one rendered chunk whose line equals "⚠ Interrupted" (RPC-078: queue count is suppressed; the TS Ink reference renders a flat warning line)
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    assert_eq!(
        session_scrollback_lines(&app, &sid("s-1")),
        vec!["\u{26A0} Interrupted".to_string()]
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: All state-only and tool variants are suppressed from scrollback
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn all_state_only_and_tool_variants_are_suppressed_from_scrollback() {
    // @step Given a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When record_chunk receives one of each suppressed variant
    //         (SessionStateChange, IsolationStateChange, DebugStateChange,
    //         FooterStateUpdate, FspecCommandRequest, FspecCommandResult,
    //         WorkUnitsUpdate, SupervisorPendingInjection,
    //         CompactionComplete, TokenUpdate, ContextFillUpdate)
    // RPC-091: ToolCall/ToolResult/ToolProgress are NO LONGER suppressed —
    // they render as visible cards per chunkProcessor parity. Covered by
    // spec/features/agentview-chunkprocessor-parity.feature instead.
    let suppressed: Vec<StreamChunk> = vec![
        StreamChunk::session_state_change(SessionState::Running),
        StreamChunk::IsolationStateChange {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc".to_string()),
        },
        StreamChunk::DebugStateChange { enabled: true },
        StreamChunk::FooterStateUpdate {
            cwd: "/tmp".to_string(),
            display_path: "/tmp".to_string(),
            is_git_repo: false,
            branch: None,
        },
        StreamChunk::FspecCommandRequest {
            fspec_request: FspecRequest {
                command: "list".to_string(),
                args_json: "{}".to_string(),
                project_root: ".".to_string(),
                tool_call_id: "t-1".to_string(),
            },
        },
        StreamChunk::FspecCommandResult {
            fspec_result: FspecResult {
                success: true,
                data: "[]".to_string(),
                error: None,
                system_reminder: None,
                tool_call_id: "t-1".to_string(),
            },
        },
        StreamChunk::WorkUnitsUpdate { work_units: vec![] },
        StreamChunk::SupervisorPendingInjection {
            supervisor_pending_injection: SupervisorPendingInjectionInfo {
                urgent: false,
                content: "x".to_string(),
            },
        },
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 0,
                compacted_tokens: 0,
                compression_ratio: 0.0,
                turns_summarized: 0,
                turns_kept: 0,
            },
        },
        StreamChunk::TokenUpdate {
            tokens: TokenTracker::default(),
        },
        StreamChunk::ContextFillUpdate {
            context_fill: ContextFillInfo {
                fill_percentage: 0,
                effective_tokens: 0.0,
                threshold: 0.0,
                context_window: 0.0,
            },
        },
    ];
    for c in suppressed {
        app.dispatch(Action::ChunkReceived(sid("s-1"), c));
    }

    // @step Then the scrollback is empty
    assert_eq!(
        session_chunk_count(&app, &sid("s-1")),
        0,
        "state-only chunks must not produce scrollback lines; got {:?}",
        session_scrollback_lines(&app, &sid("s-1"))
    );

    // @step Then the scrollback_next_seq cursor remains 0
    assert_eq!(
        session_next_seq(&app, &sid("s-1")),
        0,
        "suppressed chunks must not bump the seq cursor"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionStateChange still mutates per-session status pill even
// when suppressed from scrollback
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn session_state_change_still_mutates_per_session_status_pill_even_when_suppressed_from_scrollback()
{
    // @step Given an App with an open session s-1
    let mut app = app_with_session();

    // @step When the App dispatches Action::ChunkReceived(s-1, SessionStateChange { state: Idle })
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Idle,
        },
    ));

    // @step Then the agent_view_store.session_status_for(&s-1) returns SessionStatus::Idle
    assert_eq!(
        app.agent_view_store().session_status_for(&sid("s-1")),
        Some(&SessionStatus::Idle),
        "SessionStateChange must mutate per-session status even when no scrollback line is produced"
    );

    // @step Then the s-1 scrollback remains empty
    let lines = session_scrollback_lines(&app, &sid("s-1"));
    assert!(
        lines.is_empty(),
        "SessionStateChange must NOT leak a scrollback line; got {lines:?}"
    );
    // Sanity — also assert the SessionContext exists.
    assert_eq!(ctx(&app, &sid("s-1")).scrollback.chunk_count(), 0);
}
