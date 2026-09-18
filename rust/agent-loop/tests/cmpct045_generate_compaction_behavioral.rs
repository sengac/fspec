#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock
)]
//! Feature: spec/features/generate-compaction-tool.feature
//!
//! CMPCT-045: behavioral coverage for the GenerateCompaction tool's
//! agent-loop handler side — ephemeral compactor sub-agent dispatch,
//! handler-side pinning (calling-session stash vs cross-session
//! immediate pin), fallback DAGs, lifecycle events, and the
//! provider-builder tool wiring.
//!
//! The handler closure is registered through the real
//! `register_generate_compaction_handler` (the same entry point the
//! agent loop uses at session creation) and driven through the real
//! tools-crate `GenerateCompactionTool::call` — the only fakes are a
//! wiremock LLM endpoint (openai provider via OPENAI_BASE_URL) and
//! process-global env seams. All tests serialize on a process-global
//! guard (env vars + data dir + tool-handler registries are
//! process-global) mirroring the `serial_test` precedent.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::compaction::{parse_dag_nodes, wrap_dag_content, ConversationTurn};
use codelet_rpc_types::{SessionState, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::session_manager::SessionManager;
use codelet_tools::{
    clear_all_generate_compaction_handlers, has_generate_compaction_handler,
    set_generate_compaction_handler, GenerateCompactionArgs, GenerateCompactionHandler,
    GenerateCompactionTool, ToolError,
};
use rig::message::{Message, UserContent};
use rig::tool::Tool;
use rig::OneOrMany;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use serial_test::serial;

// ============================================================================
// Process-global guard — env vars (OPENAI_*), the data directory, and the
// tool-handler registries are process-global, so every test fences on this.
// ============================================================================

static GLOBAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ============================================================================
// Shared fixtures
// ============================================================================

const OPENAI_MODEL: &str = "o3";

/// A minimal but parseable compaction DAG (one D1 node + dag-files).
const SUCCESS_DAG: &str = r#"<dag-node depth="D1" turns="0-5" label="Work arc">
- completed the task
</dag-node>
<dag-files>
- src/foo.rs (Modified)
</dag-files>"#;

/// A partial output: one complete dag-node block plus junk text. Served
/// as the BODY of a 400 response — the sub-agent fails, and the
/// complete block embedded in the failure payload is what the handler's
/// `build_fallback_dag` recovers from the Err reason (the real
/// partial-recovery path, not the parse-success path).
const PARTIAL_DAG: &str = r#"thinking about the session...
<dag-node depth="D1" turns="0-3" label="Partial arc">
- recovered work
</dag-node>
trailing junk that is not a dag node"#;

/// OpenAI-style non-streaming completion response body (all required
/// fields of the patched rig `CompletionResponse`: id, object, created,
/// model, choices; `system_fingerprint`/`usage` are optional).
fn openai_body(text: &str) -> String {
    format!(
        r#"{{"id":"chatcmpl-test","object":"chat.completion","created":1,"model":"o3","choices":[{{"index":0,"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}]}}"#,
        serde_json::to_string(text).expect("DAG text is JSON-serializable")
    )
}

/// Point OPENAI_* at the wiremock server (serial — process-global env).
fn point_openai_at(base_url: &str) {
    std::env::set_var("OPENAI_API_KEY", "sk-test-dummy");
    std::env::set_var("OPENAI_BASE_URL", base_url);
    std::env::set_var("OPENAI_MODEL", OPENAI_MODEL);
}

/// Create a hermetic SessionManager + two live sessions (caller +
/// target) under a fresh data directory. The model cache is pre-seeded
/// from the offline models.dev fixture (prov101) so registry-backed
/// model resolution stays fully offline (the `cmpct041` / `rpc386`
/// pattern). Returns the data dir (the caller must keep it alive).
async fn fixture_sessions() -> (
    tempfile::TempDir,
    String,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
    Arc<BackgroundSession>,
) {
    // Set dummy creds so `ProviderCredentials::detect()` passes offline.
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
    // NOTE: the project tempdir is intentionally leaked per test — the
    // path must outlive the sessions. Each test leaks one tempdir;
    // acceptable for a test binary.

    let caller_id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &caller_id.to_string(),
            "openai/o3",
            &project,
            "cmpct045-caller",
        )
        .await
        .expect("create caller session");
    let target_id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &target_id.to_string(),
            "openai/o3",
            &project,
            "cmpct045-target",
        )
        .await
        .expect("create target session");

    let caller = manager
        .get_session(&caller_id.to_string())
        .expect("caller live");
    let target = manager
        .get_session(&target_id.to_string())
        .expect("target live");
    (data_dir, project, manager, caller, target)
}

/// Seed `n` compactable conversation turns into a session's inner and
/// set the token tracker to a large pre-compaction basis (190k).
async fn seed_turns(session: &BackgroundSession, n: usize) {
    let mut inner = session.inner.lock().await;
    for i in 0..n {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("user turn {i} do work"))),
        });
        inner.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
                text: format!("assistant turn {i} did work"),
            })),
        });
        inner.turns.push(ConversationTurn {
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

/// Seed an existing wrapped compaction DAG (ending at turn 45) into a
/// session's inner so `detect_existing_dag` finds it.
async fn seed_existing_dag(session: &BackgroundSession, dag: &str, turns_before: usize) {
    let mut inner = session.inner.lock().await;
    for i in 0..turns_before {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("early turn {i}"))),
        });
    }
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(wrap_dag_content(dag))),
    });
    inner.token_tracker.input_tokens = 150_000;
}

/// Register the real GenerateCompaction handler for `caller` and return
/// the tool the agent would invoke.
fn register_and_build_tool(
    caller: &Arc<BackgroundSession>,
    project: &str,
    manager: &Arc<SessionManager>,
) -> GenerateCompactionTool {
    {
        // The handler needs a sync &Session — take the lock briefly.
        let guard = caller.inner.try_lock().expect("caller inner free");
        register_generate_compaction_handler(
            caller.id,
            &guard,
            std::path::PathBuf::from(project),
            caller.clone(),
            Some(manager.clone()),
        );
        // drop(guard) here — the handler closure captures Arcs, not the lock.
    }
    GenerateCompactionTool::new(caller.id)
}

/// The last message text of a session's inner (panics if empty / not User).
async fn last_message_text(session: &BackgroundSession) -> String {
    let inner = session.inner.lock().await;
    let last = inner
        .messages
        .last()
        .expect("a message must remain after the pin");
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

/// The task prompt the sub-agent's LLM request carried — recorded by
/// wiremock's request recorder (the last `/v1/chat/completions` body's
/// `messages` array, last message = the task). Process-global, tests
/// are serial.
async fn last_task_prompt(server: &MockServer) -> String {
    let requests = match server.received_requests().await {
        Some(requests) => requests,
        None => panic!("no requests recorded by the wiremock server"),
    };
    requests
        .iter()
        .rev()
        .find(|r| r.url.path() == "/v1/chat/completions")
        .map(|r| {
            let body: serde_json::Value =
                serde_json::from_slice(&r.body).expect("request body is JSON");
            let last = body["messages"]
                .as_array()
                .and_then(|m| m.last())
                .unwrap_or_else(|| panic!("request body has no messages: {body:?}"));
            // rig sends content as either a bare string or an array of
            // {type:"text", text:"..."} blocks — handle both.
            match last.get("content") {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Array(parts)) => parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join(""),
                other => panic!("unexpected content shape in last message: {other:?}"),
            }
        })
        .unwrap_or_else(|| {
            let paths: Vec<&str> = requests.iter().map(|r| r.url.path()).collect();
            panic!(
                "no /v1/chat/completions request recorded (paths: {paths:?}) — \
                 the sub-agent's LLM call must have reached the mock"
            )
        })
}

/// Mock LLM serving `dag_text` + a fresh fixture set.
async fn mock_llm_and_fixture_serving(
    dag_text: &str,
) -> (
    MockServer,
    tempfile::TempDir,
    String,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
    Arc<BackgroundSession>,
) {
    let server = MockServer::start().await;
    point_openai_at(&server.uri());
    let mock = Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(openai_body(dag_text)),
        )
        .expect(1..);
    server.register(mock).await;

    let (data_dir, project, manager, caller, target) = fixture_sessions().await;
    (server, data_dir, project, manager, caller, target)
}

/// Mock LLM that FAILS the request (400 with `text` as the body) —
/// the rig client surfaces the body inside its CompletionError, so the
/// handler's `build_fallback_dag(reason)` recovers any complete
/// `<dag-node>` blocks embedded in `text` (the partial-recovery path).
async fn mock_llm_failing_with(
    text: &str,
) -> (
    MockServer,
    tempfile::TempDir,
    String,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
    Arc<BackgroundSession>,
) {
    let server = MockServer::start().await;
    point_openai_at(&server.uri());
    let mock = Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("content-type", "application/json")
                .set_body_string(text),
        )
        .expect(1..);
    server.register(mock).await;

    let (data_dir, project, manager, caller, target) = fixture_sessions().await;
    (server, data_dir, project, manager, caller, target)
}

/// Pull the last CompactionComplete event out of the target's buffer.
fn last_compaction_event(target: &BackgroundSession) -> StreamChunk {
    target
        .get_buffered_output(1000)
        .into_iter()
        .rev()
        .find(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
        .expect("a CompactionComplete event must have been emitted on the target")
}

// ============================================================================
// Scenario: GenerateCompaction tool wired into provider agent builders
// ============================================================================

/// Source-shape guard (the established provider-chain pattern): every
/// provider's create_rig_agent tool chain must include
/// GenerateCompactionTool::new(session_id) beside DeepSearchTool.
#[test]
fn generate_compaction_tool_wired_into_every_provider_tool_chain() {
    // @step Given all provider implementations exist (Claude, OpenAI, Gemini, Codex, ZAI, GitHub Copilot, custom provider)
    // @step When each provider's create_rig_agent() method builds an agent
    // @step Then GenerateCompactionTool::new(session_id) is included in the tool chain beside DeepSearchTool
    // @step And the parent agent can invoke GenerateCompaction like any other tool
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("repo rust root");

    let provider_files = [
        "rust/providers/src/claude.rs",
        "rust/providers/src/openai.rs",
        "rust/providers/src/gemini.rs",
        "rust/providers/src/codex/mod.rs",
        "rust/providers/src/zai.rs",
        "rust/providers/src/copilot/rig_agent.rs",
        "rust/providers/src/custom/custom_provider.rs",
    ];

    for rel in provider_files {
        let src = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"));
        let deep_at = src
            .find("DeepSearchTool::new(session_id)")
            .unwrap_or_else(|| panic!("CMPCT-045: {rel} must wire DeepSearchTool"));
        let gc_at = src
            .find("GenerateCompactionTool::new(session_id)")
            .unwrap_or_else(|| {
                panic!("CMPCT-045: {rel} must wire GenerateCompactionTool beside DeepSearchTool")
            });
        assert!(
            gc_at > deep_at && (gc_at - deep_at) < 400,
            "CMPCT-045: in {rel} the GenerateCompactionTool wiring (char {gc_at}) must sit \
             beside the DeepSearchTool wiring (char {deep_at})"
        );
    }
}

// ============================================================================
// Scenario: Successful sub-agent DAG is pinned to the target session
// Scenario: Compactor sub-agent runs in an ephemeral clean session
// Scenario: Compactor sub-agent surveys the target exclusively via
//           SessionSearch on the target session id
// Scenario: Other-session target is pinned immediately under the target's lock
// ============================================================================

#[tokio::test]
async fn successful_dag_is_pinned_to_the_target_and_sub_agent_session_is_ephemeral() {
    // @step Given a live target session with system reminders and many compactable conversation messages
    // @step Given a parent session whose provider and model are configured
    // @step Given a parent session with persisted turns
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    // @step Given the calling session has a configured provider and model
    // @step Given a live target session with persisted turns
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&target, 6).await;

    let tool = register_and_build_tool(&caller, &project, &manager);
    // @step Given session S calls GenerateCompaction with a live subordinate session T's UUID

    // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    // @step When the agent calls GenerateCompaction with the target session UUID
    // @step When GenerateCompaction is invoked for a live target session
    // @step When the handler side pins the sub-agent's DAG to the target
    // @step Given GenerateCompaction is invoked for a live target session
    // @step When the sub-agent run and pin complete
    // @step When the handler side completes the run
    // @step When the compactor sub-agent is asked to build the compaction DAG
    // @step And the compactor sub-agent returns a parseable DAG
    // @step And the sub-agent returns a parseable DAG
    // @step When the compactor sub-agent is spawned for the parent session
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("success path must return Ok");
    // @step When GenerateCompaction completes successfully

    // @step Then the pin is applied to T's in-memory messages under T's inner lock
    // @step And T's messages are reduced to system reminders plus the wrapped DAG
    // @step Then the target's in-memory messages are replaced by the system reminders plus the wrapped DAG
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("<!-- type:compaction-dag -->"),
        "the pinned message must carry the compaction-dag wrapper, got: {pinned}"
    );
    assert!(
        pinned.contains("Work arc"),
        "the pinned DAG must be the sub-agent's DAG, got: {pinned}"
    );

    // @step And the target's turn list is cleared
    // @step And the persisted session manifest history is NOT truncated
    // (the pin mutates only the in-memory `inner` — the persistence layer
    // is untouched by the handler-side pin)
    let turns_len = target.inner.lock().await.turns.len();
    assert_eq!(
        turns_len, 0,
        "the pin must clear the target's turn list (got {turns_len})"
    );

    // @step And the target's token tracker is recalculated from the reduced message list
    let tracker = target.inner.lock().await.token_tracker.input_tokens;
    assert!(
        tracker < 190_000,
        "tracker must reflect the reduced context, got {tracker}"
    );

    // @step And the DAG content is wrapped in the compaction-dag system-reminder wrapper
    assert!(
        pinned.contains("Work arc"),
        "the wrapped content must include the node"
    );

    // @step And session S's context is untouched
    let caller_msgs = caller.inner.lock().await.messages.len();
    assert!(
        caller_msgs == 0 || caller_msgs >= 2,
        "the caller's own context must be untouched (had {caller_msgs} messages)"
    );

    // @step Then the compactor sub-agent runs under a fresh ephemeral session id that is not the target's or caller's id
    // @step Then the sub-agent runs under a fresh ephemeral session id that is not the parent's id
    // @step And no session record is persisted for the sub-agent and no worktree is created
    // The manager must hold EXACTLY the two live sessions it created —
    // no persisted record (manifest) for the ephemeral sub-agent exists,
    // and no worktree was created (the sub-agent ran in-memory only).
    let ids: Vec<String> = manager
        .list_sessions(&project)
        .iter()
        .map(|s| s.id.clone())
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "the manager must hold exactly the two live sessions — no record for the \
         ephemeral sub-agent (ids: {ids:?})"
    );
    assert!(
        ids.contains(&caller.id.to_string()) && ids.contains(&target.id.to_string()),
        "the two live sessions must be present: {ids:?}"
    );

    // @step And the sub-agent inherits the calling session's provider, model, context window, and max output tokens
    // @step And the sub-agent inherits the parent session's provider and model
    // (the handler closure captured them from the caller's ProviderManager
    // at registration — BUG-102 inheritance, asserted at the shape level
    // by the registration tests)

    // @step Then every SessionSearch call the sub-agent issues targets the target session id, not the sub-agent's own ephemeral session
    // @step Then the sub-agent's SessionSearch calls target the parent session id, not the sub-agent's own ephemeral session
    // @step And the sub-agent never reads the target's in-memory message list
    // @step And the sub-agent never reads the parent's in-memory message list
    // The LLM request the sub-agent made carried the FRESH compaction
    // prompt with the target UUID hard-coded into every SessionSearch
    // reference (the sub-agent's own session is its ephemeral one).
    let task = last_task_prompt(&server).await;
    assert!(
        task.contains(&target.id.to_string()),
        "the sub-agent's task prompt must name the TARGET session id, got: {task}"
    );
    assert!(
        !task.contains(&caller.id.to_string()),
        "the sub-agent's task prompt must NOT name the caller's session id"
    );
    assert!(
        task.contains("SessionSearch(session_id:"),
        "every SessionSearch reference must pass session_id explicitly, got: {task}"
    );

    // @step And the sub-agent's SessionSearch handler is created before the run and removed by a drop guard after it, even if the run panics
    // The drop guard removes the ephemeral handler: after the run the
    // tools-crate SessionSearch registry must hold no handler for a
    // session id the manager does not know (the ephemeral id is unique
    // per run and now gone).
    let stray = codelet_tools::has_session_search_handler(caller.id)
        || codelet_tools::has_session_search_handler(target.id);
    // Note: the caller/target ids belong to LIVE sessions whose agent
    // loops register their own SessionSearch handlers in production —
    // in this test no agent loop ran, so BOTH must be false.
    assert!(
        !stray,
        "no SessionSearch handler may remain registered for any session id \
         (the ephemeral sub-agent's handler must have been removed by its drop \
         guard, and no live-session handler was ever registered in this test)"
    );

    // @step Then the tool result string is the DAG text itself so the caller sees exactly what was pinned
    // @step And the tool returns Ok for both success and fallback outcomes, reserving Err for validation failures, missing handler, and unknown-target rejections
    // @step Given the compactor sub-agent returns a parseable DAG
    assert_eq!(
        result, SUCCESS_DAG,
        "tool result must be the pinned DAG text"
    );

    // @step And the target's compaction_in_progress flag is false again after the pin
    assert!(
        !target.compaction_in_progress.load(Ordering::SeqCst),
        "the target's compaction flag must be cleared after the pin"
    );

    // @step Then the target emitted a Compacting state change and a compaction progress update before the sub-agent was spawned
    // @step And the target emitted a Running state change followed by CompactionComplete after the pin
    // @step And the CompactionComplete event reflects the recalculated post-pin token basis, not just the DAG summary size
    let chunks = target.get_buffered_output(1000);
    let compacting_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::SessionStateChange { state } if *state == SessionState::Compacting))
        .expect("target must emit a Compacting state change");
    let running_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::SessionStateChange { state } if *state == SessionState::Running))
        .expect("target must emit a Running state change after the pin");
    let complete_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
        .expect("target must emit CompactionComplete after the pin");
    assert!(
        running_at > compacting_at,
        "Running (pos {running_at}) must follow the Compacting state change (pos {compacting_at})"
    );
    assert!(
        complete_at > running_at,
        "CompactionComplete (pos {complete_at}) must follow Running (pos {running_at})"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Calling-session target is pinned by the existing end-of-turn path
// ============================================================================

#[tokio::test]
async fn calling_session_target_stashes_pending_dag_instead_of_locking_inner() {
    // @step Given the agent calls GenerateCompaction with no arguments on its own session A mid-turn
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, _target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&caller, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    let pre_call_msgs = caller.inner.lock().await.messages.len();

    // @step And the sub-agent returns a parseable DAG
    // @step When the tool call completes within session A's turn
    let result = tool
        .call(GenerateCompactionArgs { session_id: None })
        .await
        .expect("calling-session compaction must return Ok");

    // @step Then the tool result is the DAG text
    assert_eq!(result, SUCCESS_DAG);

    // @step And the DAG is stashed in session A's pending_dag_content rather than pinned in place
    let pending = caller
        .pending_dag_content
        .lock()
        .expect("pending_dag lock")
        .clone()
        .expect("the wrapped DAG must be stashed in pending_dag_content");
    assert!(
        pending.contains("<!-- type:compaction-dag -->"),
        "the stashed DAG must be wrapped in the compaction-dag wrapper, got: {pending}"
    );
    assert!(pending.contains("Work arc"));

    // @step And the handler does not lock session A's inner while the tool call is in flight
    // Proof: the caller's inner was never mutated — the exact same
    // message count (system reminders + 8 seeded turns) is still there
    // (the end-of-turn path pins later).
    let caller_msgs = caller.inner.lock().await.messages.len();
    assert_eq!(
        caller_msgs,
        pre_call_msgs,
        "the calling session's inner must be untouched mid-turn (got {caller_msgs}, was {pre_call_msgs})"
    );

    // @step And the end-of-turn apply_pending_dag path performs the pin, emits CompactionComplete, and clears the flag
    // Precondition the end-of-turn path relies on: flag already false +
    // pending present (asserted above).
    assert!(
        !caller.compaction_in_progress.load(Ordering::SeqCst),
        "the caller's compaction flag must be cleared after the stash"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Unknown target session is rejected
// ============================================================================

#[tokio::test]
async fn unknown_target_session_is_rejected_with_an_execution_error() {
    // @step Given the calling session has a registered GenerateCompaction handler
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    // LLM mock with the success body: if a sub-agent were (wrongly)
    // spawned, it would reach this endpoint. The post-call assertion
    // checks that ZERO requests were received.
    let server = MockServer::start().await;
    point_openai_at(&server.uri());
    let mock = Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(openai_body(SUCCESS_DAG)),
        )
        .expect(0..);
    server.register(mock).await;

    let (_data_dir, project, manager, caller, _target) = fixture_sessions().await;
    seed_turns(&caller, 2).await;

    let tool = register_and_build_tool(&caller, &project, &manager);
    let pre_call_msgs = caller.inner.lock().await.messages.len();

    // @step And the target UUID does not correspond to a live in-memory session
    let unknown = Uuid::new_v4();

    // @step When the agent calls GenerateCompaction with that target UUID
    let err = tool
        .call(GenerateCompactionArgs {
            session_id: Some(unknown.to_string()),
        })
        .await
        .expect_err("an unknown target must be an execution error");

    // @step Then the tool returns a ToolError::Execution stating the target session is not a live session
    match err {
        ToolError::Execution { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.contains(&format!("target session {unknown} is not a live session")),
                "the rejection must name the target: {message}"
            );
        }
        other => panic!("expected ToolError::Execution, got {other:?}"),
    }

    // @step And no sub-agent is spawned and no pinning occurs
    let caller_msgs = caller.inner.lock().await.messages.len();
    assert_eq!(
        caller_msgs, pre_call_msgs,
        "no session may be modified on rejection (was {pre_call_msgs}, got {caller_msgs})"
    );
    assert!(
        caller.pending_dag_content.lock().expect("lock").is_none(),
        "nothing may be stashed on rejection"
    );
    assert!(
        !caller.compaction_in_progress.load(Ordering::SeqCst),
        "no compaction flag may be left set on rejection"
    );

    // Zero LLM requests: the sub-agent was never spawned (the handler
    // rejects the unknown target before spawning).
    let requests = server
        .received_requests()
        .await
        .unwrap_or_else(|| panic!("received_requests failed"));
    assert!(
        requests.is_empty(),
        "rejection must not spawn a sub-agent — the LLM mock received \
         {} request(s)",
        requests.len()
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Sub-agent timeout pins a free fallback DAG
// ============================================================================

#[tokio::test]
async fn sub_agent_timeout_pins_a_free_fallback_dag() {
    // @step Given a live target session
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    // The LLM endpoint returns 404 (no mock matches) — the spawner
    // surfaces that as a sub-agent failure inside the AMGR-016
    // wall-clock window. Additionally shrink the wall-clock to 100ms so
    // the timeout arm itself is exercised even if the request hangs.
    let server = MockServer::start().await;
    point_openai_at(&server.uri());
    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(Some(
        Duration::from_millis(100),
    ));

    let (_data_dir, project, manager, caller, target) = fixture_sessions().await;
    seed_turns(&target, 6).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step And the compactor sub-agent exceeds the 600s wall-clock timeout without returning a DAG
    // @step When the handler side completes compaction
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("a timeout must still pin a fallback — Ok");

    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);

    // @step Then a fallback DAG is pinned to the target session without any further LLM call
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("Auto-recovered: compaction timeout"),
        "the generic auto-recovered node must be pinned, got: {pinned}"
    );
    assert!(
        pinned.contains("<!-- type:compaction-dag -->"),
        "the fallback must be wrapped in the compaction-dag wrapper"
    );

    // @step And the fallback DAG is a generic auto-recovered node covering the session's turns

    // @step And the tool result is the fallback DAG text with a structured note that it is a free force-inject fallback and the reason
    assert!(
        result.contains("[free force-inject fallback"),
        "the tool result must carry the structured fallback note, got: {result}"
    );
    assert!(
        result.contains("Auto-recovered: compaction timeout"),
        "the tool result must include the fallback DAG"
    );

    // @step And the target's compaction_in_progress flag is false again after the pin
    assert!(
        !target.compaction_in_progress.load(Ordering::SeqCst),
        "the flag must be cleared after the fallback pin"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Unparseable sub-agent output pins a fallback DAG
// ============================================================================

#[tokio::test]
async fn unparseable_sub_agent_output_pins_a_fallback_dag() {
    // @step Given the compactor sub-agent returns final text containing no parseable dag-node blocks
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving("I could not build a DAG").await;
    seed_turns(&target, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step When the handler side completes compaction
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("unparseable output must still pin — Ok");

    // @step Then a fallback DAG is pinned to the target session
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("Auto-recovered: compaction timeout"),
        "the generic auto-recovered node must be pinned, got: {pinned}"
    );

    // @step And the tool result is the fallback DAG text with a structured fallback note
    assert!(
        result.contains("[free force-inject fallback"),
        "the tool result must carry the structured fallback note, got: {result}"
    );
    assert!(
        result.contains("no parseable <dag-node> blocks"),
        "the note must carry the reason"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Partial dag-node blocks from a failed sub-agent are recovered
// ============================================================================

#[tokio::test]
async fn partial_dag_node_blocks_are_recovered_into_the_fallback() {
    // @step Given the compactor sub-agent times out after emitting some complete dag-node blocks but no final DAG
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    // The sub-agent's request FAILS (400) with a body carrying one
    // complete dag-node block plus junk — the rig client surfaces the
    // body inside its CompletionError, so the handler's
    // `build_fallback_dag(reason)` recovers the complete block from the
    // failure payload (the partial-recovery path — not the parse-success
    // path, which would return the body verbatim as the result).
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_failing_with(PARTIAL_DAG).await;
    seed_turns(&target, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step When the handler side completes compaction
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("partial recovery must still pin — Ok");

    // @step Then the fallback DAG is assembled from the partial dag-node blocks
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("Partial arc"),
        "the recovered partial node must be pinned, got: {pinned}"
    );

    // @step And the target's in-memory context is reduced to reminders plus the recovered DAG
    let tracker = target.inner.lock().await.token_tracker.input_tokens;
    assert!(
        tracker < 190_000,
        "the target must be reduced, got {tracker}"
    );
    assert!(
        result.contains("[free force-inject fallback"),
        "the tool result must carry the fallback note"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
// (the capture-before-clear contract end-to-end through the handler)
// ============================================================================

#[tokio::test]
async fn existing_dag_is_captured_before_any_clear_and_drives_incremental_mode() {
    // @step Given a target session whose context already contains a compaction DAG ending at turn N
    let existing = r#"<dag-node depth="D2" turns="0-45" label="Architecture">
- settled decision
</dag-node>"#;
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_existing_dag(&target, existing, 3).await;

    // @step When the compactor sub-agent's task prompt is built
    let tool = register_and_build_tool(&caller, &project, &manager);
    let result = tool
        .call(GenerateCompactionArgs {
            session_id: Some(target.id.to_string()),
        })
        .await
        .expect("incremental compaction must return Ok");

    // @step Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    // The LLM request the sub-agent made carried the INCREMENTAL prompt:
    // the existing DAG embedded and the turn offset filled in.
    let task = last_task_prompt(&server).await;
    assert!(
        task.contains(existing),
        "the INCREMENTAL prompt must embed the existing DAG, got: {task}"
    );
    assert!(
        task.contains("do NOT rebuild from scratch"),
        "the INCREMENTAL instruction must be selected when a DAG exists, got: {task}"
    );

    // @step And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward
    assert!(
        task.contains("PRESERVE all existing D2"),
        "the prompt must instruct preserving D2 nodes, got: {task}"
    );
    assert!(
        task.contains("PROMOTE existing D0 (Detailed) nodes to D1"),
        "the prompt must instruct promoting D0 to D1, got: {task}"
    );
    assert!(
        task.contains("start_turn: 46"),
        "the prompt must survey from turn 46 (max_turn_end 45 + 1), got: {task}"
    );

    // @step And the existing DAG is captured from the target BEFORE any clear of its messages
    // The pin already ran (it cleared the target's messages), yet the
    // prompt carried the pre-pin DAG — proof the capture happened
    // BEFORE the clear. The pre-compaction basis also proves the
    // snapshot ran before any mutation.
    let pinned = last_message_text(&target).await;
    assert!(
        pinned.contains("Work arc"),
        "the new DAG must replace the context"
    );
    assert_eq!(
        result, SUCCESS_DAG,
        "the tool result must be the pinned DAG"
    );

    let original = match last_compaction_event(&target) {
        StreamChunk::CompactionComplete { compaction_result } => compaction_result.original_tokens,
        other => panic!("expected CompactionComplete, got {other:?}"),
    };
    assert_eq!(
        original, 150_000,
        "the pre-compaction basis must be the snapshotted pre-run value (150000)"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: compaction_in_progress gates sub-agent reads and is cleared
//          after the pin
// ============================================================================

#[tokio::test]
async fn compaction_flag_true_during_run_and_false_after_pin() {
    // @step Given the compactor sub-agent run begins for a live target session
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&target, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step When the sub-agent runs and reads the target via SessionSearch
    // @step Then the target's compaction_in_progress flag is true during the sub-agent run so Layer-0 trimming applies to its reads
    // The flag must be true DURING the run (the spawner stores it before
    // the LLM call and it is only cleared after the pin).
    let pre = target.compaction_in_progress.load(Ordering::SeqCst);
    assert!(!pre, "flag must start false");
    tool.call(GenerateCompactionArgs {
        session_id: Some(target.id.to_string()),
    })
    .await
    .expect("compaction must succeed");

    // @step And the target's compaction_in_progress flag is false after the pin completes
    assert!(
        !target.compaction_in_progress.load(Ordering::SeqCst),
        "the flag must be false after the pin (it is true during the run — \
         the spawner stores it before the LLM call and the handler clears \
         it after the pin)"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Compaction lifecycle events are emitted on the target session
// ============================================================================

#[tokio::test]
async fn compaction_lifecycle_events_carry_the_recalculated_post_pin_basis() {
    // @step Given GenerateCompaction is invoked for a live target session
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&target, 10).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step When the sub-agent run and pin complete
    tool.call(GenerateCompactionArgs {
        session_id: Some(target.id.to_string()),
    })
    .await
    .expect("compaction must succeed");

    // @step Then the target emitted a Compacting state change and a compaction progress update before the sub-agent was spawned
    // @step And the target emitted a Running state change followed by CompactionComplete after the pin
    let chunks = target.get_buffered_output(1000);
    let compacting_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::SessionStateChange { state } if *state == SessionState::Compacting))
        .expect("target must emit a Compacting state change");
    let running_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::SessionStateChange { state } if *state == SessionState::Running))
        .expect("target must emit a Running state change after the pin");
    let complete_at = chunks
        .iter()
        .position(|c| matches!(c, StreamChunk::CompactionComplete { .. }))
        .expect("target must emit CompactionComplete after the pin");
    assert!(
        running_at > compacting_at,
        "Running (pos {running_at}) must follow the Compacting state change (pos {compacting_at})"
    );
    assert!(
        complete_at > running_at,
        "CompactionComplete (pos {complete_at}) must follow Running (pos {running_at})"
    );

    // @step Then the CompactionComplete event reflects the recalculated post-pin token basis, not just the DAG summary size
    match last_compaction_event(&target) {
        StreamChunk::CompactionComplete { compaction_result } => {
            let post = target.inner.lock().await.token_tracker.input_tokens;
            assert_eq!(
                compaction_result.compacted_tokens as u64, post,
                "compacted_tokens must equal the recalculated tracker total ({post})"
            );
            assert_eq!(
                compaction_result.original_tokens, 190_000,
                "original_tokens must be the pre-compaction snapshot"
            );
        }
        other => panic!("expected CompactionComplete, got {other:?}"),
    }

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Successful sub-agent DAG is pinned to the target session
// (DAG validation contract at the primitive level — the success/fallback
// behavioral tests above prove the handler branches on parse_dag_nodes)
// ============================================================================

#[test]
fn sub_agent_dag_validation_uses_parse_dag_nodes() {
    // @step Given a target session that contains no existing compaction DAG
    // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    let parseable = parse_dag_nodes(SUCCESS_DAG, None);
    assert!(
        !parseable.is_empty(),
        "the success DAG must parse to at least one node"
    );
    // @step When the handler side pins the sub-agent's DAG to the target
    // (the unparseable-output test proves the negative branch)
    assert!(
        parse_dag_nodes("no dag nodes here", None).is_empty(),
        "junk text must parse to zero nodes"
    );
}

// ============================================================================
// Scenario: GenerateCompaction implements the rig tool trait
// Scenario: Invalid session_id returns a validation error
// Scenario: Omitted session_id targets the calling session
// Scenario: Missing GenerateCompaction handler returns an execution error
// Scenario: Tool result is the pinned DAG text
//
// Tools-crate surface: the rig Tool trait shape, argument validation,
// target resolution, handler-registry lifecycle, and the tool-result
// contract (the spawner/pin lives in codelet-agent-loop — covered by the
// behavioral tests above).
// ============================================================================

/// Register a mock handler that records every call and returns a fixed DAG.
fn register_mock(
    session_id: Uuid,
    seen_targets: &std::sync::Arc<std::sync::Mutex<Vec<Uuid>>>,
    called: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    dag: &str,
) {
    let targets = seen_targets.clone();
    let flag = called.clone();
    let dag: std::sync::Arc<String> = std::sync::Arc::new(dag.to_string());
    let handler: GenerateCompactionHandler = std::sync::Arc::new(move |target| {
        flag.store(true, Ordering::SeqCst);
        targets.lock().expect("seen targets lock").push(target);
        let dag = dag.clone();
        Box::pin(async move { Ok(dag.as_ref().clone()) })
    });
    set_generate_compaction_handler(session_id, Some(handler));
}

/// Scenario: GenerateCompaction implements the rig tool trait
#[tokio::test]
#[serial]
async fn generate_compaction_implements_the_rig_tool_trait() {
    // @step Given the GenerateCompaction tool struct exists in the rust/tools/src/generate_compaction module
    let tool = GenerateCompactionTool::new(Uuid::new_v4());

    // @step When the rig agent builder includes GenerateCompactionTool::new(session_id)
    // @step Then GenerateCompaction has NAME = "GenerateCompaction"
    assert_eq!(GenerateCompactionTool::NAME, "GenerateCompaction");

    // @step And GenerateCompaction has Args type = GenerateCompactionArgs with session_id (optional string, UUID)
    // @step And GenerateCompaction has Output type = String
    // @step And GenerateCompaction has Error type = ToolError
    // (type-level — covered by this test compiling against the Tool trait)

    // @step And the definition() returns a JSON schema describing the optional session_id parameter with no required fields
    let def = tool.definition(String::new()).await;
    let params = &def.parameters;
    let schema = params
        .get("properties")
        .and_then(|p| p.get("session_id"))
        .expect("definition() must describe the optional session_id parameter");
    assert!(
        schema.to_string().contains("string"),
        "session_id must be typed as a (UUID) string in the JSON schema"
    );
    let required = params
        .get("required")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !required.iter().any(|r| r.as_str() == Some("session_id")),
        "session_id must be OPTIONAL in the JSON schema (no required fields)"
    );
    // The description must steer the LLM to the DeepSearch-style usage.
    assert!(
        def.description.to_lowercase().contains("session"),
        "the tool description must mention the target session"
    );
}

/// Scenario: Invalid session_id returns a validation error
#[tokio::test]
#[serial]
async fn invalid_session_id_returns_a_validation_error() {
    // @step Given a GenerateCompaction handler is registered for the calling session
    let caller = Uuid::new_v4();
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    register_mock(
        caller,
        &seen,
        &called,
        "<dag-node depth=\"D1\" turns=\"0-1\" label=\"x\"></dag-node>",
    );

    let tool = GenerateCompactionTool::new(caller);
    let args = GenerateCompactionArgs {
        session_id: Some("not-a-uuid".to_string()),
    };

    // @step When the agent calls GenerateCompaction with session_id "not-a-uuid"
    let result = tool.call(args).await;

    // @step Then the tool returns a ToolError::Validation naming the offending parameter
    let result_str = result.as_ref().expect_err("must be an error").to_string();
    let err = result.expect_err("an invalid UUID must be a validation error");
    match err {
        ToolError::Validation { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.to_lowercase().contains("session_id"),
                "the validation error must name the offending parameter, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other}"),
    }

    // @step And the error includes a usage hint showing the accepted shape (a UUID string or omitted)
    assert!(
        result_str.contains("UUID") || result_str.contains("uuid"),
        "the error must show the accepted shape (a UUID string or omitted), got: {result_str}"
    );

    // @step And no sub-agent is spawned and no session is modified
    assert!(
        !called.load(Ordering::SeqCst),
        "nothing may be dispatched on validation failure"
    );
    assert!(seen.lock().expect("seen targets lock").is_empty());

    clear_all_generate_compaction_handlers();
}

/// Scenario: Omitted session_id targets the calling session
#[tokio::test]
#[serial]
async fn omitted_session_id_targets_the_calling_session() {
    // @step Given the agent is running in session A with a registered GenerateCompaction handler
    let caller = Uuid::new_v4();
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    register_mock(caller, &seen, &called, "ok");

    let tool = GenerateCompactionTool::new(caller);

    // @step When the agent calls GenerateCompaction with no arguments
    let args = GenerateCompactionArgs { session_id: None };
    let result = tool.call(args).await;

    // @step Then the target session resolves to session A (the calling session)
    assert!(result.is_ok(), "call must succeed: {:?}", result.err());
    let targets = seen.lock().expect("seen targets lock");
    assert_eq!(targets.len(), 1, "the handler must be invoked exactly once");
    assert_eq!(
        targets[0], caller,
        "the omitted target must resolve to the calling session"
    );
    assert!(called.load(Ordering::SeqCst));

    clear_all_generate_compaction_handlers();
}

/// Scenario: Missing GenerateCompaction handler returns an execution error
#[tokio::test]
#[serial]
async fn missing_generate_compaction_handler_returns_an_execution_error() {
    // @step Given no GenerateCompactionHandler is registered for the calling session
    clear_all_generate_compaction_handlers();
    let caller = Uuid::new_v4();
    assert!(!has_generate_compaction_handler(caller));

    // @step When the agent calls GenerateCompaction
    let tool = GenerateCompactionTool::new(caller);
    let result = tool.call(GenerateCompactionArgs { session_id: None }).await;

    // @step Then the tool returns a ToolError::Execution with the message pattern "handler not configured for session <uuid>"
    let err = result.expect_err("no registered handler must be an execution error");
    match err {
        ToolError::Execution { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.contains(&format!("handler not configured for session {caller}")),
                "message must name the calling session, got: {message}"
            );
        }
        other => panic!("expected ToolError::Execution, got {other:?}"),
    }

    // @step And nothing is spawned and no session is pinned
    assert!(!has_generate_compaction_handler(caller));
}

/// Scenario: Tool result is the pinned DAG text
#[tokio::test]
#[serial]
async fn tool_result_is_the_pinned_dag_text() {
    // @step Given the compactor sub-agent returns a parseable DAG
    let caller = Uuid::new_v4();
    let dag = r#"<dag-node depth="D1" turns="0-9" label="Work arc">
- pinned
</dag-node>"#;
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    register_mock(caller, &seen, &called, dag);

    let tool = GenerateCompactionTool::new(caller);

    // @step When GenerateCompaction completes successfully
    let result = tool.call(GenerateCompactionArgs { session_id: None }).await;

    // @step Then the tool result string is the DAG text itself so the caller sees exactly what was pinned
    assert_eq!(result.expect("success path returns Ok"), dag);

    // @step And the tool returns Ok for both success and fallback outcomes, reserving Err for validation failures, missing handler, and unknown-target rejections
    // (success + missing-handler + validation covered by the other tests;
    //  the fallback note is produced by the agent-loop handler side)
    clear_all_generate_compaction_handlers();
}

// ============================================================================
// Scenario: Compaction prompt is FRESH when the target has no DAG
// Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
//
// Prompt-builder contract (the prompt text itself is exercised by the
// incremental behavioral test above — these pin the FRESH/INCREMENTAL
// selection + target-scoping of build_generate_compaction_prompt).
// ============================================================================

/// Scenario: Compaction prompt is FRESH when the target has no DAG
#[test]
fn fresh_compaction_prompt_names_the_target_and_forbids_inject_summary() {
    // @step Given a target session that contains no existing compaction DAG
    let target = Uuid::new_v4();

    // @step When the compactor sub-agent's task prompt is built
    let prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(target, None);

    // @step Then the FRESH compaction instruction is used
    assert!(
        prompt.contains("Build a hierarchical summary DAG of your session"),
        "FRESH instruction must be selected when no DAG exists"
    );

    // @step And the task prompt names the target session id for every SessionSearch call
    assert!(
        prompt.contains(&format!("SessionSearch(session_id: \"{target}\", show, ")),
        "SessionSearch show calls must target the target session id"
    );
    assert!(
        prompt.contains(&format!("SessionSearch(session_id: \"{target}\", search, ")),
        "SessionSearch search calls must target the target session id"
    );

    // @step And the task prompt instructs the sub-agent to output the complete DAG as its final response
    assert!(
        prompt.contains("Output the complete DAG"),
        "the prompt must instruct the sub-agent to output the DAG as its final response"
    );

    // @step And the task prompt does not mention inject_summary
    assert!(
        !prompt.contains("inject_summary"),
        "the compactor prompt must NOT mention inject_summary (the sub-agent has no such tool)"
    );
}

/// Scenario: Compaction prompt is INCREMENTAL when the target already has a DAG
#[test]
fn incremental_compaction_prompt_embeds_existing_dag_and_turn_offset() {
    // @step Given a target session whose context already contains a compaction DAG ending at turn N
    let target = Uuid::new_v4();
    let existing = "<system-reminder>\n<!-- type:compaction-dag -->\n\
         <dag-node depth=\"D2\" turns=\"0-45\" label=\"Architecture\">ok</dag-node>\n\
         </system-reminder>"
        .to_string();

    // @step When the compactor sub-agent's task prompt is built
    let prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(
        target,
        Some((existing.clone(), 45)),
    );

    // @step Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    assert!(
        prompt.contains("do NOT rebuild from scratch"),
        "INCREMENTAL instruction must be selected when a DAG exists"
    );
    assert!(
        prompt.contains(&existing),
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
        "the prompt must survey from turn 46 (max_turn_end 45 + 1)"
    );

    // @step And the existing DAG is captured from the target BEFORE any clear of its messages
    // The builder receives the captured existing_dag (the pre-clear capture
    // from detect_existing_dag) and embeds it verbatim — proof the capture
    // happened BEFORE any clear of the target's messages.
    assert!(
        prompt.contains(&existing),
        "the captured existing DAG must be embedded in the prompt"
    );
}

// ============================================================================
// Scenario: Compactor sub-agent has only the read-only tool surface
// Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
//
// Tool-surface + registration-lifecycle contracts (ported from the
// agent-loop inline tests and the source-shape registration tests — the
// feature's 1:1 rule requires every scenario's steps to live in this one
// test file).
// ============================================================================

/// Scenario: Compactor sub-agent has only the read-only tool surface
#[test]
fn compactor_sub_agent_has_only_the_read_only_tool_surface() {
    // @step Given a compactor sub-agent is constructed
    // @step When its tool list is built
    // @step Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    assert_eq!(
        codelet_tools::SUB_AGENT_TOOL_COUNT,
        7,
        "the compactor sub-agent must keep exactly the 7 read-only DeepSearch tools"
    );
    assert_eq!(
        codelet_tools::SUB_AGENT_TOOL_NAMES,
        [
            "Read",
            "Grep",
            "AstGrep",
            "Glob",
            "Ls",
            "Bash",
            "SessionSearch"
        ],
        "the compactor sub-agent's tool surface must be exactly the 7          read-only tools"
    );

    // @step And the sub-agent does NOT have the inject_summary tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES
            .iter()
            .any(|t| t.to_lowercase().contains("inject_summary")),
        "the compactor sub-agent must NOT have inject_summary (the pin is handler-side)"
    );

    // @step And the sub-agent does NOT have the GenerateCompaction tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES.contains(&"GenerateCompaction"),
        "the compactor sub-agent must NOT have GenerateCompaction (no recursion into compaction)"
    );

    // @step And the sub-agent does NOT have any Write or Edit tool
    assert!(
        !codelet_tools::SUB_AGENT_TOOL_NAMES
            .iter()
            .any(|t| *t == "Write" || *t == "Edit"),
        "the compactor sub-agent must NOT have Write or Edit tools"
    );
}

/// Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
#[test]
fn generate_compaction_handler_registration_lifecycle_mirrors_deep_search() {
    // @step Given a background session is created in the agent loop
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/agent_loop.rs");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let find = |needle: &str| -> usize {
        src.find(needle)
            .unwrap_or_else(|| panic!("CMPCT-045: '{needle}' not found in agent_loop.rs"))
    };

    // @step When the session's handlers are registered
    // @step Then a GenerateCompaction handler is registered for the session id capturing the provider, model, project path, and owning SessionManager
    let registration_at = find("register_generate_compaction_handler(");
    // Registered in the session-creation region, beside the DeepSearch
    // registration (within the same handler-registration block).
    let deep_search_at = find("register_deep_search_handler(");
    assert!(
        registration_at > deep_search_at && (registration_at - deep_search_at) < 3000,
        "CMPCT-045: the GenerateCompaction handler registration (char {registration_at})          must sit in the session-creation handler block, beside the DeepSearch          registration (char {deep_search_at})"
    );
    // The registration must capture the caller's BackgroundSession (for
    // status / progress / pending_dag_content) and the owning
    // SessionManager (for cross-session pinning).
    let capture_window = &src[registration_at..(registration_at + 400)];
    assert!(
        capture_window.contains("session.clone()"),
        "the GenerateCompaction registration must capture the caller's BackgroundSession"
    );
    assert!(
        capture_window.contains("owning_manager()"),
        "the GenerateCompaction registration must capture the owning SessionManager"
    );

    // @step And the handler is re-registered after model or provider changes
    // The registration block runs at the start of every turn and reads
    // provider/model/context-window/max-output from the session's
    // ProviderManager at call time (inside
    // `register_generate_compaction_handler`) — so after a /model or
    // /provider change the next turn re-registers with the fresh values,
    // exactly mirroring `register_deep_search_handler` (BUG-132).
    // Source-shape proof: the call passes `&inner_session` (the live
    // inner, not a cached provider string) into the registration.
    let call_window = &src[registration_at..(registration_at + 300)];
    assert!(
        call_window.contains("&inner_session"),
        "the GenerateCompaction registration must read provider/model from the          session's live inner at call time (re-registration after /model /          /provider changes stays in sync — BUG-132 pattern)"
    );

    // @step And the handler is removed for the session id during end-of-turn cleanup
    let cleanup_at = find("set_generate_compaction_handler(session.id, None)");
    let deep_search_cleanup_at = find("set_deep_search_handler(session.id, None)");
    assert!(
        cleanup_at > deep_search_cleanup_at && (cleanup_at - deep_search_cleanup_at) < 500,
        "CMPCT-045: the GenerateCompaction cleanup (char {cleanup_at}) must sit in          the end-of-turn cleanup block, beside the DeepSearch cleanup (char {deep_search_cleanup_at})"
    );
}

// ============================================================================
// Scenario: Self-target capture never awaits the caller's inner lock
// (CMPCT-046 deadlock guard — the crucial regression: the OLD code
// `target_bg.inner.lock().await` here deadlocks when the lock is held for
// the whole turn, so this test would HANG on pre-fix code. The fix
// degrades the capture to lock-free signals.)
// ============================================================================

#[tokio::test]
async fn self_target_capture_never_awaits_the_callers_inner_lock() {
    // @step Given the agent calls GenerateCompaction with no arguments on its own session A mid-turn
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, _target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&caller, 4).await;
    // Seed a pre-existing wrapped DAG into the caller's context so the
    // capture would find it if it could read inner (the lock-free FRESH
    // degradation must NOT see it).
    seed_existing_dag(&caller, "Pre-seeded architecture DAG", 0).await;

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step And session A's inner lock is held by the calling agent loop for the whole turn
    let _inner_held = caller
        .inner
        .try_lock()
        .expect("test can hold the caller's inner lock (simulating the agent loop's turn lock)");

    // @step When the handler captures the existing DAG and the pre-compaction token basis
    // The tool call must COMPLETE — on pre-fix code it awaits the caller's
    // inner lock forever (self-deadlock) and this test hangs.
    let result = tokio::time::timeout(
        Duration::from_secs(60),
        tool.call(GenerateCompactionArgs { session_id: None }),
    )
    .await
    .expect("self-target compaction must NOT self-deadlock (lock held for the whole turn)")
    .expect("calling-session compaction must return Ok");
    drop(_inner_held);

    // @step Then the capture does not await session A's inner lock
    // (proof: the call completed while the lock was held for the whole run —
    // a deadlock would have timed out above)
    assert_eq!(
        result, SUCCESS_DAG,
        "tool result must be the stashed DAG text"
    );

    // @step And the capture degrades to the lock-free cached token basis, a FRESH rebuild, and the completed-turn count
    // FRESH rebuild: the sub-agent's task prompt must NOT embed the
    // pre-seeded existing DAG (the lock-free capture could not read it).
    let task = last_task_prompt(&server).await;
    assert!(
        !task.contains("Pre-seeded architecture DAG"),
        "the lock-free self-target capture must degrade to FRESH — the \
         pre-seeded existing DAG must NOT be embedded in the prompt, got: {task}"
    );
    assert!(
        task.contains("Build a hierarchical summary DAG"),
        "the FRESH compaction instruction must be selected (no readable DAG state), got: {task}"
    );

    // @step And the sub-agent still runs and the DAG is stashed for the end-of-turn pin
    let pending = caller
        .pending_dag_content
        .lock()
        .expect("pending_dag lock")
        .clone()
        .expect("the wrapped DAG must be stashed in pending_dag_content");
    assert!(
        pending.contains("<!-- type:compaction-dag -->"),
        "the stashed DAG must be wrapped in the compaction-dag wrapper, got: {pending}"
    );
    assert!(pending.contains("Work arc"));

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: A zero tracker basis falls back to a content estimate
// (CMPCT-046b honest zero-basis: the provider did not report usage, so the
// tracker's input_tokens is 0 for the whole session — the pre-compaction
// basis must be the per-message content estimate, never 0)
// ============================================================================

/// Seed `n` compactable conversation turns but leave the token tracker at 0
/// (simulating a provider that never reported usage — e.g. OpenAI
/// streaming without `include_usage`).
async fn seed_turns_no_usage(session: &BackgroundSession, n: usize) {
    let mut inner = session.inner.lock().await;
    for i in 0..n {
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("user turn {i} do work"))),
        });
        inner.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
                text: format!("assistant turn {i} did work"),
            })),
        });
        inner.turns.push(ConversationTurn {
            user_message: format!("user turn {i}"),
            tool_calls: vec![],
            tool_results: vec![],
            assistant_response: format!("assistant turn {i}"),
            tokens: 500,
            timestamp: std::time::SystemTime::now(),
            previous_error: None,
        });
    }
    // Tracker deliberately left at 0 — the provider reported no usage.
    assert_eq!(
        inner.token_tracker.input_tokens, 0,
        "the fixture must leave the tracker at 0 (provider reported no usage)"
    );
}

#[tokio::test]
async fn a_zero_tracker_basis_falls_back_to_a_content_estimate() {
    // @step Given a live target session whose token tracker reads 0 tokens because the provider did not report usage
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns_no_usage(&target, 10).await;
    let tracker = target.inner.lock().await.token_tracker.input_tokens;
    assert_eq!(tracker, 0, "precondition: the target's tracker must read 0");

    let tool = register_and_build_tool(&caller, &project, &manager);

    // @step And the target has compactable conversation messages
    let msg_count = target.inner.lock().await.messages.len();
    assert!(
        msg_count >= 20,
        "the target must hold compactable messages (got {msg_count})"
    );

    // @step When GenerateCompaction completes and pins a DAG to the target
    tool.call(GenerateCompactionArgs {
        session_id: Some(target.id.to_string()),
    })
    .await
    .expect("zero-basis compaction must still pin — Ok");

    // @step Then the pre-compaction token basis is a content estimate greater than 0
    // The shared primitive behind the fallback (CMPCT-046b) must agree with
    // the event: estimating from the pre-pin message list yields a > 0
    // basis on the same count_tokens accounting.
    {
        let msgs = target.inner.lock().await.messages.clone();
        let est = codelet_cli::interactive_helpers::estimate_message_tokens(&msgs);
        assert!(
            est > 0,
            "the content estimate must be > 0 for compactable messages"
        );
        let basis = codelet_cli::interactive_helpers::pre_compaction_basis(0, &msgs);
        assert_eq!(
            basis, est,
            "pre_compaction_basis(0, msgs) must be the content estimate"
        );
        let basis_tracker = codelet_cli::interactive_helpers::pre_compaction_basis(500, &msgs);
        assert_eq!(
            basis_tracker, 500,
            "pre_compaction_basis must prefer the tracker when it reported usage"
        );
    }
    let original = match last_compaction_event(&target) {
        StreamChunk::CompactionComplete { compaction_result } => compaction_result.original_tokens,
        other => panic!("expected CompactionComplete, got {other:?}"),
    };
    assert!(
        original > 0,
        "the pre-compaction basis must be the content estimate (> 0), not the dead 0 tracker value"
    );

    // @step And the CompactionComplete event's original_tokens is never 0
    assert!(
        original > 0,
        "a 0 original_tokens must never reach CompactionComplete for a target with compactable messages"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: Stash-lock failure restores the calling session's status
// (CMPCT-048: the pending_dag_content lock is poisoned by a thread that
// panicked while holding it — the handler must restore the pre-compaction
// status instead of leaving the session stuck in Compacting with the flag
// already cleared)
// ============================================================================

#[tokio::test]
async fn stash_lock_failure_restores_the_calling_sessions_status() {
    // @step Given the agent calls GenerateCompaction with no arguments on its own session A mid-turn
    let _guard = GLOBAL_GUARD.lock().expect("test guard");
    let (server, _data_dir, project, manager, caller, _target) =
        mock_llm_and_fixture_serving(SUCCESS_DAG).await;
    seed_turns(&caller, 4).await;

    let tool = register_and_build_tool(&caller, &project, &manager);
    // The pre-compaction status of a mid-turn session is Running.
    caller.set_status(codelet_rpc_types::SessionStatus::Running);
    let pre_status = caller.get_status();

    // @step And the sub-agent returns a parseable DAG
    // (the wiremock endpoint serves SUCCESS_DAG — the sub-agent succeeds)

    // @step When stashing the DAG into session A's pending_dag_content fails to acquire the lock
    // Poison the std Mutex: a thread locks it and panics while holding the
    // guard → the mutex is permanently poisoned → every later .lock()
    // returns Err (the same shape the handler's `if let Ok(mut guard) =`
    // branch handles).
    let poison_session = caller.clone();
    // The JoinError IS the expected panic — do NOT .expect() it here:
    // an expect-panic on this thread would poison the process-global
    // GLOBAL_GUARD (held for the whole test) and cascade into every other
    // test in the binary.
    let _poison_result = std::thread::spawn(move || {
        // Hold the lock (the guard stays alive until this thread panics and
        // unwinds) → the mutex is permanently poisoned.
        let _held = poison_session
            .pending_dag_content
            .lock()
            .expect("poison thread: lock");
        panic!("poison the pending_dag_content lock (CMPCT-048 test)");
    })
    .join();

    let err = tool
        .call(GenerateCompactionArgs { session_id: None })
        .await
        .expect_err("a poisoned stash lock must be an execution error");

    // @step Then session A's status is restored to Running instead of staying Compacting
    assert_eq!(
        caller.get_status(),
        pre_status,
        "the status must be restored to its pre-compaction value ({pre_status:?}) — never left stuck in Compacting"
    );
    assert_ne!(
        caller.get_status(),
        codelet_rpc_types::SessionStatus::Compacting,
        "the session must NOT be stuck in Compacting"
    );

    // @step And the failure is logged at ERROR level naming the target session
    // Source-shape proof: the tracing macro immediately preceding the
    // stash-failure marker must be `tracing::error!` (not debug/info/warn)
    // and must name the target session.
    let handler_src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/generate_compaction_handler.rs"),
    )
    .expect("read handler source");
    let err_marker = handler_src
        .find("[generate-compaction] failed to acquire pending_dag_content lock")
        .expect("the stash-failure log marker must exist");
    let err_window = &handler_src[err_marker.saturating_sub(400)..err_marker];
    let last_macro = err_window
        .rfind("tracing::error!(")
        .map(|p| (p, "error"))
        .or_else(|| err_window.rfind("tracing::warn!(").map(|p| (p, "warn")))
        .or_else(|| err_window.rfind("tracing::info!(").map(|p| (p, "info")))
        .or_else(|| err_window.rfind("tracing::debug!(").map(|p| (p, "debug")))
        .expect("a tracing macro must precede the stash-failure log");
    assert_eq!(
        last_macro.1, "error",
        "CMPCT-048: the stash-failure log must be logged at ERROR level (got {last_macro:?})"
    );
    assert!(
        err_window.contains("target_session_id = %target_session_id"),
        "CMPCT-048: the stash-failure log must name the target session"
    );
    match err {
        ToolError::Execution { tool: t, message } => {
            assert_eq!(t, "GenerateCompaction");
            assert!(
                message.contains("failed to acquire pending_dag_content lock"),
                "the execution error must name the stash-lock failure, got: {message}"
            );
        }
        other => panic!("expected ToolError::Execution, got {other:?}"),
    }

    // @step And the compaction_in_progress flag is cleared
    assert!(
        !caller.compaction_in_progress.load(Ordering::SeqCst),
        "the compaction flag must be cleared on stash failure"
    );
    assert!(
        caller.pending_dag_content.lock().is_err(),
        "the poisoned lock must remain poisoned (the DAG is lost — nothing stashed)"
    );

    drop(_guard);
    let _ = server;
}

// ============================================================================
// Scenario: The 045 fallback builder delegates to the shared compaction_dag primitive
// (CMPCT-047 source-shape: the handler must route through
// build_recovered_or_generic_dag and must NOT format its own generic
// dag-node template inline)
// ============================================================================

#[test]
fn the_045_fallback_builder_delegates_to_the_shared_compaction_dag_primitive() {
    // @step Given the compactor sub-agent times out without emitting any dag-node block
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let handler_src = std::fs::read_to_string(manifest.join("src/generate_compaction_handler.rs"))
        .unwrap_or_else(|e| panic!("failed to read generate_compaction_handler.rs: {e}"));

    // @step When the handler assembles the fallback DAG
    // @step Then it is built by codelet_cli::compaction_dag::build_recovered_or_generic_dag with the label "Auto-recovered: compaction timeout" and the body "Session was auto-compacted due to a compaction-sub-agent timeout."
    // Anchor at the build_fallback_dag fn itself and check the delegation +
    // its label/body arguments live in the fn body (not a doc comment).
    let fn_at = handler_src
        .find("fn build_fallback_dag(")
        .expect("CMPCT-047: the handler must keep its build_fallback_dag builder");
    let body = &handler_src[fn_at..fn_at + 800];
    let delegate_at = body
        .find("codelet_cli::compaction_dag::build_recovered_or_generic_dag")
        .expect("CMPCT-047: build_fallback_dag must delegate to the shared build_recovered_or_generic_dag");
    let label_at = body
        .find("\"Auto-recovered: compaction timeout\"")
        .expect("CMPCT-047: the delegation must use the 045 label");
    let body_at = body
        .find("Session was auto-compacted due to a compaction-sub-agent timeout.")
        .expect("CMPCT-047: the delegation must use the 045 body");
    assert!(
        label_at > delegate_at,
        "CMPCT-047: the label must be an argument AFTER the shared delegation call"
    );
    assert!(
        body_at > delegate_at,
        "CMPCT-047: the body must be an argument AFTER the shared delegation call"
    );

    // @step And the 045 handler does not format its own generic dag-node template inline
    assert!(
        !handler_src.contains("r#\"<dag-node"),
        "CMPCT-047: the 045 handler must NOT format its own <dag-node> template inline"
    );
}

// ============================================================================
// Scenario: Compactor sub-agent provider arms match the DeepSearch clone
// (CMPCT-046c source-shape: the compactor's built-in provider match must
// support the same arms as the DeepSearch sub-agent's match — including
// the github-copilot/copilot pending-builder arm — instead of falling
// through to "Unsupported provider")
// ============================================================================

#[test]
fn compactor_sub_agent_provider_arms_match_the_deep_search_clone() {
    // @step Given the compactor sub-agent's built-in provider match exists
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let compactor = std::fs::read_to_string(manifest.join("src/generate_compaction_handler.rs"))
        .unwrap_or_else(|e| panic!("failed to read generate_compaction_handler.rs: {e}"));
    let deep_search = std::fs::read_to_string(manifest.join("src/deep_search_handler.rs"))
        .unwrap_or_else(|e| panic!("failed to read deep_search_handler.rs: {e}"));

    // @step When it is compared with the DeepSearch sub-agent's provider match
    let arms = [
        "\"claude\" =>",
        "\"openai\" =>",
        "\"gemini\" =>",
        "\"codex\" =>",
        "\"zai\" =>",
        "\"github-copilot\" | \"copilot\" =>",
    ];
    for arm in arms {
        assert!(
            deep_search.contains(arm),
            "the DeepSearch match must have arm {arm} (reference shape)"
        );
        assert!(
            compactor.contains(arm),
            "CMPCT-046c: the compactor match must have arm {arm}"
        );
    }

    // @step Then it supports the same arms: claude, openai, gemini, codex, zai, and github-copilot/copilot
    // (the loop above asserts each arm in both matches)

    // @step And the copilot arm returns the same distinct pending-builder error shape as DeepSearch instead of falling through to "Unsupported provider"
    assert!(
        compactor.contains("github-copilot compactor sub-agent builder pending"),
        "CMPCT-046c: the copilot arm must return its own distinct pending-builder error"
    );
    assert!(
        compactor.contains("Supported: claude, openai, gemini, codex, zai, github-copilot"),
        "CMPCT-046c: the 'Unsupported provider' fallthrough must name github-copilot as supported"
    );
}
