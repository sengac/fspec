#![cfg(not(feature = "noop"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/compaction-complete-measurement-basis.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.
//
// CMPCT-038: CompactionComplete must carry compacted_tokens measured from the
// recalculated post-injection token tracker (reminders + summary), NOT from
// the wrapped DAG summary text alone. The emission therefore moves to the
// apply site via `apply_pending_dag_and_emit`.
//
// BOTH twins are exercised here: codelet-agent-loop (Rust fspec-tui engine)
// and codelet-napi (TS Ink TUI). This is the only test crate that may see
// both twins — codelet-agent-loop MUST NOT depend on codelet-napi
// (enforced by agent-loop/tests/no_napi_dependency.rs), so the lock-step
// contract is validated from the napi side via a dev-dependency.
// Both twins share `codelet_rpc_types::StreamChunk`, so one recorder works
// for both.

use std::sync::{Arc, Mutex};

use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::{wrap_dag_content, DagNodeMeta};
use codelet_napi::{SessionState, StreamChunk};

// ========================================
// Test helpers
// ========================================

/// Signature shared by both twins' `apply_pending_dag_and_emit`.
type ApplyAndEmitFn = fn(
    &mut codelet_cli::session::Session,
    &Arc<Mutex<Option<String>>>,
    u32,
    &dyn Fn(StreamChunk),
) -> Option<Vec<DagNodeMeta>>;

/// Signature shared by both twins' `emit_post_injection_events`.
type EmitFn = fn(&dyn Fn(StreamChunk), u32, u32);

/// Both twins, exercised identically in every scenario.
fn twins() -> [(&'static str, ApplyAndEmitFn); 2] {
    [
        (
            "agent-loop",
            codelet_agent_loop::inject_summary_handler::apply_pending_dag_and_emit,
        ),
        (
            "napi",
            codelet_napi::inject_summary_handler::apply_pending_dag_and_emit,
        ),
    ]
}

fn emit_twins() -> [(&'static str, EmitFn); 2] {
    [
        (
            "agent-loop",
            codelet_agent_loop::inject_summary_handler::emit_post_injection_events,
        ),
        (
            "napi",
            codelet_napi::inject_summary_handler::emit_post_injection_events,
        ),
    ]
}

fn create_test_session() -> codelet_cli::session::Session {
    let provider_manager = codelet_providers::ProviderManager::new()
        .or_else(|_| codelet_providers::ProviderManager::with_provider("gemini"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("zai"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("claude"))
        .expect("Need at least one API key for tests");
    codelet_cli::session::Session::from_provider_manager(provider_manager)
}

/// Build a large system-reminder message body (roughly `approx_tokens` tokens,
/// exact size does not matter — tests measure with `count_tokens` themselves).
/// Must contain both the `<system-reminder>` tag and a `<!-- type: -->`
/// marker so `reset_session_to_reminders` preserves it through the apply.
fn big_reminder(approx_tokens: usize) -> String {
    // "environment data padding " is 25 chars ≈ 6 estimator tokens.
    let filler = "environment data padding ".repeat((approx_tokens / 6).max(1));
    format!(
        "<system-reminder>\n<!-- type:environment -->\n{}\n</system-reminder>",
        filler
    )
}

fn push_user_message(session: &mut codelet_cli::session::Session, text: &str) {
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(text)),
    });
}

type Recorded = Arc<Mutex<Vec<StreamChunk>>>;

fn recorder() -> (Recorded, impl Fn(StreamChunk)) {
    let chunks: Recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = chunks.clone();
    (chunks, move |chunk: StreamChunk| {
        sink.lock().unwrap().push(chunk);
    })
}

fn find_compaction_result(chunks: &[StreamChunk]) -> Option<codelet_napi::CompactionResult> {
    chunks.iter().find_map(|c| match c {
        StreamChunk::CompactionComplete { compaction_result } => Some(compaction_result.clone()),
        _ => None,
    })
}

// ========================================
// Scenario: compacted_tokens reflects the real post-injection context, not the summary alone
// ========================================

#[test]
fn test_compacted_tokens_uses_recalculated_tracker_not_summary() {
    for (twin, apply_and_emit) in twins() {
        // @step Given a session whose messages include large system reminders that survive compaction
        let mut session = create_test_session();
        let reminder = big_reminder(8_000);
        push_user_message(&mut session, &reminder);
        push_user_message(&mut session, "old conversation to be compacted away");

        // @step And a pending DAG summary that is far smaller than the surviving reminders
        let summary = "# D2: Architecture\n- Decision A\n# D0: Recent\n- Current task";
        let wrapped = wrap_dag_content(summary);
        let summary_tokens = count_tokens(&wrapped) as u32;
        let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some(wrapped)));

        // @step And pre-compaction original tokens far larger than the post-injection context
        let original_tokens: u32 = 100_000;

        // @step When the agent loop applies the pending DAG and emits the completion chunk
        let (chunks, emit) = recorder();
        let applied = apply_and_emit(&mut session, &pending_dag, original_tokens, &emit);
        assert!(applied.is_some(), "[{twin}] DAG should have been applied");

        // @step Then the emitted CompactionComplete compacted_tokens equals the session's recalculated token tracker total
        let chunks = chunks.lock().unwrap();
        let result = find_compaction_result(&chunks).expect("CompactionComplete must be emitted");
        assert_eq!(
            result.compacted_tokens as u64, session.token_tracker.input_tokens,
            "[{twin}] compacted_tokens must equal the recalculated post-injection tracker total"
        );

        // @step And the compacted_tokens is greater than the token count of the wrapped summary alone
        assert!(
            result.compacted_tokens > summary_tokens,
            "[{twin}] compacted_tokens ({}) must include surviving reminders, not just the summary ({})",
            result.compacted_tokens,
            summary_tokens
        );

        // @step And the compression_ratio equals the percent removed computed on the recalculated basis
        let expected_ratio = codelet_cli::interactive_helpers::compression_ratio(
            original_tokens as u64,
            session.token_tracker.input_tokens,
        ) * 100.0;
        assert!(
            (result.compression_ratio - expected_ratio.max(0.0)).abs() < 1e-9,
            "[{twin}] compression_ratio {} must be computed on the recalculated basis (expected {})",
            result.compression_ratio,
            expected_ratio
        );
        assert_eq!(result.original_tokens, original_tokens, "[{twin}]");
    }
}

// ========================================
// Scenario: A real ~60 percent reduction is reported as ~60, never ~99
// ========================================

#[test]
fn test_sixty_percent_reduction_reports_sixty_not_ninety_nine() {
    for (twin, apply_and_emit) in twins() {
        // @step Given a session whose post-injection context is about 40 percent of the original size
        let mut session = create_test_session();
        let reminder = big_reminder(8_000);
        push_user_message(&mut session, &reminder);

        let summary = "# D2: Architecture\n- JWT auth decision\n# D1: Arc\n- Building login";
        let wrapped = wrap_dag_content(summary);
        let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some(wrapped.clone())));

        // Post-injection context will be exactly [surviving reminder, wrapped summary].
        let expected_post_total = count_tokens(&reminder) as u64 + count_tokens(&wrapped) as u64;
        // Choose original so the true reduction is 60%: post/original = 0.4
        let original_tokens = ((expected_post_total as f64) / 0.4).round() as u32;

        // @step When the agent loop applies the pending DAG and emits the completion chunk
        let (chunks, emit) = recorder();
        let applied = apply_and_emit(&mut session, &pending_dag, original_tokens, &emit);
        assert!(applied.is_some(), "[{twin}] DAG should have been applied");

        let chunks = chunks.lock().unwrap();
        let result = find_compaction_result(&chunks).expect("CompactionComplete must be emitted");

        // @step Then the emitted compression_ratio is approximately 60.0
        assert!(
            (result.compression_ratio - 60.0).abs() < 1.0,
            "[{twin}] expected ~60.0 percent reduction, got {}",
            result.compression_ratio
        );

        // @step And the emitted compression_ratio is nowhere near the ~99.0 the summary-only basis would produce
        let summary_only_ratio = codelet_cli::interactive_helpers::compression_ratio(
            original_tokens as u64,
            count_tokens(&wrapped) as u64,
        ) * 100.0;
        assert!(
            summary_only_ratio > 90.0,
            "[{twin}] sanity: the buggy summary-only basis would report {} (must be >90 for this fixture)",
            summary_only_ratio
        );
        assert!(
            result.compression_ratio < 90.0,
            "[{twin}] compression_ratio {} must NOT be the summary-only fantasy value {}",
            result.compression_ratio,
            summary_only_ratio
        );
    }
}

// ========================================
// Scenario: compression_ratio is clamped to zero when the post-injection context exceeds the original
// ========================================

#[test]
fn test_ratio_clamped_to_zero_when_post_exceeds_original() {
    for (twin, apply_and_emit) in twins() {
        // @step Given a tiny session where surviving reminders plus summary exceed the original token count
        let mut session = create_test_session();
        push_user_message(&mut session, &big_reminder(2_000));
        let wrapped = wrap_dag_content("# D0: tiny\n- nothing much happened");
        let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some(wrapped)));
        let original_tokens: u32 = 100; // far below the surviving reminder size

        // @step When the agent loop applies the pending DAG and emits the completion chunk
        let (chunks, emit) = recorder();
        let applied = apply_and_emit(&mut session, &pending_dag, original_tokens, &emit);
        assert!(applied.is_some(), "[{twin}] DAG should have been applied");

        let chunks = chunks.lock().unwrap();
        let result = find_compaction_result(&chunks).expect("CompactionComplete must be emitted");

        // Sanity: this fixture really does have post > original
        assert!(
            session.token_tracker.input_tokens > original_tokens as u64,
            "[{twin}] fixture must have post-injection total > original"
        );

        // @step Then the emitted compression_ratio is exactly 0.0
        assert_eq!(
            result.compression_ratio, 0.0,
            "[{twin}] ratio must clamp to 0.0 when post-injection total exceeds original"
        );

        // @step And the compression_ratio is never negative on the wire
        assert!(
            result.compression_ratio >= 0.0,
            "[{twin}] ratio must never be negative on the wire, got {}",
            result.compression_ratio
        );
    }
}

/// Unit-level clamp guard on the emit helpers themselves: compacted > original
/// must never produce a negative ratio, in either twin.
#[test]
fn test_emit_post_injection_events_clamps_negative_ratio() {
    for (twin, emit_events) in emit_twins() {
        // @step Given a tiny session where surviving reminders plus summary exceed the original token count
        let (chunks, emit) = recorder();

        // @step When the agent loop applies the pending DAG and emits the completion chunk
        emit_events(
            &emit, 1_000, // original
            1_500, // compacted (post-injection dominated by reminders)
        );

        // @step Then the emitted compression_ratio is exactly 0.0
        // @step And the compression_ratio is never negative on the wire
        let chunks = chunks.lock().unwrap();
        let result = find_compaction_result(&chunks).expect("CompactionComplete must be emitted");
        assert!(
            result.compression_ratio >= 0.0,
            "[{twin}] ratio must never be negative, got {}",
            result.compression_ratio
        );
        assert_eq!(
            result.compression_ratio, 0.0,
            "[{twin}] ratio must clamp to exactly 0.0"
        );
    }
}

// ========================================
// Scenario: Running is emitted before CompactionComplete at the apply site
// ========================================

#[test]
fn test_running_emitted_before_compaction_complete_at_apply_site() {
    for (twin, apply_and_emit) in twins() {
        // @step Given a session with a pending DAG summary
        let mut session = create_test_session();
        push_user_message(&mut session, &big_reminder(500));
        let wrapped = wrap_dag_content("# D0: work\n- something");
        let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some(wrapped)));

        // @step When the agent loop applies the pending DAG and emits the completion chunk
        let (chunks, emit) = recorder();
        let applied = apply_and_emit(&mut session, &pending_dag, 50_000, &emit);
        assert!(applied.is_some(), "[{twin}]");

        // @step Then a SessionStateChange Running chunk is emitted before the CompactionComplete chunk
        let chunks = chunks.lock().unwrap();
        let running_idx = chunks
            .iter()
            .position(|c| matches!(c, StreamChunk::SessionStateChange { state } if *state == SessionState::Running))
            .expect("Must emit SessionStateChange(Running)");
        let complete_idx = chunks
            .iter()
            .position(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
            .expect("Must emit CompactionComplete");
        assert!(
            running_idx < complete_idx,
            "[{twin}] Running (idx {}) must precede CompactionComplete (idx {})",
            running_idx,
            complete_idx
        );

        // @step And no Idle or Done chunk is emitted by the apply-and-emit step itself
        assert!(
            !chunks.iter().any(|c| matches!(
                c,
                StreamChunk::SessionStateChange { state } if *state == SessionState::Idle
            ) || matches!(c, StreamChunk::Done)),
            "[{twin}] apply_pending_dag_and_emit must not emit Idle or Done"
        );
    }
}

// ========================================
// Scenario: Applying with no pending DAG emits nothing
// ========================================

#[test]
fn test_no_pending_dag_emits_nothing() {
    for (twin, apply_and_emit) in twins() {
        // @step Given a session with no pending DAG content
        let mut session = create_test_session();
        push_user_message(&mut session, "hello");
        let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        // @step When the agent loop runs the apply-and-emit step
        let (chunks, emit) = recorder();
        let applied = apply_and_emit(&mut session, &pending_dag, 10_000, &emit);

        // @step Then no chunk is emitted
        assert!(
            chunks.lock().unwrap().is_empty(),
            "[{twin}] no chunks may be emitted when there is no pending DAG"
        );

        // @step And the step reports that nothing was applied
        assert!(
            applied.is_none(),
            "[{twin}] must return None when nothing was applied"
        );
    }
}

// ========================================
// Scenario: The agent-loop and NAPI twins produce identical CompactionResult values
// ========================================

#[test]
fn test_twins_produce_identical_compaction_results() {
    // @step Given the same session shape with identical reminders, summary, and original tokens
    let reminder = big_reminder(4_000);
    let summary = "# D2: Architecture\n- Twin parity decision\n# D0: Recent\n- Parity check";
    let original_tokens: u32 = 80_000;

    let build_session = || {
        let mut session = create_test_session();
        push_user_message(&mut session, &reminder);
        push_user_message(&mut session, "identical old conversation");
        session
    };
    let build_pending =
        || -> Arc<Mutex<Option<String>>> { Arc::new(Mutex::new(Some(wrap_dag_content(summary)))) };

    // @step When the pending DAG is applied and emitted through the agent-loop twin and through the NAPI twin
    let mut results: Vec<(&'static str, codelet_napi::CompactionResult)> = Vec::new();
    for (twin, apply_and_emit) in twins() {
        let mut session = build_session();
        let (chunks, emit) = recorder();
        let applied = apply_and_emit(&mut session, &build_pending(), original_tokens, &emit);
        assert!(applied.is_some(), "[{twin}] twin must apply");
        let chunks = chunks.lock().unwrap();
        let result = find_compaction_result(&chunks).expect("twin must emit CompactionComplete");
        results.push((twin, result));
    }

    // @step Then both twins emit CompactionComplete with identical original_tokens, compacted_tokens, and compression_ratio
    let (_, agent_loop_result) = &results[0];
    let (_, napi_result) = &results[1];
    assert_eq!(
        agent_loop_result.original_tokens, napi_result.original_tokens,
        "original_tokens must be identical across twins"
    );
    assert_eq!(
        agent_loop_result.compacted_tokens, napi_result.compacted_tokens,
        "compacted_tokens must be identical across twins"
    );
    assert_eq!(
        agent_loop_result.compression_ratio, napi_result.compression_ratio,
        "compression_ratio must be identical across twins"
    );
}
