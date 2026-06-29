// Feature: spec/features/agent-manager-owning-session-manager.feature
//
// RPC-386 — AgentManager handler binds to the daemon-owned SessionManager
// (the one that created the spawner session) instead of the global
// SessionManager::instance() singleton.
//
// ====================================================================
// RED PHASE / ASSUMED-API NOTE (read before "fixing" a compile error)
// ====================================================================
// These tests drive the BEHAVIOUR the RPC-386 fix must introduce:
// dependency-injecting the owning SessionManager into the AgentManager
// sync + async handlers. The injection seam does NOT exist yet, so the
// tests are written against the proposed (not-yet-implemented) API:
//
//   * `create_handler(owning_manager: Option<Arc<SessionManager>>, ...)`
//       — when `Some(M)`, the handler operates on M; when `None`, it
//         falls back to `SessionManager::instance()` (NAPI parity).
//   * `create_async_handler(owning_manager: Option<Arc<SessionManager>>)`
//       — same injection for the async path (await_idle / profile).
//
// Because the current `create_handler` takes NO manager argument (it
// hardcodes `SessionManager::instance()` at line 43) and
// `create_async_handler` takes none (hardcodes `instance()` at line 909),
// every test in this file currently FAILS TO COMPILE — that is the
// intended red signal. The supervisor should confirm this exact API
// shape before the implementation step. If a different injection
// mechanism is chosen (e.g. a `register_agent_manager_handler_with_manager`
// wrapper, or a `BackgroundSession::owning_manager()` resolved inside the
// existing factory), only the two `create_*` call sites below need to be
// re-pointed at it — the assertions encode the behaviour, not the plumbing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Each test serialises on a process-global Mutex (GLOBAL_GUARD) to fence the
// shared data dir + singleton; the guard is intentionally held across awaits.
#![allow(clippy::await_holding_lock)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use codelet_agent_loop::agent_manager_handler::{create_async_handler, create_handler};
use codelet_sessions::SessionManager;
// `execute_agent_manager`/`execute_agent_manager_async` are only `pub` inside
// the `agent_manager::handler` submodule (they are not re-exported at the crate
// root the way `set_*`/`clear_*` are), so reach them via the full module path.
use codelet_tools::agent_manager::handler::{execute_agent_manager, execute_agent_manager_async};
use codelet_tools::{
    clear_all_agent_manager_handlers, set_agent_manager_async_handler, set_agent_manager_handler,
    AgentManagerAction, AgentManagerResult, AwaitOutcome, SessionIdParam,
};
use uuid::Uuid;

/// Trimmed offline models.dev catalog (anthropic/openai/google), shared with
/// the sessions-crate PROV-101 / RPC-385 tests. Seeding it into the temp data
/// dir's cache keeps registry validation fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises every test that swaps the process-global data directory and/or
/// touches the process-global AgentManager handler registry + the
/// `SessionManager::instance()` singleton, so parallel tests never observe each
/// other's sessions. Mirrors the PROV-118/119 + RPC-385 `DATA_DIR_GUARD`
/// precedent.
static GLOBAL_GUARD: Mutex<()> = Mutex::new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a non-singleton `SessionManager` rooted in a fresh data dir whose
/// model cache is pre-seeded from the offline fixture, with the default model
/// set so `create_session*` succeeds (the PROV-101 decline never fires).
///
/// This returns its OWN `Arc<SessionManager>` — deliberately NOT
/// `SessionManager::instance()` — so the tests can prove a spawn landed in
/// the owning manager and NOT in the global singleton.
fn owning_manager() -> Result<(tempfile::TempDir, Arc<SessionManager>), String> {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    // ASSUMED API: the fix wraps the manager in Arc via a constructor that
    // stamps a `Weak<Self>` self-reference so created sessions can carry an
    // owning-manager back-reference. `Arc::new(SessionManager::new())` is the
    // current shape and is used here; if the fix adds `SessionManager::new_arc()`
    // swap this single line.
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-opus-4-5");
    Ok((data_dir, manager))
}

/// Create a spawner session inside manager `M` and register an AgentManager
/// handler BOUND TO M (not the singleton). Returns the spawner's UUID.
///
/// This is the heart of the RPC-386 fix surface: the handler must be wired to
/// the manager that created the spawner. We construct the handler via
/// `create_handler(Some(M.clone()), ...)` — the proposed injected signature.
async fn spawner_with_handler_bound_to(
    manager: &Arc<SessionManager>,
    project: &str,
) -> Result<Uuid, String> {
    let spawner_id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &spawner_id.to_string(),
            "anthropic/claude-opus-4-5",
            project,
            "spawner",
        )
        .await?;

    // ASSUMED API: `create_handler` gains an owning-manager first argument.
    let handler = create_handler(
        Some(manager.clone()),
        project.to_string(),
        Some("anthropic/claude-opus-4-5".to_string()),
        None,
        None,
    );
    set_agent_manager_handler(spawner_id, Some(handler));
    Ok(spawner_id)
}

// =============================================================================
// Scenario: Spawn creates the subordinate in the owning manager, not the
//           global singleton
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawn_creates_subordinate_in_owning_manager_not_singleton() -> Result<(), String> {
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // @step Given a daemon-owned SessionManager M that is not the global singleton
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    let (_data_dir, manager) = owning_manager()?;
    assert!(
        !std::ptr::eq(manager.as_ref(), SessionManager::instance()),
        "precondition: M must be a distinct manager from the global singleton"
    );

    // @step And a spawner session created by M with an AgentManager handler bound to M
    let spawner_id = spawner_with_handler_bound_to(&manager, &project).await?;

    // @step When the spawner invokes the AgentManager spawn action
    let result = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match result {
        AgentManagerResult::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };

    // @step Then the subordinate appears in M.list_sessions()
    let in_m = manager
        .list_sessions()
        .into_iter()
        .any(|s| s.id == subordinate_id);
    assert!(
        in_m,
        "subordinate {subordinate_id} must appear in the owning manager M.list_sessions()"
    );

    // @step And the subordinate does not appear in SessionManager::instance().list_sessions()
    let in_singleton = SessionManager::instance()
        .list_sessions()
        .into_iter()
        .any(|s| s.id == subordinate_id);
    assert!(
        !in_singleton,
        "subordinate {subordinate_id} must NOT leak into the global singleton"
    );

    clear_all_agent_manager_handlers();
    Ok(())
}

// =============================================================================
// Scenario: Spawn fires the owning manager's session_created broadcast
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawn_fires_owning_manager_session_created_broadcast() -> Result<(), String> {
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // @step Given a daemon-owned SessionManager M that is not the global singleton
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    let (_data_dir, manager) = owning_manager()?;

    // @step And a spawner session created by M with an AgentManager handler bound to M
    let spawner_id = spawner_with_handler_bound_to(&manager, &project).await?;

    // @step And a subscriber to M.session_created_tx()
    // Subscribe AFTER the spawner session is created so the only event we can
    // receive is the subordinate's.
    let mut created_rx = manager.session_created_tx().subscribe();

    // @step When the spawner invokes the AgentManager spawn action
    let result = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match result {
        AgentManagerResult::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };

    // @step Then the subscriber receives the subordinate's SessionInfo
    let received = tokio::time::timeout(Duration::from_millis(500), created_rx.recv())
        .await
        .map_err(|_| "timed out waiting for session-created broadcast on M".to_string())?
        .map_err(|e| format!("broadcast recv error: {e}"))?;
    assert_eq!(
        received.id, subordinate_id,
        "M.session_created_tx() must carry the subordinate's SessionInfo"
    );

    clear_all_agent_manager_handlers();
    Ok(())
}

// =============================================================================
// Scenario: A subordinate spawned into a manager with real hooks runs its
//           agent loop
// =============================================================================
//
// This scenario proves the subordinate is ALIVE (not a dead session object in
// the Noop-hooks singleton): with FspecAgentHooks installed on M, the
// subordinate's `spawn_agent_loop` must fire so a follow-up message is
// processed and produces output chunks. We use the deterministic stub
// LlmProvider (gated behind the `test-support` feature, matching
// rpc072_work_agent_roundtrip.rs) so the round-trip is hermetic and offline.
//
// The seam asserted: the subordinate created by the AgentManager spawn — bound
// to M — subscribes to ITS OWN stream and, after a `message`, emits a
// StreamChunk::Text. If the spawn had landed in the Noop-hooks singleton (the
// bug), spawn_agent_loop would be a no-op and no Text chunk would ever arrive.
#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subordinate_spawned_into_real_hooks_manager_runs_agent_loop() -> Result<(), String> {
    use codelet_agent_loop::FspecAgentHooks;
    use codelet_rpc_types::StreamChunk;

    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // @step Given a daemon-owned SessionManager M whose hooks start the agent loop
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    let (_data_dir, manager) = owning_manager()?;
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

    // @step And a spawner session created by M with an AgentManager handler bound to M
    let spawner_id = Uuid::new_v4();
    manager
        .create_session_with_id(&spawner_id.to_string(), "stub/canned", &project, "spawner")
        .await?;
    // ASSUMED API: injected owning-manager argument on create_handler.
    let handler = create_handler(
        Some(manager.clone()),
        project.clone(),
        Some("stub/canned".to_string()),
        None,
        None,
    );
    set_agent_manager_handler(spawner_id, Some(handler));

    // @step When the spawner spawns a subordinate and sends it a follow-up message
    let spawned = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match spawned {
        AgentManagerResult::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };

    let subordinate = manager
        .get_session(&subordinate_id)
        .map_err(|e| format!("subordinate must live in M: {e}"))?;
    let mut sub_chunks = subordinate.subscribe_to_stream();

    let delivered = execute_agent_manager(
        spawner_id,
        AgentManagerAction::Message {
            session_id: subordinate_id.clone(),
            message: "hello".to_string(),
            context: None,
        },
    );
    match delivered {
        AgentManagerResult::MessageDelivered { delivered, .. } => {
            assert!(
                delivered,
                "message must be delivered to the live subordinate"
            );
        }
        other => panic!("expected MessageDelivered, got: {other:?}"),
    }

    // @step Then the subordinate processes the message and emits output chunks
    let mut got_text = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(250), sub_chunks.recv()).await {
            Ok(Ok(StreamChunk::Text { text, .. })) => {
                if text == "hi back" {
                    got_text = true;
                    break;
                }
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => continue,
        }
    }
    assert!(
        got_text,
        "subordinate's agent loop must run (FspecAgentHooks on M) and emit a Text chunk; \
         if it landed in the Noop-hooks singleton it would stay silent"
    );

    clear_all_agent_manager_handlers();
    Ok(())
}

// =============================================================================
// Scenario: Spawner can list, get status, and close the subordinate on the
//           owning manager
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawner_can_list_get_status_and_close_on_owning_manager() -> Result<(), String> {
    use codelet_tools::AgentManagerResult as R;

    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // @step Given a daemon-owned SessionManager M that is not the global singleton
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    let (_data_dir, manager) = owning_manager()?;

    // @step And a spawner session created by M with an AgentManager handler bound to M
    let spawner_id = spawner_with_handler_bound_to(&manager, &project).await?;

    // @step When the spawner spawns a subordinate via the AgentManager spawn action
    let spawned = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match spawned {
        R::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };

    // @step Then the AgentManager list action returns the subordinate id
    let listed = execute_agent_manager(spawner_id, AgentManagerAction::List);
    match listed {
        R::Listed { sessions } => {
            assert!(
                sessions.iter().any(|s| s.session_id == subordinate_id),
                "list (resolved on M) must include the subordinate id"
            );
        }
        other => panic!("expected Listed, got: {other:?}"),
    }

    // @step And the AgentManager get_status action resolves the subordinate on M
    let status = execute_agent_manager(
        spawner_id,
        AgentManagerAction::GetStatus {
            session_id: subordinate_id.clone(),
        },
    );
    match status {
        R::Status(s) => {
            assert_eq!(
                s.session_id, subordinate_id,
                "get_status must resolve the subordinate on M (not error from the singleton)"
            );
        }
        other => panic!("expected Status, got: {other:?}"),
    }

    // @step And the chain-of-command on M records the spawner as the subordinate's supervisor
    let sub_uuid = Uuid::parse_str(&subordinate_id).map_err(|e| e.to_string())?;
    let supervisors = manager.get_supervisors(sub_uuid);
    assert!(
        supervisors.contains(&spawner_id),
        "M's ChainOfCommand must record the spawner as the subordinate's supervisor"
    );

    // @step And the AgentManager close action removes the subordinate from M
    let closed = execute_agent_manager(
        spawner_id,
        AgentManagerAction::Close {
            session_id: subordinate_id.clone(),
        },
    );
    match closed {
        R::Closed { closed, .. } => assert!(closed, "close must succeed"),
        other => panic!("expected Closed, got: {other:?}"),
    }
    let still_in_m = manager
        .list_sessions()
        .into_iter()
        .any(|s| s.id == subordinate_id);
    assert!(
        !still_in_m,
        "after close, the subordinate must be removed from M.list_sessions()"
    );

    clear_all_agent_manager_handlers();
    Ok(())
}

// =============================================================================
// Scenario: NAPI path falls back to the global singleton when no owning
//           manager is set
// =============================================================================
//
// Simulates the NAPI path: the AgentManager handler is constructed with NO
// owning-manager back-reference (`None`). In that case the handler MUST resolve
// and create the subordinate on `SessionManager::instance()` — byte-for-byte
// the legacy behaviour, so existing NAPI/AMGR suites stay green.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn napi_path_falls_back_to_singleton_when_no_owning_manager() -> Result<(), String> {
    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // Seed an offline data dir so the singleton's create_session* validates
    // against the registry offline. The singleton needs a default model set
    // (it is a process-global instance shared across tests).
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    SessionManager::instance().set_default_model("anthropic/claude-opus-4-5");

    // @step Given a spawner session with no owning-manager back-reference set
    // The spawner lives in the singleton (the NAPI invariant) and the handler
    // is bound with `None` for the owning manager.
    let spawner_id = Uuid::new_v4();
    SessionManager::instance()
        .create_session_with_id(
            &spawner_id.to_string(),
            "anthropic/claude-opus-4-5",
            &project,
            "napi-spawner",
        )
        .await?;
    // ASSUMED API: `None` selects the singleton-fallback path.
    let handler = create_handler(
        None,
        project.clone(),
        Some("anthropic/claude-opus-4-5".to_string()),
        None,
        None,
    );
    set_agent_manager_handler(spawner_id, Some(handler));

    // @step When the spawner invokes the AgentManager spawn action
    let spawned = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match spawned {
        AgentManagerResult::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };

    // @step Then the subordinate is created on SessionManager::instance()
    let in_singleton = SessionManager::instance()
        .list_sessions()
        .into_iter()
        .any(|s| s.id == subordinate_id);
    assert!(
        in_singleton,
        "with no owning manager, the subordinate must be created on the singleton (NAPI parity)"
    );

    // Cleanup: remove the sessions we created on the shared singleton.
    let _ = SessionManager::instance().destroy_session(&subordinate_id);
    let _ = SessionManager::instance().destroy_session(&spawner_id.to_string());
    clear_all_agent_manager_handlers();
    Ok(())
}

// =============================================================================
// Scenario: await_idle resolves the subordinate on the owning manager
// =============================================================================
//
// The async handler path (await_idle / profile) currently hardcodes
// `SessionManager::instance()` at line 909. This test pins that the async
// handler, when bound to M, resolves the subordinate on M: a subordinate that
// is already idle in M must make await_idle return immediately with an Idle
// outcome. If the async handler looked at the singleton (the bug), the
// subordinate would be "session_not_found" there and await_idle would error
// instead of returning Idle.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn await_idle_resolves_subordinate_on_owning_manager() -> Result<(), String> {
    use codelet_rpc_types::SessionStatus as RpcSessionStatus;
    use codelet_tools::AgentManagerResult as R;

    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // @step Given a daemon-owned SessionManager M whose hooks start the agent loop
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    let (_data_dir, manager) = owning_manager()?;

    // @step And a spawner session created by M with an AgentManager handler bound to M
    let spawner_id = spawner_with_handler_bound_to(&manager, &project).await?;
    // ASSUMED API: the async handler also takes the owning-manager argument.
    let async_handler = create_async_handler(Some(manager.clone()));
    set_agent_manager_async_handler(spawner_id, Some(async_handler));

    // @step And the spawner has spawned a subordinate that becomes idle
    let spawned = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match spawned {
        R::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };
    // Force the subordinate to idle so await_idle can resolve immediately
    // (no real LLM loop needed for this seam).
    let subordinate = manager
        .get_session(&subordinate_id)
        .map_err(|e| format!("subordinate must live in M: {e}"))?;
    subordinate.set_status(RpcSessionStatus::Idle);

    // @step When the spawner invokes the AgentManager await_idle action for the subordinate
    let result = execute_agent_manager_async(
        spawner_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(subordinate_id.clone()),
            timeout: Some(2),
        },
    )
    .await;

    // @step Then await_idle blocks on the subordinate in M and returns its idle status
    match result {
        R::AwaitResult { results } => {
            assert_eq!(results.len(), 1, "exactly one await result expected");
            assert_eq!(results[0].session_id, subordinate_id);
            assert_eq!(
                results[0].status,
                AwaitOutcome::Idle,
                "await_idle must resolve the subordinate on M and report Idle"
            );
        }
        other => panic!("expected AwaitResult, got: {other:?}"),
    }

    clear_all_agent_manager_handlers();
    Ok(())
}

// =============================================================================
// Scenario: set_role and message resolve the subordinate on the owning manager
// =============================================================================
//
// RPC-386 review round: pins that BOTH synchronous AgentManager actions
// `SetRole` and `Message` resolve the target subordinate on the owning manager
// M (via the injected handler), never on `SessionManager::instance()`. With
// real FspecAgentHooks installed on M, the subordinate is ALIVE: `set_role`
// mutates the role on the session that lives in M, and a subsequent `message`
// is delivered to that same in-M session and processed by its agent loop (it
// emits a Text chunk). If either action looked at the singleton, the
// subordinate would be `session_not_found` there and these assertions would
// fail. We additionally assert the subordinate is absent from the singleton so
// "resolved on M" cannot be satisfied accidentally by the global instance.
#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_role_and_message_resolve_subordinate_on_owning_manager() -> Result<(), String> {
    use codelet_agent_loop::FspecAgentHooks;
    use codelet_rpc_types::StreamChunk;
    use codelet_tools::AgentManagerResult as R;

    let _guard = GLOBAL_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_all_agent_manager_handlers();

    // @step Given a daemon-owned SessionManager M whose hooks start the agent loop
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = tmp.path().to_str().expect("utf8 tempdir").to_string();
    let (_data_dir, manager) = owning_manager()?;
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

    // @step And a spawner session created by M with an AgentManager handler bound to M
    let spawner_id = Uuid::new_v4();
    manager
        .create_session_with_id(&spawner_id.to_string(), "stub/canned", &project, "spawner")
        .await?;
    let handler = create_handler(
        Some(manager.clone()),
        project.clone(),
        Some("stub/canned".to_string()),
        None,
        None,
    );
    set_agent_manager_handler(spawner_id, Some(handler));

    // @step And the spawner has spawned a subordinate via the AgentManager spawn action
    let spawned = execute_agent_manager(spawner_id, AgentManagerAction::Spawn { role: None });
    let subordinate_id = match spawned {
        R::Spawned { session_id } => session_id,
        other => panic!("expected Spawned, got: {other:?}"),
    };

    // Subscribe to the subordinate's OWN stream (resolved on M) before sending
    // the message, so we can prove the in-M session actually processes it.
    let subordinate = manager
        .get_session(&subordinate_id)
        .map_err(|e| format!("subordinate must live in M: {e}"))?;
    let mut sub_chunks = subordinate.subscribe_to_stream();

    // @step When the spawner invokes the AgentManager set_role action for the subordinate
    let role_text = "You are a focused reviewer.".to_string();
    let role_set = execute_agent_manager(
        spawner_id,
        AgentManagerAction::SetRole {
            session_id: Some(subordinate_id.clone()),
            role: role_text.clone(),
        },
    );
    match role_set {
        R::RoleSet { session_id, role } => {
            assert_eq!(
                session_id, subordinate_id,
                "RoleSet must target the subordinate"
            );
            assert_eq!(
                role,
                Some(role_text.clone()),
                "RoleSet must echo the new role"
            );
        }
        other => panic!("expected RoleSet, got: {other:?}"),
    }

    // @step And the spawner invokes the AgentManager message action for the subordinate
    let delivered = execute_agent_manager(
        spawner_id,
        AgentManagerAction::Message {
            session_id: subordinate_id.clone(),
            message: "hello".to_string(),
            context: None,
        },
    );

    // @step Then the role is applied to the subordinate in M
    let in_m_role = manager
        .get_session(&subordinate_id)
        .map_err(|e| format!("subordinate must live in M: {e}"))?
        .get_role();
    assert_eq!(
        in_m_role,
        Some(role_text),
        "set_role must mutate the subordinate that lives in M, not the singleton"
    );
    assert!(
        SessionManager::instance()
            .get_session(&subordinate_id)
            .is_err(),
        "the subordinate must NOT exist in the global singleton — resolution is on M"
    );

    // @step And the message is delivered to the subordinate in M
    match delivered {
        R::MessageDelivered {
            delivered,
            session_id,
        } => {
            assert!(
                delivered,
                "message must be delivered to the live subordinate resolved on M"
            );
            assert_eq!(session_id, subordinate_id);
        }
        other => panic!("expected MessageDelivered, got: {other:?}"),
    }
    // Prove the in-M subordinate actually processed the delivered message: its
    // agent loop (FspecAgentHooks on M) emits a Text chunk on its own stream.
    let mut got_text = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(250), sub_chunks.recv()).await {
            Ok(Ok(StreamChunk::Text { text, .. })) => {
                if text == "hi back" {
                    got_text = true;
                    break;
                }
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => continue,
        }
    }
    assert!(
        got_text,
        "the message delivered to the in-M subordinate must be processed by its agent loop; \
         if it resolved on the Noop-hooks singleton it would stay silent"
    );

    clear_all_agent_manager_handlers();
    Ok(())
}
