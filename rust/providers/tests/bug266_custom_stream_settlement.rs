//! Feature: spec/features/custom-stream-settlement.feature

#[path = "bug227/fixture.rs"]
pub mod fixture;

use fixture::{frames, rig_stream, Server, TestResult};
use futures::StreamExt;
use rig::streaming::StreamedAssistantContent;
use std::time::Duration;

async fn successful(events: &[&str], pending: bool) -> TestResult {
    let mut server = Server::start(frames(events), 200, false, pending).await?;
    let mut stream = rig_stream(&server.url).await?;
    let items: Vec<_> =
        tokio::time::timeout(Duration::from_secs(3), stream.by_ref().collect()).await?;
    assert!(items.iter().all(Result::is_ok), "{items:?}");
    let response = stream.response.ok_or("missing final response")?;
    assert_eq!((response.input_tokens, response.output_tokens), (12, 3));
    assert_eq!(response.stop_reason, "end_turn");
    assert_eq!(
        items
            .iter()
            .filter(|item| matches!(item, Ok(StreamedAssistantContent::Final(_))))
            .count(),
        1
    );
    assert!(!items.iter().any(|item| matches!(
        item,
        Ok(StreamedAssistantContent::Text(_) | StreamedAssistantContent::ToolCall(_))
    )));
    server.wait().await?;
    Ok(())
}

#[tokio::test]
async fn done_settles_trailing_usage_without_waiting_for_eof() -> TestResult {
    // @step Given a local custom stream emits stop then usage then DONE and stays open
    // @step When the real scripted model consumes the response
    // @step Then it settles promptly with exact input and output usage
    successful(&["stop:end_turn", "usage", "[DONE]"], true).await
}

#[tokio::test]
async fn clean_eof_after_stop_retains_trailing_usage() -> TestResult {
    // @step Given a local custom stream emits stop then usage and clean EOF
    // @step When the real scripted model consumes the response
    // @step Then it settles with exact input and output usage
    successful(&["stop:end_turn", "usage"], false).await
}

#[tokio::test]
async fn post_stop_failure_cannot_release_executable_tools() -> TestResult {
    // @step Given a local custom stream queues a tool then stop and a transport failure
    let mut server = Server::start(frames(&["tool", "stop:tool_use"]), 200, true, false).await?;
    // @step When the real scripted model consumes the response
    let mut stream = rig_stream(&server.url).await?;
    let items: Vec<_> = stream.by_ref().collect().await;
    // @step Then one error survives without successful final or executable calls
    assert_eq!(
        items.iter().filter(|item| item.is_err()).count(),
        1,
        "{items:?}"
    );
    assert!(items.last().is_some_and(Result::is_err));
    assert!(stream.response.is_none());
    assert!(!items.iter().any(|item| matches!(
        item,
        Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_))
    )));
    server.wait().await?;
    Ok(())
}

#[tokio::test]
async fn missing_terminal_evidence_remains_a_failure() -> TestResult {
    // @step Given a local custom stream ends with text but no stop or DONE
    let mut server = Server::start(frames(&["text"]), 200, false, false).await?;
    // @step When the real scripted model consumes the response
    let mut stream = rig_stream(&server.url).await?;
    let items: Vec<_> = stream.by_ref().collect().await;
    // @step Then one error survives without successful final or executable calls
    assert_eq!(items.iter().filter(|item| item.is_err()).count(), 1);
    assert!(items.last().is_some_and(Result::is_err));
    assert!(stream.response.is_none());
    assert!(!items.iter().any(|item| matches!(
        item,
        Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_))
    )));
    server.wait().await?;
    Ok(())
}

#[tokio::test]
async fn stop_alone_does_not_invent_transport_success() -> TestResult {
    // @step Given a local custom stream emits text and stop then stays open
    let mut server = Server::start(frames(&["text", "stop:end_turn"]), 200, false, true).await?;
    let mut stream = rig_stream(&server.url).await?;
    assert!(matches!(
        stream.next().await,
        Some(Ok(StreamedAssistantContent::Text(_)))
    ));
    // @step When the consumer waits for settlement and then cancels
    assert!(
        tokio::time::timeout(Duration::from_millis(100), stream.next())
            .await
            .is_err()
    );
    // @step Then no successful final appears and cancellation closes the connection
    assert!(stream.response.is_none());
    drop(stream);
    server.wait().await?;
    Ok(())
}

#[tokio::test]
async fn post_stop_content_cannot_extend_the_completed_response() -> TestResult {
    // @step Given a local custom stream emits stop then extra text tools and usage before DONE
    // @step When the real scripted model consumes the response
    // @step Then trailing usage is preserved without extra text or executable calls
    successful(&["stop:end_turn", "text", "tool", "usage", "[DONE]"], false).await
}
