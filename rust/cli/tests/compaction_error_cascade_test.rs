#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/complete-error-cascade-in-run-retry-stream.feature
//!
//! CMPCT-027: Complete error cascade in run_retry_stream.
//!
//! These tests cover the in-loop compaction restart that replaces the separate
//! `run_retry_stream` module. The stream loop now cascades ALL post-compaction
//! errors through the primary error classifier instead of a weaker retry-only
//! handler, so the behavior matrix collapses from two copies to one.
//!
//! The tests exercise the unit-level pieces: the circuit-breaker constant, the
//! budget-exhausted message formatter, and the `execute_compaction_and_capture_events`
//! helper (which encapsulates the step the old `handle_compaction_retry`
//! performed before spinning up a separate retry stream). Two structural
//! checks guarantee that the obsolete `run_retry_stream` is gone and that the
//! primary loop wires in the bounded retry counter.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, OnceLock};

use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive::{
    build_compaction_budget_exhausted_message, execute_compaction_and_capture_events,
    MAX_COMPACTION_RETRIES,
};
use codelet_cli::session::Session;
use codelet_core::TokenState;

/// Process-wide tempdir kept alive for the lifetime of the test binary. The
/// `OnceLock` owns the `TempDir`, so it lives until process exit and is
/// cleaned up by the OS — no `unsafe` required.
static TEST_DATA_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Ensure `codelet_common`'s data directory is initialized to a unique temp
/// directory for this test binary.
///
/// The `execute_compaction_and_capture_events` helper eventually calls
/// `get_debug_capture_manager()`, whose global `OnceLock` initializer requires
/// `set_data_directory(..)` to have been called first (production does this
/// in `cli/src/main.rs`). Tests that exercise that helper must call
/// `ensure_test_data_dir()` as their first line.
fn ensure_test_data_dir() {
    TEST_DATA_DIR.get_or_init(|| {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        codelet_common::set_data_directory(temp_dir.path().to_path_buf())
            .expect("Failed to set data directory");
        temp_dir
    });
}

/// Fake StreamOutput that records every emitted event.
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn count<F: Fn(&StreamEvent) -> bool>(&self, pred: F) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| pred(e))
            .count()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn fresh_session() -> Session {
    Session::new(None).expect("failed to create test session")
}

fn fresh_token_state() -> Arc<Mutex<TokenState>> {
    Arc::new(Mutex::new(TokenState {
        input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: false,
    }))
}

// ============================================================================
// Scenario: MAX_COMPACTION_RETRIES constant is exposed as 3
// ============================================================================

#[test]
fn max_compaction_retries_constant_is_three() {
    // @step Given the stream loop provides a bounded retry budget for cascaded compaction attempts
    // (constant is defined in recovery_compaction module and re-exported via interactive::mod)

    // @step When a test reads the MAX_COMPACTION_RETRIES constant from the interactive module
    let value = MAX_COMPACTION_RETRIES;

    // @step Then its value is exactly 3
    assert_eq!(
        value, 3,
        "MAX_COMPACTION_RETRIES must be 3 per the CMPCT-027 plan"
    );
}

// ============================================================================
// Scenario: Budget exhausted message mentions the attempt count
// ============================================================================

#[test]
fn budget_exhausted_message_mentions_attempt_count_and_compaction() {
    // @step Given a caller exhausted the compaction retry budget
    let attempts = MAX_COMPACTION_RETRIES;

    // @step When build_compaction_budget_exhausted_message is invoked with the attempt count
    let msg = build_compaction_budget_exhausted_message(attempts);

    // @step Then the returned message contains the numeric attempt count and the word compaction
    assert!(
        msg.contains("3"),
        "message must mention the attempt count, got: {msg:?}"
    );
    assert!(
        msg.to_lowercase().contains("compaction"),
        "message must mention 'compaction', got: {msg:?}"
    );
}

// ============================================================================
// Scenario: execute_compaction_and_capture_events runs compaction and resets
// token tracker
// ============================================================================

#[tokio::test]
async fn execute_compaction_helper_clears_prior_turns_and_resets_tracker() {
    ensure_test_data_dir();

    // @step Given a session with a populated conversation and a previously triggered compaction token state
    let mut session = fresh_session();

    // Seed a realistic user + assistant pair so execute_compaction has
    // something to condense (convert_messages_to_turns needs pairs).
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text("tell me about rust")),
    });
    session.messages.push(rig::message::Message::Assistant {
        id: None,
        content: rig::OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
            text: "Rust is a systems language.".to_string(),
        })),
    });

    // Seed the tracker with non-trivial values so we can observe the reset.
    session.token_tracker.input_tokens = 50_000;
    session.token_tracker.output_tokens = 8_000;

    let token_state = fresh_token_state();
    if let Ok(mut state) = token_state.lock() {
        state.compaction_needed = true;
        state.input_tokens = 50_000;
    }

    let compaction_flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();

    let threshold: u64 = 100_000;
    let context_window: u64 = 200_000;

    // @step When execute_compaction_and_capture_events is invoked with the original user prompt
    execute_compaction_and_capture_events(
        &mut session,
        compaction_flag.clone(),
        "tell me about rust",
        threshold,
        context_window,
        &token_state,
        &output,
    )
    .await
    .expect("helper must succeed for a compactable session");

    // @step Then execute_compaction has cleared the prior turns and injected the compaction instruction
    // execute_compaction clears compactable turns (user/assistant pairs) and
    // pushes a single User message carrying the compaction instruction. The
    // instruction embeds the original prompt we passed in so the agent knows
    // what to resume after building the DAG.
    let last = session
        .messages
        .last()
        .expect("session must have at least the compaction instruction");
    match last {
        rig::message::Message::User { content, .. } => {
            let text = content
                .iter()
                .find_map(|c| match c {
                    rig::message::UserContent::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            assert!(
                text.contains("tell me about rust"),
                "compaction instruction must embed the original prompt, got: {text:?}"
            );
        }
        other => {
            panic!("expected final message to be User (compaction instruction), got {other:?}")
        }
    }

    // The original user/assistant pair must no longer be present.
    let has_original_assistant = session.messages.iter().any(|m| matches!(
        m,
        rig::message::Message::Assistant { content, .. }
            if content.iter().any(|c| matches!(c, rig::message::AssistantContent::Text(t) if t.text.contains("Rust is a systems language")))
    ));
    assert!(
        !has_original_assistant,
        "the original assistant turn must have been cleared by execute_compaction"
    );

    // @step Then the session token tracker has been reset via reset_after_compaction
    // reset_after_compaction zeroes out the last-turn billing but preserves
    // cumulative billing. The critical observable here is the display-level
    // input_tokens: after compaction + reset it reflects the recalculated
    // in-view-DAG context, which is much smaller than the seeded 50_000.
    assert!(
        session.token_tracker.input_tokens < 50_000,
        "token_tracker.input_tokens should have been recalculated by execute_compaction; got {}",
        session.token_tracker.input_tokens
    );
    // The compaction_in_progress flag must have been set by execute_compaction.
    assert!(
        compaction_flag.load(std::sync::atomic::Ordering::SeqCst),
        "compaction_in_progress flag must be set after execute_compaction"
    );
}

// ============================================================================
// Scenario: execute_compaction_and_capture_events emits compaction continuing
// event on success
// ============================================================================

#[tokio::test]
async fn execute_compaction_helper_emits_compaction_continuing_once() {
    ensure_test_data_dir();

    // @step Given a session that will successfully run compaction
    let mut session = fresh_session();
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text("do some work")),
    });
    session.messages.push(rig::message::Message::Assistant {
        id: None,
        content: rig::OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
            text: "ok".to_string(),
        })),
    });

    let token_state = fresh_token_state();
    let compaction_flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();

    // @step When execute_compaction_and_capture_events is invoked with a recording StreamOutput
    execute_compaction_and_capture_events(
        &mut session,
        compaction_flag,
        "do some work",
        50_000,
        200_000,
        &token_state,
        &output,
    )
    .await
    .expect("helper must succeed");

    // @step Then exactly one CompactionContinuing event is emitted
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionContinuing)),
        1,
        "CompactionContinuing should be emitted exactly once per successful compaction"
    );

    // @step Then no CompactionStarted or CompactionProgress events are emitted from the helper
    // These are emitted by `begin_compaction_recovery` upstream — emitting
    // them again here would double-count the lifecycle events (the same
    // double-emit bug that CMPCT-023 cleaned up).
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        0,
        "CompactionStarted must NOT be re-emitted by execute_compaction_and_capture_events"
    );
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionProgress(_))),
        0,
        "CompactionProgress must NOT be re-emitted by execute_compaction_and_capture_events"
    );
}

// ============================================================================
// Scenario: The obsolete run_retry_stream function has been removed from the
// compaction retry module
// ============================================================================

#[test]
fn run_retry_stream_is_removed_from_cli_crate() {
    // @step Given the in-loop restart replaces the separate post-loop retry stream
    // @step When the codebase is searched for a fn run_retry_stream definition
    // @step Then no such definition exists in the cli crate sources
    use std::path::PathBuf;

    let cli_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive");
    let retry_module = cli_src.join("compaction_retry.rs");

    // The module may be deleted outright OR kept as a demoted helper with no
    // `fn run_retry_stream`. Both shapes satisfy CMPCT-027; we only check the
    // obsolete function signature is absent everywhere.
    let sources = collect_rust_sources(&cli_src);
    let mut offenders: Vec<String> = Vec::new();
    for path in sources {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if content.contains("fn run_retry_stream") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "run_retry_stream must be removed (found in: {offenders:?})"
    );

    // compaction_retry.rs must no longer exist OR must not define
    // handle_compaction_retry (the container for run_retry_stream).
    if retry_module.exists() {
        let content = std::fs::read_to_string(&retry_module).unwrap_or_default();
        assert!(
            !content.contains("fn handle_compaction_retry"),
            "compaction_retry.rs still defines handle_compaction_retry — the in-loop restart should supersede it"
        );
    }
}

fn collect_rust_sources(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(collect_rust_sources(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    out
}

// ============================================================================
// Scenario: Stream loop wires in a compaction retry counter bounded by
// MAX_COMPACTION_RETRIES
// ============================================================================

#[test]
fn stream_loop_declares_compaction_retry_counter_and_checks_budget() {
    // @step Given the stream loop must bound cascaded compaction attempts
    let stream_loop =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive/stream_loop.rs");

    // @step When the stream_loop.rs source is inspected
    let content = std::fs::read_to_string(&stream_loop).expect("stream_loop.rs must exist");

    // @step Then it declares a mutable compaction_retry_count counter and checks it against MAX_COMPACTION_RETRIES before each cascaded compaction
    assert!(
        content.contains("compaction_retry_count"),
        "stream_loop.rs must declare a compaction_retry_count counter"
    );
    assert!(
        content.contains("MAX_COMPACTION_RETRIES"),
        "stream_loop.rs must reference MAX_COMPACTION_RETRIES for the circuit breaker"
    );

    // The in-loop restart path must replace the old post-loop handler. The
    // dead-code `handle_compaction_retry` call that lived at stream_loop.rs:1547
    // must no longer be reachable from stream_loop.rs.
    assert!(
        !content.contains("handle_compaction_retry("),
        "stream_loop.rs must not call handle_compaction_retry — the in-loop restart supersedes it"
    );
}
