#![cfg(feature = "test-support")]
//! Feature: spec/features/completion-persisted-history-composition.feature

#[path = "support/bug268_environment.rs"]
mod environment;
#[path = "support/bug268_live.rs"]
pub mod support;

use environment::{Environment, TestResult};
use serde_json::Value;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn accepted_native_completion_persists_a_paired_transcript() -> TestResult {
    // @step Given an isolated native provider session has continuation armed
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let server = support::server().await;
    support::auth(root, &server, "synthetic-persistence-account").await?;
    let mut live = support::Live::new(root, None)?;
    // @step When a serialized response accepts native Done through the shared output owner
    live.turn("retain the accepted completion on disk").await?;
    let session_id = live.background.id;
    assert_eq!(
        server
            .received_requests()
            .await
            .ok_or("missing capture")?
            .len(),
        1
    );
    // @step And cached persistence stores are reset before reloading the conversation
    codelet_core::persistence::reset_stores_for_tests();
    let manifest = codelet_core::persistence::load_session(session_id)?;
    let stored = codelet_core::persistence::get_session_messages_full(&manifest)?;
    let envelopes = stored
        .iter()
        .map(|message| serde_json::to_value(&message.metadata))
        .collect::<Result<Vec<Value>, _>>()?;
    let blocks = envelopes
        .iter()
        .flat_map(|envelope| {
            envelope["message"]["content"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
    // @step Then disk message envelopes retain exactly one matching Done invocation and result
    let calls = blocks
        .iter()
        .filter(|block| block["type"] == "tool_use")
        .collect::<Vec<_>>();
    let results = blocks
        .iter()
        .filter(|block| block["type"] == "tool_result")
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1, "{envelopes:?}");
    assert_eq!(results.len(), 1, "{envelopes:?}");
    assert_eq!(calls[0]["name"], "Done");
    assert_eq!(calls[0]["call_id"], "owned-call");
    assert_eq!(results[0]["tool_use_id"], calls[0]["id"]);
    assert_eq!(results[0]["call_id"], calls[0]["call_id"]);
    assert_eq!(results[0]["is_error"], false);
    // @step And accepted completion is retained once without owned opaque transport state
    assert_eq!(
        results[0]["content"]
            .as_str()
            .ok_or("result content absent")?
            .matches("Completion recorded")
            .count(),
        1
    );
    assert!(!serde_json::to_string(&envelopes)?.contains(support::OPAQUE));
    assert!(!serde_json::to_string(&manifest)?.contains(support::OPAQUE));
    assert_eq!(manifest.token_usage.cumulative_billed_input, 12);
    assert_eq!(manifest.token_usage.cumulative_billed_output, 3);
    Ok(())
}
