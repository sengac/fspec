//! Feature: spec/features/custom-stream-failure-propagation.feature

use super::fixture::*;
use bytes::Bytes;
use codelet_providers::custom::stream::StreamChunk;
use codelet_providers::StopReason;
use futures::{stream, StreamExt};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

#[tokio::test]
async fn scenario_fail_transport_before_any_output() -> TestResult {
    // @step Given a custom stream with a body error before any events
    let polls = Arc::new(AtomicUsize::new(0));
    let observed = polls.clone();
    let body = stream::iter(vec![body_error(), Ok(frame("text")), Ok(frame("[DONE]"))]).inspect(
        move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
        },
    );
    // @step When the bridge consumes the stream
    let items: Vec<_> = bridge(body)?.collect().await;
    // @step Then it yields one typed provider error and does not read subsequent events
    assert_error(&items, "deterministic body failure");
    assert_eq!(items.len(), 1);
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn scenario_preserve_partial_deltas_without_flushing_tools_after_transport_failure(
) -> TestResult {
    // @step Given a custom stream with text reasoning usage and buffered tool arguments
    let mut body = vec![
        Ok(frame("text")),
        Ok(frame("reason")),
        Ok(frame("usage")),
        Ok(frame("tool")),
    ];
    // @step When a body error interrupts the stream
    body.extend([body_error(), Ok(frame("[DONE]"))]);
    let items: Vec<_> = bridge(stream::iter(body))?.collect().await;
    // @step Then the emitted deltas precede one error without tool completion or stop
    assert_error(&items, "deterministic body failure");
    assert!(matches!(&items[0], Ok(StreamChunk::TextDelta(text)) if text == "partial"));
    assert!(matches!(&items[1], Ok(StreamChunk::ReasoningDelta(text)) if text == "thinking"));
    assert!(
        matches!(&items[2], Ok(StreamChunk::UsageDelta(usage)) if usage.output_tokens == Some(3))
    );
    assert!(!items.iter().any(|item| matches!(
        item,
        Ok(StreamChunk::ToolCallComplete { .. } | StreamChunk::StopReason(_))
    )));
    Ok(())
}

#[tokio::test]
async fn scenario_treat_sse_decoding_errors_as_fatal() -> TestResult {
    // @step Given invalid UTF-8 followed by valid SSE events
    let body = stream::iter(vec![
        Ok(Bytes::from_static(b"data: \xff\n\n")),
        Ok(frame("text")),
        Ok(frame("[DONE]")),
    ]);
    // @step When the bridge decodes the stream
    let items: Vec<_> = bridge(body)?.collect().await;
    // @step Then it yields one decoding error without processing later events
    assert_error(&items, "SSE");
    assert_eq!(items.len(), 1);
    Ok(())
}

#[tokio::test]
async fn scenario_do_not_conceal_transport_failure_after_explicit_stop() -> TestResult {
    // @step Given a custom stream with an explicit stop followed by a body error
    let body = stream::iter(vec![Ok(frame("stop:end_turn")), body_error()]);
    // @step When the bridge consumes the remaining body
    let items: Vec<_> = bridge(body)?.collect().await;
    // @step Then the stop is followed by one terminal error
    assert!(matches!(
        items.first(),
        Some(Ok(StreamChunk::StopReason(StopReason::EndTurn)))
    ));
    assert_error(&items, "deterministic body failure");
    assert_eq!(items.len(), 2);
    Ok(())
}

#[tokio::test]
async fn scenario_reject_eof_without_terminal_evidence() -> TestResult {
    // @step Given empty text-only pending-tool and truncated custom streams without a stop or DONE
    for body in [
        vec![],
        frames(&["text"]),
        frames(&["tool"]),
        b"data: text\n".to_vec(),
        b"data: [DO".to_vec(),
    ] {
        // @step When each stream reaches ordinary EOF
        let items: Vec<_> = bridge(stream::iter(vec![Ok(Bytes::from(body))]))?
            .collect()
            .await;
        // @step Then each stream yields one incomplete-stream error without flushing tools or stop
        assert_error(&items, "terminal");
        assert!(!items.iter().any(|item| matches!(
            item,
            Ok(StreamChunk::ToolCallComplete { .. } | StreamChunk::StopReason(_))
        )));
    }
    Ok(())
}

#[tokio::test]
async fn scenario_preserve_clean_done_completion() -> TestResult {
    // @step Given text reasoning usage and buffered tool arguments followed by DONE
    let events = ["text", "reason", "usage", "tool", "[DONE]", "boom"];
    // @step When the bridge consumes the clean stream
    let items = collect_bridge(&events).await?;
    // @step Then the deltas and completed tool are preserved without passing DONE to Rhai
    assert!(items.iter().all(Result::is_ok), "{items:?}");
    assert!(matches!(&items[0], Ok(StreamChunk::TextDelta(text)) if text == "partial"));
    assert!(matches!(&items[1], Ok(StreamChunk::ReasoningDelta(text)) if text == "thinking"));
    assert!(
        matches!(&items[2], Ok(StreamChunk::UsageDelta(usage)) if usage.input_tokens == Some(12))
    );
    assert!(
        matches!(items.last(), Some(Ok(StreamChunk::ToolCallComplete { input, .. })) if input == &serde_json::json!({"path":"a.txt"}))
    );
    Ok(())
}

#[tokio::test]
async fn scenario_preserve_explicit_stop_with_clean_eof() -> TestResult {
    // @step Given custom streams ending with each supported explicit stop reason
    for (event, expected) in [
        ("stop:end_turn", StopReason::EndTurn),
        ("stop:tool_use", StopReason::ToolUse),
        ("stop:max_tokens", StopReason::MaxTokens),
    ] {
        // @step When the bridge reaches clean EOF
        let items = collect_bridge(&[event]).await?;
        // @step Then each stream preserves its stop reason without error
        assert_eq!(items.len(), 1);
        assert!(
            matches!(items.first(), Some(Ok(StreamChunk::StopReason(reason))) if *reason == expected),
            "{items:?}"
        );
    }
    Ok(())
}

struct DropSignal(Arc<AtomicBool>);
impl Drop for DropSignal {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn scenario_cancel_a_pending_custom_stream() -> TestResult {
    // @step Given a custom byte source pending after its first text delta
    let dropped = Arc::new(AtomicBool::new(false));
    let guard = DropSignal(dropped.clone());
    let polls = Arc::new(AtomicUsize::new(0));
    let observed = polls.clone();
    let mut body = bridge(async_stream::stream! {
        let _guard = guard;
        observed.fetch_add(1, Ordering::SeqCst);
        yield Ok::<_, std::io::Error>(frame("text"));
        observed.fetch_add(1, Ordering::SeqCst);
        futures::future::pending::<()>().await;
    })?;
    assert!(matches!(
        body.next().await,
        Some(Ok(StreamChunk::TextDelta(_)))
    ));
    // @step When the consumer drops the bridge after that delta
    drop(body);
    // @step Then the source is dropped without further polling or finalization
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    Ok(())
}
