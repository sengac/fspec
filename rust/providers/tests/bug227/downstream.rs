//! Feature: spec/features/custom-rig-stream-failure-propagation.feature

use super::fixture::*;
use futures::StreamExt;
use rig::streaming::StreamedAssistantContent;
use std::time::Duration;

#[tokio::test]
async fn scenario_rig_rejects_interrupted_and_malformed_http_streams() -> TestResult {
    // @step Given local HTTP streams with body failures decoding failures or missing terminal evidence
    let mut invalid = frames(&["text", "tool"]);
    invalid.extend_from_slice(b"data: \xff\n\n");
    for (body, interrupted) in [
        (vec![], true),
        (frames(&["text", "reason", "usage", "tool"]), true),
        (frames(&["text", "stop:end_turn"]), true),
        (invalid, false),
        (vec![], false),
        (frames(&["text", "tool"]), false),
        (b"data: text\n".to_vec(), false),
        (b"data: [DO".to_vec(), false),
    ] {
        let mut server = Server::start(body, 200, interrupted, false).await?;
        // @step When the real custom rig model consumes each stream
        let mut stream = rig_stream(&server.url).await?;
        let items: Vec<_> =
            tokio::time::timeout(Duration::from_secs(5), stream.by_ref().collect()).await?;
        // @step Then each stream reports one error and no successful FinalResponse or EndTurn
        assert_eq!(
            items.iter().filter(|item| item.is_err()).count(),
            1,
            "{items:?}"
        );
        assert!(items.last().is_some_and(Result::is_err), "{items:?}");
        assert!(stream.response.is_none());
        assert!(
            !items
                .iter()
                .any(|item| matches!(item, Ok(StreamedAssistantContent::Final(_)))),
            "{items:?}"
        );
        // @step And buffered tools are not completed by failure finalization
        assert!(
            !items
                .iter()
                .any(|item| matches!(item, Ok(StreamedAssistantContent::ToolCall(_)))),
            "{items:?}"
        );
        server.wait().await?;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_rig_preserves_successful_clean_terminal_responses() -> TestResult {
    // @step Given local HTTP streams with text reasoning usage tools and clean terminal evidence
    for (terminal, expected) in [
        ("[DONE]", "end_turn"),
        ("stop:end_turn", "end_turn"),
        ("stop:tool_use", "tool_use"),
        ("stop:max_tokens", "max_tokens"),
    ] {
        let mut server = Server::start(
            frames(&["text", "reason", "usage", "tool", terminal]),
            200,
            false,
            false,
        )
        .await?;
        // @step When the real custom rig model consumes each stream
        let mut stream = rig_stream(&server.url).await?;
        let items: Vec<_> = stream.by_ref().collect().await;
        // @step Then each stream preserves its deltas tools usage and final stop reason
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        assert_eq!(
            items
                .iter()
                .filter(|item| matches!(item, Ok(StreamedAssistantContent::Final(_))))
                .count(),
            1
        );
        assert!(items.iter().any(|item| matches!(item, Ok(StreamedAssistantContent::Text(text)) if text.text == "partial")));
        assert!(items.iter().any(|item| matches!(item, Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) if reasoning == "thinking")));
        assert!(items.iter().any(|item| matches!(item, Ok(StreamedAssistantContent::ToolCall(tool)) if tool.function.name == "read_file" && tool.function.arguments == serde_json::json!({"path":"a.txt"}))));
        let response = stream.response.ok_or("missing final response")?;
        assert_eq!(response.stop_reason, expected);
        assert_eq!((response.input_tokens, response.output_tokens), (12, 3));
        server.wait().await?;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_rig_preserves_http_and_rhai_error_behavior() -> TestResult {
    // @step Given local HTTP streams with an unauthorized response or a Rhai runtime failure
    for (status, body, expected) in [
        (401, b"denied".to_vec(), "unauthorized"),
        (
            200,
            frames(&["text", "tool", "boom"]),
            "unexpected stream payload",
        ),
    ] {
        let mut server = Server::start(body, status, false, false).await?;
        // @step When the real custom rig model consumes each stream
        let mut stream = rig_stream(&server.url).await?;
        let items: Vec<_> = stream.by_ref().collect().await;
        // @step Then each stream reports its original error without a successful final response
        assert_eq!(
            items.iter().filter(|item| item.is_err()).count(),
            1,
            "{items:?}"
        );
        assert!(
            matches!(items.last(), Some(Err(error)) if error.to_string().contains(expected)),
            "{items:?}"
        );
        assert!(stream.response.is_none());
        assert!(!items.iter().any(|item| matches!(
            item,
            Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_))
        )));
        server.wait().await?;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_cancel_a_pending_rig_stream() -> TestResult {
    // @step Given a local HTTP stream pending after its first text delta
    let mut server = Server::start(frames(&["text"]), 200, false, true).await?;
    let mut stream = rig_stream(&server.url).await?;
    assert!(matches!(
        stream.next().await,
        Some(Ok(StreamedAssistantContent::Text(_)))
    ));
    assert!(stream.response.is_none());
    // @step When the consumer drops the rig stream after that delta
    drop(stream);
    // @step Then the HTTP connection closes without a successful final response
    server.wait().await?;
    Ok(())
}
