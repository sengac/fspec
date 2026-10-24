use codelet_providers::codex::history::{CodexCheckpoint, CodexHistory};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{Message, ToolResultContent, UserContent};
use rig::OneOrMany;
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

fn output() -> Vec<Value> {
    vec![
        json!({"type":"reasoning","id":"rs_1","summary":[],"encrypted_content":"synthetic+/=秘密"}),
        json!({"type":"message","id":"msg_1","role":"assistant","phase":"commentary","content":[{"type":"output_text","text":"Checking","annotations":[]}]}),
        json!({"type":"message","id":"msg_2","role":"assistant","phase":"final_answer","content":[{"type":"output_text","text":"Calling tools","annotations":[]}]}),
        json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"inspect","arguments":"{}"}),
        json!({"type":"function_call","id":"fc_2","call_id":"call_2","name":"inspect","arguments":"{}"}),
    ]
}

fn terminal(items: Vec<Value>) -> String {
    fixture::event(json!({"type":"response.completed","response":{
        "id":"resp_1","status":"completed","output":items,"end_turn":true,
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}))
}

#[tokio::test]
async fn tool_continuation_preserves_typed_output_independently_of_readable_thinking(
) -> fixture::TestResult {
    // @step Given a Codex response with encrypted reasoning phased messages and parallel function calls
    let items = output();
    let server =
        fixture::Server::start(vec![terminal(items.clone()), terminal(vec![])], false).await?;
    let history = CodexHistory::default();
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let request = CompletionRequestBuilder::new(model.clone(), "inspect both").build();
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let results = Message::User {
        content: OneOrMany::many([
            // @step When tool results continue the conversation with readable thinking disabled
            UserContent::tool_result_with_call_id(
                "fc_2",
                "call_2".into(),
                OneOrMany::one(ToolResultContent::text("second")),
            ),
            UserContent::tool_result_with_call_id(
                "fc_1",
                "call_1".into(),
                OneOrMany::one(ToolResultContent::text("first")),
            ),
        ])?,
    };
    let request = CompletionRequestBuilder::new(model.clone(), results)
        .messages(vec![
            Message::user("inspect both"),
            Message::assistant("Checking"),
            Message::assistant("Calling tools"),
        ])
        .build();
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let requests = server
        .requests
        .lock()
        .map_err(|_| "request capture lock poisoned")?;
    // @step Then the next request preserves ordered typed items and opaque reasoning exactly
    // @step And each tool result matches its original call without duplicate calls
    assert_eq!(
        requests[1]["input"]
            .as_array()
            .ok_or("input array missing")?[1..6],
        items
    );
    assert_eq!(requests[1]["input"][6]["call_id"], "call_2");
    assert_eq!(requests[1]["input"][7]["call_id"], "call_1");
    assert_eq!(
        requests[1]["input"]
            .as_array()
            .ok_or("input array missing")?
            .len(),
        8
    );
    Ok(())
}

proptest::proptest! {
    #[test]
    fn opaque_reasoning_round_trips_without_disclosure(opaque in ".{1,150}") {
        // @step Given arbitrary synthetic opaque reasoning payloads
        let history = CodexHistory::default();
        let request = json!({"input":[{"role":"user","content":"hello"}]});
        history.commit(&request, &[json!({"type":"reasoning","id":"r","summary":[],"encrypted_content":opaque})]).map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let checkpoint = history.snapshot()
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?
            .ok_or_else(|| proptest::test_runner::TestCaseError::fail("checkpoint missing"))?;
        let encoded = serde_json::to_string(&checkpoint).map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        // @step When typed checkpoints are serialized and restored
        let decoded: CodexCheckpoint = serde_json::from_str(&encoded).map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        // @step Then every payload is preserved exactly without appearing in debug output
        proptest::prop_assert_eq!(checkpoint, decoded);
        proptest::prop_assert_eq!(format!("{history:?}"), "CodexHistory { .. }");
    }
}
