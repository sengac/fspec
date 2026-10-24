//! Feature: spec/features/compaction-builder-budget-recovery.feature

#[path = "support/automatic_compaction.rs"]
mod automatic;
#[path = "support/bug211_budget.rs"]
mod budget;
#[path = "support/bug210_completion_metrics.rs"]
mod metrics;

use codelet_rpc_types::{SessionStatus, StreamChunk};
use std::sync::atomic::Ordering;

#[tokio::test]
#[serial_test::serial]
async fn scenario_streaming_synthesis_preserves_the_final_original_evidence_page() {
    // @step Given a streaming compactor uses its exploration budget reading persisted originals
    let fixture = metrics::Fixture::new().await.expect("durable fixture");
    let builder = budget::codex_exploration(&fixture.server, fixture.session.id).await;
    // @step When its final tool result contains evidence needed by the summary
    let result = codelet_agent_loop::compaction_execution::run(
        builder.agent.clone(),
        fixture.session.id,
        "Build the summary",
        "codex",
        20,
        codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    )
    .await;
    assert!(
        result.is_ok(),
        "streaming exhaustion must synthesize: {result:?}"
    );
    // @step Then one tool-free streaming synthesis request includes that final tool result
    let requests = fixture.server.received_requests().await.expect("requests");
    assert_eq!(
        requests
            .iter()
            .filter(|request| !budget::has_tools(request))
            .count(),
        1
    );
    let body: serde_json::Value = requests
        .last()
        .expect("synthesis request")
        .body_json()
        .expect("JSON");
    let position = requests.len() - 2;
    let outputs = body["input"]
        .as_array()
        .expect("structured input")
        .iter()
        .filter(|item| {
            item["type"] == "function_call_output"
                && item["call_id"] == format!("evidence-{position}")
        })
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 1);
    assert!(outputs[0]["output"]
        .to_string()
        .contains(&format!("original {position}")));
    // @step And the valid summary is returned on the same provider and model
    assert_eq!(body["model"], "selected-codex-model");
    assert_eq!(result.expect("streaming summary"), metrics::DAG);
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_exhausted_exploration_can_finish_a_summary_without_further_tools() {
    // @step Given a compactor uses its exploration budget reading persisted originals
    let fixture = metrics::Fixture::new().await.expect("durable fixture");
    budget::exploration(&fixture.server, fixture.session.id, metrics::DAG, false).await;
    // @step When its exploration ends before it returns a summary
    let result = budget::build(fixture.session.id).await;
    // @step Then one tool-free synthesis request includes the gathered evidence
    assert!(
        result.is_ok(),
        "depth exhaustion must synthesize: {result:?}"
    );
    let requests = fixture.server.received_requests().await.expect("requests");
    assert_eq!(
        requests
            .iter()
            .filter(|request| !budget::has_tools(request))
            .count(),
        1
    );
    let final_request = requests.last().expect("synthesis request");
    let body: serde_json::Value = final_request.body_json().expect("synthesis JSON");
    let last_original = requests.len() - 2;
    assert!(
        body["messages"]
            .to_string()
            .contains(&format!("original {last_original}")),
        "final request: {body}"
    );
    // @step And its valid summary is returned without another exploration round
    assert_eq!(result.expect("synthesized DAG"), metrics::DAG);
    assert!(!budget::has_tools(final_request));
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_invalid_synthesized_summaries_are_never_installed() {
    // @step Given an exhausted compactor synthesizes an invalid summary
    let fixture = metrics::Fixture::new().await.expect("durable fixture");
    budget::exploration(&fixture.server, fixture.session.id, "not a DAG", false).await;
    let original =
        codelet_core::persistence::load_session(fixture.session.id).expect("original boundary");
    // @step When the bounded candidate correction budget is exhausted
    let result = budget::build(fixture.session.id).await;
    assert!(result.is_err());
    assert!(result
        .expect_err("invalid summary rejected")
        .contains("after 3 attempts"));
    let requests = fixture.server.received_requests().await.expect("requests");
    assert_eq!(
        requests
            .iter()
            .filter(|request| !budget::has_tools(request))
            .count(),
        3
    );
    // @step Then compaction fails without changing the durable conversation boundary
    let retained =
        codelet_core::persistence::load_session(fixture.session.id).expect("retained boundary");
    assert_eq!(
        retained
            .messages
            .iter()
            .map(|message| &message.message_id)
            .collect::<Vec<_>>(),
        original
            .messages
            .iter()
            .map(|message| &message.message_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(retained.compaction_commit, original.compaction_commit);
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_provider_failures_do_not_trigger_budget_synthesis() {
    // @step Given a compactor receives a provider error rather than a depth limit
    let fixture = metrics::Fixture::new().await.expect("durable fixture");
    fixture.server.reset().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(400).set_body_json(
                serde_json::json!({"error":{"message":"provider rejected request"}}),
            ),
        )
        .mount(&fixture.server)
        .await;
    // @step When the builder reports the failure
    let result = budget::build(fixture.session.id).await;
    assert!(result
        .expect_err("provider failure")
        .contains("provider rejected request"));
    // @step Then no tool-free synthesis request is made
    let requests = fixture.server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
    assert!(budget::has_tools(&requests[0]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_fresh_user_input_retries_a_failed_builder_before_ordinary_inference() {
    // @step Given automatic compaction failed before committing a summary
    let mut fixture = automatic::Harness::new(false, true, false).await;
    fixture.send();
    fixture.until_done().await;
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 1);
    fixture.fail_builder.store(false, Ordering::SeqCst);
    // @step When a fresh user message arrives after the builder becomes available
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then a new summary is committed before ordinary inference resumes
    let committed = chunks
        .iter()
        .position(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. }));
    let finished = chunks.iter().position(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains("Original task finished")));
    assert!(
        committed.is_some(),
        "new input must retry failed builder: {chunks:?}"
    );
    assert!(committed < finished);
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 2);
    // @step And the user message completes without replaying the oversized request
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 2);
    let requests = fixture.server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 4);
    assert!(String::from_utf8_lossy(&requests[2].body).contains("Target session:"));
    assert!(String::from_utf8_lossy(&requests[3].body).contains("Completed evidence"));
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
    fixture.assert_originals_available();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_repeated_builder_failures_wait_for_fresh_user_input() {
    // @step Given automatic compaction failed before committing a summary
    let mut fixture = automatic::Harness::new(false, true, false).await;
    fixture.send();
    fixture.until_done().await;
    // @step When a fresh user message retries the still failing builder
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then one builder attempt fails and the session returns idle
    assert_eq!(
        fixture.builders.load(Ordering::SeqCst),
        2,
        "one explicit builder retry"
    );
    assert_eq!(fixture.session.get_status(), SessionStatus::Idle);
    assert!(chunks.iter().any(
        |chunk| matches!(chunk, StreamChunk::Error { error, .. } if error.contains("retry later"))
    ));
    // @step And no ordinary inference or automatic retry starts
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 2);
    assert!(!chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_unrelated_inference_blocks_cannot_restart_compaction() {
    // @step Given ordinary inference is blocked for an unresolved publication failure
    let mut fixture = automatic::Harness::new(false, false, false).await;
    fixture
        .session
        .inner
        .lock()
        .await
        .block_compaction_inference("publication requires explicit resolution".into());
    // @step When a fresh user message arrives
    fixture.send();
    let chunks = fixture.until_done().await;
    // @step Then the existing failure remains blocked
    assert!(chunks.iter().any(|chunk| matches!(chunk, StreamChunk::Error { error, .. } if error.contains("publication requires explicit resolution"))));
    assert!(fixture
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    // @step And neither a builder nor ordinary inference starts
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_background_notifications_cannot_authorize_a_failed_builder_retry() {
    // @step Given automatic compaction failed before committing a summary
    let mut fixture = automatic::Harness::new(false, true, false).await;
    fixture.send();
    fixture.until_done().await;
    fixture.fail_builder.store(false, Ordering::SeqCst);
    // @step When another agent sends a background notification
    fixture
        .session
        .receive_incoming_message(
            codelet_sessions::background_session::IncomingMessage::new(
                uuid::Uuid::new_v4().to_string(),
                "reviewer".into(),
                "background progress update".into(),
            )
            .expect("background notification"),
        )
        .expect("deliver notification");
    let chunks = fixture.until_done().await;
    // @step Then the failed operation remains blocked
    assert!(chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
    assert!(fixture
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    // @step And neither a builder nor ordinary inference starts
    assert_eq!(fixture.builders.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.infer.load(Ordering::SeqCst), 1);
}
