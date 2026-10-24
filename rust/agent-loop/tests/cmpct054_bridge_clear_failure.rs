#[path = "support/terminal_recovery.rs"]
mod support;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::{SessionState, StreamChunk};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_failed_bridge_clear_reports_the_durable_reset_failure() {
    // @step Given a registered bridge control with committed messages and provider state
    let (directory, session, input_rx) = support::fixture();
    let server = MockServer::start().await;
    let entered = Arc::new(Notify::new());
    let provider_entered = entered.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &Request| {
            provider_entered.notify_one();
            ResponseTemplate::new(200).set_delay(Duration::from_secs(60))
        })
        .mount(&server)
        .await;
    std::env::set_var("OPENAI_API_KEY", "test-no-network");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    {
        let mut inner = session.inner.lock().await;
        inner.provider_manager_mut().set_facade_override(None);
        inner
            .provider_manager_mut()
            .set_model_direct("openai", "o3", Some(100_000), Some(4096), None)
            .expect("admit selected provider request");
    }
    let (_injection_tx, injection_rx) = mpsc::channel(4);
    let task = tokio::spawn(agent_loop(session.clone(), input_rx, injection_rx));
    session
        .send_input("preserve this".into(), None)
        .expect("queue turn");
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .expect("provider request reached registered bridge boundary");
    let control = codelet_tools::get_bridge_session_context(session.id)
        .expect("registered bridge context")
        .control_handler
        .clone()
        .expect("real bridge control handler");
    task.abort();
    let _ = task.await;
    let before = {
        let mut inner = session.inner.lock().await;
        inner.provider_session.ensure_provider("codex");
        inner
            .bind_provider_persistence(session.id)
            .expect("real checkpoint writer");
        serde_json::json!({"messages": inner.messages, "tokens": inner.token_tracker})
    };
    let original_path = directory.path().join("messages/messages.jsonl");
    let original_bytes = std::fs::read(&original_path).expect("immutable original bytes");
    // @step And the provider checkpoint store becomes unavailable
    std::fs::remove_file(&original_path).expect("remove original store path");
    std::fs::create_dir(&original_path).expect("make store path unavailable as a file");
    codelet_core::persistence::reset_stores_for_tests();
    let mut events = session.subscribe_to_stream();
    // @step When the bridge requests conversation clearing
    tokio::task::block_in_place(|| control("clear", None));
    // @step Then an explicit clear failure is emitted without a cleared or completion event
    let mut failures = Vec::new();
    while let Ok(event) = events.try_recv() {
        assert!(
            !matches!(
                event,
                StreamChunk::CompactionComplete { .. }
                    | StreamChunk::SessionStateChange {
                        state: SessionState::Cleared
                    }
            ),
            "failed reset must not publish conversation replacement"
        );
        if let StreamChunk::Error { error } = event {
            failures.push(error);
        }
    }
    assert_eq!(
        failures.len(),
        1,
        "durable reset failure must reach the bridge observer"
    );
    assert!(failures[0].contains("clear"));
    // @step And the committed messages and counters remain unchanged
    let inner = session.inner.lock().await;
    assert_eq!(
        serde_json::json!({"messages": inner.messages, "tokens": inner.token_tracker}),
        before
    );
    std::fs::remove_dir(&original_path).expect("remove unavailable store fixture");
    std::fs::write(&original_path, original_bytes).expect("restore original store bytes");
    codelet_tools::remove_bridge_session_context(session.id);
}
