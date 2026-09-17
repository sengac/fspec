#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::await_holding_lock)]
//! Feature: spec/features/terminal-overflow-recovery-for-background-sessions.feature
//!
//! CMPCT-044: the agent-loop TERMINAL-ERROR arm routes a context-overflow
//! error that the in-loop cascade could not resolve through the SAME
//! compactor recovery entry point the stream loop uses — the session is
//! reduced to reminders + a DAG, returns to Idle, and the oversized
//! payload is never replayed.
//!
//! These tests drive the real
//! `codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery`
//! against a real `BackgroundSession` (SessionManager fixture + offline
//! model cache, the `cmpct045` pattern) with a fake compactor handler
//! (the spawner is covered by the stream-loop and cmpct045 suites).

use std::sync::Arc;

use codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::session_manager::SessionManager;
use codelet_sessions::background_session::BackgroundSession;
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use serial_test::serial;
use uuid::Uuid;

const SUB_AGENT_DAG: &str = r#"<dag-node depth="D1" turns="0-4" label="Terminal arc">
- the session survived the terminal overflow
</dag-node>"#;

static GLOBAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A hermetic SessionManager + one live session under a fresh data
/// directory (the `cmpct045` fixture pattern — offline model cache).
async fn fixture_session() -> (tempfile::TempDir, String, Arc<SessionManager>, Arc<BackgroundSession>) {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");

    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(
        cache_dir.join("models.json"),
        include_str!("fixtures/prov101_models.json"),
    )
    .expect("write models fixture");
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data dir");
    codelet_core::persistence::reset_stores_for_tests();

    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("openai/o3");

    let project = tempfile::tempdir()
        .expect("project tempdir")
        .keep()
        .to_str()
        .expect("utf8")
        .to_string();

    let session_id = Uuid::new_v4();
    manager
        .create_session_with_id(&session_id.to_string(), "openai/o3", &project, "cmpct044-terminal")
        .await
        .expect("create session");
    let session = manager
        .get_session(&session_id.to_string())
        .expect("session live");
    (data_dir, project, manager, session)
}

/// Seed `n` compactable turns into the session's inner.
async fn seed_turns(session: &BackgroundSession, n: usize) {
    let mut inner = session.inner.lock().await;
    for i in 0..n {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!(
                "user turn {i} do work"
            ))),
        });
        inner.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(
                rig::message::Text {
                    text: format!("assistant turn {i} did work"),
                },
            )),
        });
        inner.turns.push(codelet_core::compaction::ConversationTurn {
            user_message: format!("user turn {i}"),
            tool_calls: vec![],
            tool_results: vec![],
            assistant_response: format!("assistant turn {i}"),
            tokens: 500,
            timestamp: std::time::SystemTime::now(),
            previous_error: None,
        });
    }
    inner.token_tracker.input_tokens = 190_000;
}

/// The last message text of a session's inner.
async fn last_message_text(session: &BackgroundSession) -> String {
    let inner = session.inner.lock().await;
    let last = inner.messages.last().expect("a message must remain");
    match last {
        Message::User { content } => content
            .iter()
            .find_map(|c| match c {
                UserContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        _ => panic!("the pinned message must be a User message"),
    }
}

/// Scenario: Background session terminal overflow error compacts via the same entry point
#[serial]
#[tokio::test]
async fn background_terminal_overflow_compacts_via_the_same_entry_point() {
    let _guard = GLOBAL_GUARD.lock().expect("test guard");

    // @step Given a background agent-loop session whose turn dies with a context-overflow error that no classifier matched
    let (_data_dir, _project, _manager, session) = fixture_session().await;
    seed_turns(&session, 5).await;

    // The fake handler stands in for the compactor sub-agent spawner —
    // it returns a parseable DAG, proving the terminal path routes to
    // the SAME `run_compactor_sub_agent_round` entry point the stream
    // loop uses.
    set_compactor_sub_agent_handler(
        session.id,
        Some(Arc::new(|_parent, _existing| {
            Box::pin(async { Ok(SUB_AGENT_DAG.to_string()) })
        })),
    );

    // Provider-variant wording the legacy classifier misses: the 400
    // body matches NO `is_prompt_too_long_error` substring, only the
    // robust `is_context_overflow_error`.
    let overflow_error = anyhow::anyhow!(
        "API error: 400 Input is too long: 201,000 tokens > 200,000 maximum"
    );

    // @step When the agent-loop terminal-error arm processes the error
    let recovered =
        codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
            session.clone(),
            &overflow_error,
        )
        .await;

    // @step Then the compactor recovery entry point runs for that session
    assert!(
        recovered,
        "the terminal overflow must route to the compactor recovery entry point"
    );

    // @step And the session's context is cleared to reminders plus a DAG
    let pinned = last_message_text(&session).await;
    assert!(
        pinned.contains("<!-- type:compaction-dag -->"),
        "the pinned message must carry the compaction-dag wrapper, got: {pinned}"
    );
    assert!(
        pinned.contains("Terminal arc"),
        "the sub-agent's DAG must be pinned, got: {pinned}"
    );
    let tracker = session.inner.lock().await.token_tracker.input_tokens;
    assert!(
        tracker < 190_000,
        "the session must be reduced, got {tracker}"
    );

    // @step And the session returns to Idle
    assert_eq!(
        session.get_status(),
        SessionStatus::Idle,
        "the session must return to Idle after the terminal recovery"
    );

    // @step And the next user message starts a fresh stream on the reduced context
    // The CompactionComplete event carries the honest post-pin basis —
    // the next stream is built from the reduced in-memory context
    // (reminders + DAG), never the oversized payload.
    let chunks = session.get_buffered_output(1000);
    let complete = chunks
        .into_iter()
        .find(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
        .expect("a CompactionComplete event must be emitted on the terminal path");
    match complete {
        StreamChunk::CompactionComplete {
            compaction_result: result,
        } => {
            assert!(
                result.original_tokens >= 190_000,
                "the pre-compaction basis must be the tracker total ({})",
                result.original_tokens
            );
            assert!(
                result.compacted_tokens < result.original_tokens,
                "the post-pin basis must reflect the reduced context"
            );
            // RPC-420: compression_ratio is PERCENT removed, in [0, 100].
            assert!(
                (0.0..=100.0).contains(&result.compression_ratio),
                "the ratio must be a percent removed, got {}",
                result.compression_ratio
            );
        }
        other => panic!("expected CompactionComplete, got {other:?}"),
    }

    set_compactor_sub_agent_handler(session.id, None);
    drop(_guard);
}

/// Scenario: Background session terminal overflow error — no compactable turns
/// (the terminal path must NOT compact; the error follows the existing
/// terminal handling).
#[serial]
#[tokio::test]
async fn terminal_overflow_without_compactable_turns_does_not_compact() {
    let _guard = GLOBAL_GUARD.lock().expect("test guard");

    // @step Given a streaming session with only system-reminder messages and no user/assistant turns
    let (_data_dir, _project, _manager, session) = fixture_session().await;
    // Only system-reminder-style messages — no user/assistant turns.
    {
        let mut inner = session.inner.lock().await;
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(
                "<system-reminder>ambient context</system-reminder>",
            )),
        });
    }

    // @step And the provider returns a context-overflow error
    let overflow_error =
        anyhow::anyhow!("Input is too long: 201,000 tokens > 200,000 maximum");

    // @step When the stream loop processes the error
    let recovered =
        codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
            session.clone(),
            &overflow_error,
        )
        .await;

    // @step Then no compaction is triggered because there are no compactable turns
    // No compaction ran — the gate (PROV-010) held.
    assert!(
        !recovered,
        "overflow without compactable turns must NOT trigger compaction"
    );
    let tracker = session.inner.lock().await.token_tracker.input_tokens;
    assert!(
        tracker == 0,
        "the untouched session's tracker must be unchanged (got {tracker})"
    );
    // @step And the error follows the existing terminal error handling
    assert_eq!(
        session.get_status(),
        SessionStatus::Idle,
        "the session stays Idle — the existing terminal handling applies"
    );

    drop(_guard);
}

/// Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
#[test]
fn compactor_handler_registration_lifecycle_mirrors_deep_search() {
    // @step Given a background session is created in the agent loop
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/agent_loop.rs");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let find = |needle: &str| -> usize {
        src.find(needle)
            .unwrap_or_else(|| panic!("CMPCT-044: '{needle}' not found in agent_loop.rs"))
    };

    // @step When the session's handlers are registered
    // @step Then a compactor-sub-agent handler is registered for the session id capturing the provider, model, and project path
    // Registered in the session-creation handler block, beside the
    // DeepSearch registration — the same capture pattern (provider,
    // model, context window, project path, compaction flag).
    let registration_at = find("register_compactor_sub_agent_handler(");
    let deep_search_at = find("register_deep_search_handler(");
    assert!(
        registration_at > deep_search_at && (registration_at - deep_search_at) < 3000,
        "the compactor sub-agent registration (char {registration_at}) must sit in the \
         session-creation handler block, beside the DeepSearch registration \
         (char {deep_search_at})"
    );
    // The registration reads the provider/model from the live inner at
    // call time (re-registration after /model /provider changes).
    let call_window = &src[registration_at..(registration_at + 300)];
    assert!(
        call_window.contains("&inner_session"),
        "the compactor sub-agent registration must read provider/model from the \
         session's live inner at call time (BUG-132 pattern)"
    );

    // @step And the handler is removed for the session id during end-of-turn cleanup
    let cleanup_at = find("set_compactor_sub_agent_handler(session.id, None)");
    let deep_search_cleanup_at = find("set_deep_search_handler(session.id, None)");
    assert!(
        cleanup_at > deep_search_cleanup_at && (cleanup_at - deep_search_cleanup_at) < 500,
        "the compactor sub-agent cleanup (char {cleanup_at}) must sit in the \
         end-of-turn cleanup block, beside the DeepSearch cleanup \
         (char {deep_search_cleanup_at})"
    );
}

/// Scenario: Compactor sub-agent has only the read-only tool surface
#[test]
fn compactor_sub_agent_has_only_the_read_only_tool_surface() {
    // @step Given a compactor sub-agent is constructed
    // @step When its tool list is built
    // @step Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    assert_eq!(
        codelet_tools::SUB_AGENT_TOOL_COUNT, 7,
        "the compactor sub-agent must keep exactly the 7 read-only DeepSearch tools"
    );
    assert_eq!(
        codelet_tools::SUB_AGENT_TOOL_NAMES,
        ["Read", "Grep", "AstGrep", "Glob", "Ls", "Bash", "SessionSearch"],
        "the compactor sub-agent's tool surface must be exactly the 7 \
         read-only tools"
    );

    // @step And the sub-agent does NOT have the inject_summary tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES
            .iter()
            .any(|t| t.to_lowercase().contains("inject_summary")),
        "the compactor sub-agent must NOT have inject_summary (the pin is handler-side)"
    );

    // @step And the sub-agent does NOT have any Write or Edit tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES
            .iter()
            .any(|t| *t == "Write" || *t == "Edit"),
        "the compactor sub-agent must NOT have Write or Edit tools"
    );
}

/// A non-overflow terminal error is NOT routed to compaction.
#[serial]
#[tokio::test]
async fn terminal_non_overflow_error_does_not_compact() {
    let _guard = GLOBAL_GUARD.lock().expect("test guard");

    let (_data_dir, _project, _manager, session) = fixture_session().await;
    seed_turns(&session, 3).await;

    let auth_error = anyhow::anyhow!("401 Invalid API key provided: sk-abc123");
    let recovered =
        codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
            session.clone(),
            &auth_error,
        )
        .await;

    assert!(
        !recovered,
        "a non-overflow terminal error must follow the existing terminal handling"
    );
    let messages = session.inner.lock().await.messages.len();
    assert!(
        messages >= 6,
        "the session's context must be untouched (had {messages} messages)"
    );

    drop(_guard);
}

/// Scenario: Compaction instruction is FRESH when the parent has no DAG
#[test]
fn compaction_instruction_is_fresh_when_the_parent_has_no_dag() {
    // @step Given a parent session that contains no existing compaction DAG
    let parent = Uuid::new_v4();

    // @step When the compactor sub-agent's task prompt is built
    let prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(parent, None);

    // @step Then the FRESH compaction instruction is used
    assert!(
        prompt.contains("Build a hierarchical summary DAG of your session"),
        "the FRESH instruction must be selected when the parent has no DAG"
    );

    // @step And the task prompt names the parent session id for every SessionSearch call
    assert!(
        prompt.contains(&format!("SessionSearch(session_id: \"{parent}\", show, ")),
        "every SessionSearch show call must name the parent session id"
    );
    assert!(
        prompt.contains(&format!("SessionSearch(session_id: \"{parent}\", search, ")),
        "every SessionSearch search call must name the parent session id"
    );

    // @step And the task prompt instructs the sub-agent to output the complete DAG as its final response instead of calling inject_summary
    assert!(
        prompt.contains("Output the complete DAG"),
        "the prompt must instruct the sub-agent to output the DAG as its final response"
    );
    assert!(
        !prompt.contains("inject_summary"),
        "the sub-agent has no inject_summary tool — the prompt must not mention it"
    );
}

/// Scenario: Compaction instruction is INCREMENTAL when the parent already has a DAG
#[test]
fn compaction_instruction_is_incremental_when_the_parent_has_a_dag() {
    // @step Given a parent session whose context already contains a compaction DAG ending at turn N
    let parent = Uuid::new_v4();
    let existing_dag = r#"<system-reminder>
<!-- type:compaction-dag -->
<dag-node depth="D2" turns="0-45" label="Architecture">settled decisions</dag-node>
</system-reminder>"#;

    // @step When the compactor sub-agent's task prompt is built
    let prompt =
        codelet_cli::compaction_dag::build_generate_compaction_prompt(parent, Some((existing_dag.to_string(), 45)));

    // @step Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    assert!(
        prompt.contains("do NOT rebuild from scratch"),
        "the INCREMENTAL instruction must be selected when the parent has a DAG"
    );
    assert!(
        prompt.contains(existing_dag),
        "the existing DAG must be embedded in the prompt"
    );

    // @step And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward
    assert!(
        prompt.contains("PRESERVE all existing D2"),
        "the prompt must instruct preserving D2 nodes"
    );
    assert!(
        prompt.contains("PROMOTE existing D0 (Detailed) nodes to D1"),
        "the prompt must instruct promoting D0 to D1"
    );
    assert!(
        prompt.contains("start_turn: 46"),
        "the prompt must survey from turn 46 (N+1, where N = max_turn_end 45)"
    );
}
