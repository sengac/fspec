//! Feature: spec/features/aborted-turn-compaction-dependency-cleanup.feature

#[path = "support/terminal_recovery.rs"]
pub mod support;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler;
use codelet_rpc_types::StreamChunk;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_aborting_a_real_turn_releases_every_compaction_dependency() {
    // @step Given the real outer loop waiting for a provider with registered compaction dependencies
    let (_directory, session, input_rx) = support::fixture();
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
    let before = {
        let mut inner = session.inner.lock().await;
        inner.messages = vec![
            rig::message::Message::user("committed evidence"),
            rig::message::Message::assistant("committed response"),
        ];
        inner.provider_manager_mut().set_facade_override(None);
        inner
            .provider_manager_mut()
            .set_model_direct("openai", "o3", Some(100_000), Some(4096), None)
            .expect("admit the selected streaming request");
        serde_json::to_value(&inner.messages).expect("original context")
    };
    let mut events = session.subscribe_to_stream();
    let (_injection_tx, injection_rx) = mpsc::channel(4);
    let task = tokio::spawn(agent_loop(session.clone(), input_rx, injection_rx));
    session
        .send_input("recover".into(), None)
        .expect("queue turn");
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .expect("real provider request starts");
    assert!(has_compactor_sub_agent_handler(session.id));
    assert!(codelet_tools::has_generate_compaction_handler(session.id));
    assert!(codelet_tools::has_inject_summary_handler(session.id));
    let durable_before = serde_json::to_value(
        codelet_core::persistence::load_session(session.id).expect("admitted turn originals"),
    )
    .expect("durable snapshot after ordinary input persistence");
    // @step When the owning agent task is aborted after the provider request starts
    task.abort();
    let _ = task.await;
    // @step Then the builder and candidate submission registrations are removed
    assert!(!has_compactor_sub_agent_handler(session.id));
    assert!(
        !codelet_tools::has_generate_compaction_handler(session.id),
        "aborted turns must not retain a stale generation owner"
    );
    assert!(
        !codelet_tools::has_inject_summary_handler(session.id),
        "aborted turns must not accept candidates against a stale source"
    );
    // @step And committed history remains unchanged without completion
    assert_eq!(
        serde_json::to_value(&session.inner.lock().await.messages[..2])
            .expect("unchanged committed context"),
        before
    );
    assert_eq!(
        serde_json::to_value(
            codelet_core::persistence::load_session(session.id).expect("durable originals"),
        )
        .expect("durable snapshot"),
        durable_before
    );
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event, StreamChunk::CompactionComplete { .. }));
    }
}
