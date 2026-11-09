//! Feature: spec/features/malformed-tool-call-argument-guidance.feature

#[path = "bug221/support.rs"]
mod support;

use codelet_cli::interactive::StreamEvent;
use rig::message::{AssistantContent, Message, ToolCall, ToolFunction, UserContent};
use rig::OneOrMany;
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, PoisonError};
use support::{response, Fixture, TestResult};
use wiremock::ResponseTemplate;

const ANSWER: &str = "Corrected and finished.";

/// A streamed tool call whose `arguments` field is sent verbatim.
fn tool_call_response(arguments: Value) -> ResponseTemplate {
    let chunk = json!({
        "id":"malformed-tool", "object":"chat.completion.chunk", "created":1,
        "model":"selected-model",
        "choices":[{"index":0,"delta":{"role":"assistant",
            "tool_calls":[{"index":0,"id":"inspect-1","type":"function",
                "function":{"name":"inspect","arguments":arguments}}]},
            "finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120}
    });
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

fn request_messages(request: &wiremock::Request) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let body: Value = serde_json::from_slice(&request.body)?;
    Ok(body["messages"].as_array().cloned().unwrap_or_default())
}

fn tool_result_text(messages: &[Value]) -> String {
    messages
        .iter()
        .filter(|message| message["role"] == "tool")
        .map(|message| message["content"].to_string())
        .collect()
}

fn replayed_tool_arguments(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .filter_map(|message| message["tool_calls"].as_array())
        .flatten()
        .map(|call| call["function"]["arguments"].clone())
        .collect()
}

fn finished_with_answer(fixture: &Fixture) -> bool {
    let events = fixture
        .output
        .events
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    events
        .iter()
        .any(|event| matches!(event, StreamEvent::Text(text) if text == ANSWER))
        && !events
            .iter()
            .any(|event| matches!(event, StreamEvent::Error(_)))
}

#[tokio::test]
#[serial_test::serial]
async fn a_tool_call_with_null_arguments_is_not_run_and_the_model_is_told_how_to_call_the_tool(
) -> TestResult {
    // @step Given a model that calls the inspect tool with null arguments and then answers
    let mut fixture = Fixture::new(
        vec![
            tool_call_response(Value::Null),
            response("", ANSWER, Some("stop")),
        ],
        false,
        false,
    )
    .await?;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then the inspect tool is not executed
    assert_eq!(fixture.tool_calls.load(Ordering::Acquire), 0);
    let requests = fixture.requests().await?;
    assert_eq!(requests.len(), 2);
    // @step And the tool result explains that inspect was not run and lists its required parameters
    let follow_up = request_messages(&requests[1])?;
    let result = tool_result_text(&follow_up);
    assert!(result.contains("`inspect`"), "{result}");
    assert!(result.contains("NOT run"), "{result}");
    assert!(result.contains("received null"), "{result}");
    assert!(result.contains("work_unit_id"), "{result}");
    // @step And the follow-up request carries the tool call with empty JSON object arguments
    assert_eq!(replayed_tool_arguments(&follow_up), vec![json!("{}")]);
    // @step And the turn finishes with the model's answer and no error
    assert!(finished_with_answer(&fixture));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_tool_call_with_truncated_json_arguments_reports_what_was_received() -> TestResult {
    // @step Given a model that calls the inspect tool with arguments truncated mid-object and then answers
    let truncated = r#"{"work_unit_id": "NONE", "file_path": "/tmp/a.rs""#;
    let mut fixture = Fixture::new(
        vec![
            tool_call_response(json!(truncated)),
            response("", ANSWER, Some("stop")),
        ],
        false,
        false,
    )
    .await?;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then the inspect tool is not executed
    assert_eq!(fixture.tool_calls.load(Ordering::Acquire), 0);
    // @step And the tool result reports the received character count and an excerpt and suggests splitting large content
    let requests = fixture.requests().await?;
    let follow_up = request_messages(&requests[1])?;
    let result = tool_result_text(&follow_up);
    assert!(
        result.contains(&format!("{}-character", truncated.chars().count())),
        "{result}"
    );
    assert!(result.contains("/tmp/a.rs"), "{result}");
    assert!(result.contains("split"), "{result}");
    assert_eq!(replayed_tool_arguments(&follow_up), vec![json!("{}")]);
    assert!(finished_with_answer(&fixture));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn stringified_json_object_arguments_are_executed_normally() -> TestResult {
    // @step Given a model that calls the inspect tool with a JSON object encoded as a string and then answers
    let encoded = json!(r#"{"work_unit_id":"NONE"}"#).to_string();
    let mut fixture = Fixture::new(
        vec![
            tool_call_response(json!(encoded)),
            response("", ANSWER, Some("stop")),
        ],
        false,
        false,
    )
    .await?;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then the inspect tool is executed once
    assert_eq!(fixture.tool_calls.load(Ordering::Acquire), 1);
    assert!(finished_with_answer(&fixture));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn legacy_history_with_null_tool_call_arguments_replays_as_a_valid_request() -> TestResult {
    // @step Given a session history containing a tool call whose arguments are null
    let mut fixture = Fixture::new(vec![response("", ANSWER, Some("stop"))], false, false).await?;
    fixture.session.messages.extend([
        Message::User {
            content: OneOrMany::one(UserContent::text("write the file")),
        },
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(ToolCall {
                id: "legacy-1".into(),
                call_id: None,
                function: ToolFunction {
                    name: "Write".into(),
                    arguments: Value::Null,
                },
                signature: None,
                additional_params: None,
            })),
        },
        Message::User {
            content: OneOrMany::one(UserContent::tool_result(
                "legacy-1",
                OneOrMany::one(rig::message::ToolResultContent::text("failed")),
            )),
        },
    ]);
    // @step When the next request is sent to the OpenAI-compatible server
    fixture.run().await?;
    // @step Then the replayed tool call arguments are an empty JSON object
    let requests = fixture.requests().await?;
    let first = request_messages(&requests[0])?;
    assert_eq!(replayed_tool_arguments(&first), vec![json!("{}")]);
    Ok(())
}
