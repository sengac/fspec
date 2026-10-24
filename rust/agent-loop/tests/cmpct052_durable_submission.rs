#[path = "cmpct056/support.rs"]
mod support;

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_core::RigAgent;
use codelet_tools::inject_summary::{execute_inject_summary, InjectSummaryTool};
use rig::client::CompletionClient;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use support::{assert_no_completion, Fixture, DAG};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Output(Mutex<Vec<StreamEvent>>);

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .expect("record selected stream event")
            .push(event);
    }
}

async fn provider(content: &str) -> MockServer {
    let server = MockServer::start().await;
    let arguments = serde_json::json!({"content": content}).to_string();
    let chunks = [
        serde_json::json!({"id":"submission","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"summary-call","type":"function","function":{"name":"inject_summary","arguments":arguments}}]},"finish_reason":null}]}),
        serde_json::json!({"id":"submission","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
    ];
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!(
                    "{}data: [DONE]\n\n",
                    chunks
                        .iter()
                        .map(|chunk| format!("data: {chunk}\n\n"))
                        .collect::<String>()
                )),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_production_summary_submission_commits_its_captured_durable_source() {
    // @step Given a live session with immutable originals and a selected physical serializer
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let original_records = std::fs::read(fixture.directory.path().join("messages/messages.jsonl"))
        .expect("original envelope bytes");
    let mut inner = fixture.target.inner.lock().await;
    let queue = register_generate_compaction_handler(
        fixture.target.id,
        &inner,
        fixture.directory.path().into(),
        fixture.target.clone(),
        Some(fixture.manager.clone()),
    );
    let server = provider(DAG).await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("test")
            .base_url(server.uri())
            .build()
            .expect("local tool client");
    let agent = RigAgent::new(
        client
            .agent("o3")
            .max_tokens(4096)
            .tool(InjectSummaryTool::new(fixture.target.id))
            .build(),
        3,
    );
    let output = Output::default();
    // @step When its registered summary tool queues a complete DAG during streaming
    run_agent_stream(
        agent,
        "submit captured summary",
        &mut inner,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(tokio::sync::Notify::new()),
        &output,
        fixture.target.id,
    )
    .await
    .expect("handoff before next inference");
    assert_eq!(server.received_requests().await.expect("requests").len(), 1);
    // @step Then inference yields with the real tool call and result paired
    codelet_cli::interactive_helpers::validate_no_orphan_tool_calls(&inner.messages)
        .expect("actual tool pair survives handoff");
    assert!(inner.ensure_compaction_inference_ready().is_err());
    // @step And the commit owner promotes and publishes exactly that captured generation
    assert!(queue
        .apply(&mut inner, &fixture.target)
        .expect("durable candidate application"));
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    // @step And cold replay retains the original envelopes and admitted generation
    let manifest =
        codelet_core::persistence::load_session(fixture.target.id).expect("cold manifest");
    let boundary = manifest
        .compaction_commit
        .as_ref()
        .expect("durable generation");
    assert_eq!(boundary.covered_prefix.len(), 2);
    assert!(!boundary.publication_pending);
    assert_eq!(manifest.messages.len(), 2);
    assert_eq!(
        fixture
            .target
            .committed_context_snapshot()
            .expect("published generation")
            .generation,
        boundary.identity.generation as i64
    );
    assert!(!queue
        .apply(&mut inner, &fixture.target)
        .expect("no duplicate application"));
    drop(inner);
    codelet_core::persistence::reset_stores_for_tests();
    let cold_manifest = codelet_core::persistence::load_session(fixture.target.id)
        .expect("manifest from disk without cached stores");
    assert_eq!(
        serde_json::to_value(&cold_manifest).expect("cold manifest"),
        serde_json::to_value(&manifest).expect("promoted manifest")
    );
    let replay = codelet_core::persistence::get_session_message_envelopes(fixture.target.id)
        .expect("committed replay");
    assert_eq!(replay.len(), 1);
    let envelope: serde_json::Value = serde_json::from_str(&replay[0]).expect("summary envelope");
    assert_eq!(
        envelope["_committedCompaction"]["generation"],
        boundary.identity.generation
    );
    assert_eq!(
        envelope["message"]["content"],
        serde_json::json!([
            {"type":"text","text":format!("[Previous conversation summary]\n\n{DAG}")}
        ])
    );
    let cold_manager = codelet_sessions::SessionManager::new();
    cold_manager
        .create_session_from_manifest(&cold_manifest, "openai/o3")
        .await
        .expect("fresh runtime from committed identity");
    codelet_core::SessionManagerHandle::restore_session_messages(
        &cold_manager,
        &codelet_rpc_types::SessionId::new(fixture.target.id.to_string()),
        replay,
    )
    .expect("restore committed runtime replay");
    let restored = cold_manager
        .get_session(&fixture.target.id.to_string())
        .expect("cold runtime");
    let restored = restored.inner.lock().await;
    let installation = restored
        .restored_replay
        .as_ref()
        .expect("installed cold replay");
    assert_eq!(installation.replay_len, 1);
    assert_eq!(
        serde_json::to_value(&restored.messages[installation.runtime_prefix_len..])
            .expect("complete runtime replay"),
        serde_json::to_value([rig::message::Message::user(format!(
            "[Previous conversation summary]\n\n{DAG}"
        ))])
        .expect("expected summary message")
    );
    assert!(restored.ensure_compaction_inference_ready().is_ok());
    assert_eq!(
        std::fs::read(fixture.directory.path().join("messages/messages.jsonl"))
            .expect("unchanged original envelope bytes"),
        original_records
    );
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_a_queued_production_candidate_cannot_be_overwritten() {
    // @step Given a live session with an accepted summary awaiting durable application
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let mut inner = fixture.target.inner.lock().await;
    let queue = register_generate_compaction_handler(
        fixture.target.id,
        &inner,
        fixture.directory.path().into(),
        fixture.target.clone(),
        Some(fixture.manager.clone()),
    );
    execute_inject_summary(fixture.target.id, DAG.into()).expect("first captured submission");
    let other = DAG.replace("completed evidence", "replacement must not commit");
    // @step When another summary submission tries to replace the candidate
    // @step Then the second submission is rejected without changing the queued candidate
    assert!(execute_inject_summary(fixture.target.id, other).is_err());
    // @step And only the first candidate can become the committed generation
    queue
        .apply(&mut inner, &fixture.target)
        .expect("first candidate application");
    let manifest =
        codelet_core::persistence::load_session(fixture.target.id).expect("cold manifest");
    assert_eq!(manifest.compaction.expect("committed DAG").summary, DAG);
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_production_candidate_rejection_preserves_memory_and_blocks_inference() {
    // @step Given a live session with immutable originals and a selected physical serializer
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let mut inner = fixture.target.inner.lock().await;
    let queue = register_generate_compaction_handler(
        fixture.target.id,
        &inner,
        fixture.directory.path().into(),
        fixture.target.clone(),
        Some(fixture.manager.clone()),
    );
    let before = serde_json::json!({"messages": inner.messages, "tokens": inner.token_tracker});
    let mut events = fixture.target.subscribe_to_stream();
    let server = provider("invalid candidate, not a DAG").await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("test")
            .base_url(server.uri())
            .build()
            .expect("local tool client");
    let agent = RigAgent::new(
        client
            .agent("o3")
            .max_tokens(4096)
            .tool(InjectSummaryTool::new(fixture.target.id))
            .build(),
        3,
    );
    // @step When its registered summary tool submits an invalid DAG during streaming
    let result = run_agent_stream(
        agent,
        "submit invalid candidate",
        &mut inner,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(tokio::sync::Notify::new()),
        &Output::default(),
        fixture.target.id,
    )
    .await;
    assert!(result.is_err());
    // @step Then ordinary inference remains blocked without a committed generation
    assert_eq!(server.received_requests().await.expect("requests").len(), 1);
    assert!(inner.ensure_compaction_inference_ready().is_err());
    let rejected_messages = serde_json::to_value(&inner.messages).expect("rejected history");
    let retry_agent = RigAgent::new(client.agent("o3").max_tokens(4096).build(), 3);
    let retry = run_agent_stream(
        retry_agent,
        "ordinary inference after rejected summary",
        &mut inner,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(tokio::sync::Notify::new()),
        &Output::default(),
        fixture.target.id,
    )
    .await;
    assert!(retry.is_err());
    assert_eq!(server.received_requests().await.expect("requests").len(), 1);
    assert_eq!(
        serde_json::to_value(&inner.messages).expect("history after rejected retry"),
        rejected_messages
    );
    assert!(!queue
        .apply(&mut inner, &fixture.target)
        .expect("nothing accepted"));
    let manifest =
        codelet_core::persistence::load_session(fixture.target.id).expect("old generation");
    assert!(manifest.compaction_commit.is_none());
    // @step And committed messages and counters remain unchanged without completion
    assert_eq!(manifest.messages.len(), 2);
    assert_eq!(
        serde_json::to_value(&inner.token_tracker).expect("counters"),
        before["tokens"]
    );
    assert_no_completion(&mut events);
}
