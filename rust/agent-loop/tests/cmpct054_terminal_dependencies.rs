//! Feature: spec/features/terminal-compaction-dependency-lifetime.feature

#[path = "support/terminal_recovery.rs"]
mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_cli::compactor_sub_agent::{
    has_compactor_sub_agent_handler, has_terminal_overflow_owner, set_compactor_sub_agent_handler,
};
use codelet_rpc_types::StreamChunk;
use serial_test::serial;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_terminal_recovery_invokes_the_turn_builder_before_cleanup() {
    // @step Given a background session with compactable history and an overflowing terminal error
    let (_directory, session, input_rx) = support::fixture();
    let server = wiremock::MockServer::start().await;
    let builder_target = format!("Target session: {}", session.id);
    let expected_target = builder_target.clone();
    let terminal_dependencies = Arc::new(AtomicBool::new(false));
    let captured_dependencies = terminal_dependencies.clone();
    let session_id = session.id;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |request: &wiremock::Request| {
            let error = if String::from_utf8_lossy(&request.body).contains(&builder_target) {
                captured_dependencies.store(
                    !codelet_tools::has_fspec_handler_for_session(session_id)
                        && has_compactor_sub_agent_handler(session_id)
                        && codelet_tools::has_generate_compaction_handler(session_id)
                        && codelet_tools::has_inject_summary_handler(session_id),
                    Ordering::Release,
                );
                serde_json::json!({"error":{"message":"terminal builder deliberately rejected"}})
            } else {
                serde_json::json!({"error":{
                    "code":"context_length_exceeded",
                    "message":"ordinary request exceeds the context window"
                }})
            };
            wiremock::ResponseTemplate::new(400).set_body_json(error)
        })
        .mount(&server)
        .await;
    std::env::set_var("OPENAI_API_KEY", "test-no-network");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    let before = {
        let mut inner = session.inner.lock().await;
        inner
            .messages
            .push(rig::message::Message::user("committed evidence"));
        inner
            .messages
            .push(rig::message::Message::assistant("committed response"));
        inner.provider_manager_mut().set_facade_override(None);
        inner
            .provider_manager_mut()
            .set_model_direct("openai", "o3", Some(100_000), Some(4096), None)
            .expect("admit the physical request that the provider rejects");
        serde_json::to_value(&inner.messages).expect("snapshot")
    };
    let mut chunks = session.subscribe_to_stream();
    let (_injection_tx, injection_rx) = mpsc::channel(4);
    // @step When the real outer loop resolves the terminal error
    let task = tokio::spawn(agent_loop(session.clone(), input_rx, injection_rx));
    session
        .send_input("resolve failure".into(), None)
        .expect("queue input");
    let error = tokio::time::timeout(Duration::from_secs(5), async {
        let mut terminal_error = None;
        loop {
            match chunks.recv().await.expect("stream open") {
                StreamChunk::Error { error, .. } => terminal_error = Some(error),
                StreamChunk::Done { .. } => break terminal_error.expect("terminal failure"),
                StreamChunk::CompactionComplete { .. } => {
                    return "unexpected compaction completion".into();
                }
                _ => {}
            }
        }
    })
    .await;
    task.abort();
    let _ = task.await;
    // @step Then the registered turn builder runs before registration cleanup
    let error = error.expect("terminal resolution terminates");
    assert!(
        error.contains("terminal builder deliberately rejected"),
        "builder failure: {error}"
    );
    let requests = server.received_requests().await.expect("builder requests");
    assert_eq!(requests.len(), 2, "ordinary rejection followed by builder");
    // @step And the physical request belongs to the captured target builder rather than ordinary inference
    let request = String::from_utf8_lossy(&requests[1].body);
    assert!(
        request.contains(&expected_target),
        "terminal resolution must invoke the captured builder, not ordinary inference: {request}"
    );
    // @step And ordinary turn handlers are released while compaction dependencies remain registered
    assert!(
        terminal_dependencies.load(Ordering::Acquire),
        "only the outer terminal decision retains compaction dependencies after ordinary cleanup"
    );
    assert!(!has_compactor_sub_agent_handler(session.id));
    assert!(!codelet_tools::has_generate_compaction_handler(session.id));
    assert!(!codelet_tools::has_inject_summary_handler(session.id));
    // @step And its failure leaves committed history unchanged without completion
    assert_eq!(
        serde_json::to_value(&session.inner.lock().await.messages).expect("snapshot"),
        before
    );
    assert!(
        session
            .inner
            .lock()
            .await
            .ensure_compaction_inference_ready()
            .is_err(),
        "failed terminal recovery must not reopen inference"
    );
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(chunk, StreamChunk::CompactionComplete { .. }));
    }
}

#[tokio::test]
#[serial]
async fn scenario_missing_recovery_dependencies_fail_without_replacing_context() {
    // @step Given committed history without a registered compaction builder
    let (_directory, session, _input_rx) = support::fixture();
    let mut inner = session.inner.lock().await;
    inner
        .messages
        .push(rig::message::Message::user("committed evidence"));
    inner
        .messages
        .push(rig::message::Message::assistant("committed response"));
    let before = serde_json::to_value(&inner.messages).expect("snapshot");
    drop(inner);
    let mut chunks = session.subscribe_to_stream();
    // @step When terminal overflow recovery is requested
    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
            session.clone(),
            &support::overflow_error(),
        ),
    )
    .await
    .expect("recovery terminates");
    // @step Then recovery returns a typed unavailable failure
    assert!(
        matches!(
            outcome,
            Err(codelet_agent_loop::terminal_overflow_recovery::TerminalRecoveryError::Unavailable)
        ),
        "unavailable: {outcome:?}"
    );
    // @step And committed history remains unchanged without completion
    assert_eq!(
        serde_json::to_value(&session.inner.lock().await.messages).expect("snapshot"),
        before
    );
    // @step And inference remains blocked until the selected operation is resolved
    assert!(
        session
            .inner
            .lock()
            .await
            .ensure_compaction_inference_ready()
            .is_err(),
        "missing terminal dependencies must not reopen inference"
    );
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(chunk, StreamChunk::CompactionComplete { .. }));
    }
}

#[tokio::test]
#[serial]
async fn scenario_cancellation_releases_the_operation_registration() {
    // @step Given an operation-owned compaction builder registration
    let session_id = uuid::Uuid::new_v4();
    let entered = Arc::new(tokio::sync::Notify::new());
    let registered = entered.clone();
    let task = tokio::spawn(async move {
        set_compactor_sub_agent_handler(
            session_id,
            Some(Arc::new(|_, _| {
                Box::pin(async { Err("selected builder failed".into()) })
            })),
        );
        let _owner =
            codelet_agent_loop::terminal_overflow_recovery::CompactorRegistration::new(session_id);
        registered.notify_one();
        std::future::pending::<()>().await;
    });
    entered.notified().await;
    assert!(has_compactor_sub_agent_handler(session_id));
    assert!(has_terminal_overflow_owner(session_id));
    // @step When its owning task is cancelled
    task.abort();
    let _ = task.await;
    // @step Then the builder registration is removed
    assert!(!has_compactor_sub_agent_handler(session_id));
    assert!(!has_terminal_overflow_owner(session_id));
    // @step And a later operation can register and release its own builder
    set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(|_, _| {
            Box::pin(async { Err("new operation".into()) })
        })),
    );
    let owner =
        codelet_agent_loop::terminal_overflow_recovery::CompactorRegistration::new(session_id);
    assert!(has_compactor_sub_agent_handler(session_id));
    drop(owner);
    assert!(!has_compactor_sub_agent_handler(session_id));
    assert!(!has_terminal_overflow_owner(session_id));
}
