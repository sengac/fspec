//! Feature: spec/features/honest-chat-terminal-evidence.feature

#[path = "support/bug262_chat.rs"]
pub mod chat;
#[path = "support/prov161_gated_sse.rs"]
pub mod gated;
#[path = "support/prov161_runtime.rs"]
pub mod runtime;
#[path = "support/bug263_chat.rs"]
mod terminal;

use runtime::TestResult;
use serde_json::{json, Value};
use terminal::{consume, done_response, done_sse, frame, run_done, server, sse, usage};
use wiremock::ResponseTemplate;

#[tokio::test]
async fn usage_without_a_finish_reason_cannot_declare_completion() -> TestResult {
    // @step Given a Chat response with consistent usage but no finish reason
    let mut response = chat::response();
    response["choices"][0]["finish_reason"] = json!("");
    for streaming in [true, false] {
        let response = if streaming {
            sse(terminal::terminal(usage(), Value::Null))
        } else {
            ResponseTemplate::new(200).set_body_json(&response)
        };
        let server = server(response).await;
        // @step When the streaming or JSON response ends
        let observed = consume(&server.uri(), streaming).await?;
        // @step Then the request fails once without successful completion or executable tools
        assert_eq!(observed.errors.len(), 1, "{observed:?}");
        assert!(observed.finals.is_empty(), "{observed:?}");
        assert_eq!(observed.calls, 0, "{observed:?}");
    }
    Ok(())
}

#[tokio::test]
async fn malformed_and_provider_error_frames_retain_actionable_causes() -> TestResult {
    // @step Given malformed Chat frames or a provider error followed by otherwise valid terminal evidence
    for (failure, cause) in [
        ("data: {invalid\n\n".into(), "malformed"),
        (
            frame(json!({"error":{"message":"rejected tool schema","code":"invalid_schema"}})),
            "rejected tool schema",
        ),
    ] {
        let body =
            failure + terminal::terminal(usage(), json!("stop")).as_str() + "data: [DONE]\n\n";
        let server = server(sse(body)).await;
        // @step When the response is consumed
        let observed = consume(&server.uri(), true).await?;
        // @step Then the original failure is reported instead of silently declaring success
        assert_eq!(observed.errors.len(), 1, "{observed:?}");
        assert!(observed.errors[0].contains(cause), "{observed:?}");
        assert!(observed.finals.is_empty(), "{observed:?}");
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn done_cannot_conceal_invalid_terminal_evidence() -> TestResult {
    // @step Given a real armed Done call queued before invalid terminal evidence
    let inconsistent = json!({"prompt_tokens":20,"completion_tokens":30,"total_tokens":1});
    let error = frame(json!({"error":{"message":"terminal rejected","code":"rejected"}}));
    for (tokens, tail) in [
        (Value::Null, "data: [DONE]\n\n".to_owned()),
        (inconsistent.clone(), "data: [DONE]\n\n".to_owned()),
        (usage(), error),
        (usage(), "data: {invalid\n\ndata: [DONE]\n\n".to_owned()),
    ] {
        // @step When the Chat response passes through normal attribution and the shared stream loop
        // @step Then Done is not accepted and trailing work and ordinary continuation nudges do not execute
        run_done(sse(done_sse(tokens, &tail)), true, false).await?;
    }
    for tokens in [Value::Null, inconsistent] {
        let mut response = done_response();
        response["usage"] = tokens;
        run_done(
            ResponseTemplate::new(200).set_body_json(response),
            false,
            false,
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn valid_native_done_settles_physical_usage_before_immediate_completion() -> TestResult {
    // @step Given an attributed native Done call followed by trailing work with valid terminal evidence
    for streaming in [true, false] {
        let response = if streaming {
            sse(done_sse(usage(), "data: [DONE]\n\n"))
        } else {
            ResponseTemplate::new(200).set_body_json(done_response())
        };
        // @step When streaming and JSON responses pass through the shared stream loop
        // @step Then exactly one request is billed and its context is settled before accepted Done cancels trailing work
        run_done(response, streaming, true).await?;
    }
    Ok(())
}

#[tokio::test]
async fn explicit_terminal_marker_settles_without_waiting_for_eof() -> TestResult {
    // @step Given a Chat connection sends finish reason and trailing usage followed by DONE but remains open
    let body = terminal::terminal(Value::Null, json!("stop"))
        + frame(json!({"choices":[],"usage":usage()})).as_str()
        + "data: [DONE]\n\n";
    let server = gated::GatedSse::start(vec![body, " ".into()]).await?;
    // @step When the response is consumed with a bounded deadline
    let observed = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        consume(&server.url, true),
    )
    .await;
    // @step Then one final response contains exact usage and stop reason without requiring transport closure
    let _ = server.complete.send(());
    let observed = observed??;
    assert!(observed.errors.is_empty(), "{observed:?}");
    assert_eq!(observed.finals, [(50, Some("end_turn".into()))]);
    server.request.await??;
    Ok(())
}
