//! HTTP streaming bridge for `RhaiCustomProvider` (PROV-064). Splits
//! the async `open_stream` adapter out of `stream.rs` so each file
//! stays under the 300-line cap.

use std::pin::Pin;

use async_stream::stream;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};

use super::stream::{RhaiStreamProcessor, StreamChunk};
use crate::error::ProviderError;

/// Build the streaming adapter used by
/// `RhaiCustomProvider::complete_with_tools_streaming`.
///
/// `status` is the HTTP status code and `body_stream` is the raw byte
/// stream produced by `reqwest::Response::bytes_stream()` (or an
/// equivalent source in tests). When `status` is outside the 2xx range,
/// the adapter drains the body, invokes `map_error_fn`, yields a single
/// `Err` and terminates without touching the Rhai bridge.
///
/// Inside the happy path the adapter consumes SSE events via
/// `eventsource-stream`, treats `data: [DONE]` as a terminator (without
/// calling `parse_stream_chunk`), and forwards every other event's
/// `data` field to `RhaiStreamProcessor::process_event`.
pub fn open_stream<E, MapErr, MapErrFut, Body>(
    processor: RhaiStreamProcessor,
    status: u16,
    body_stream: Body,
    map_error_fn: MapErr,
) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>
where
    E: std::fmt::Display + Send + 'static,
    Body: Stream<Item = Result<bytes::Bytes, E>> + Send + 'static,
    MapErr: FnOnce(u16, String) -> MapErrFut + Send + 'static,
    MapErrFut: std::future::Future<Output = ProviderError> + Send + 'static,
{
    Box::pin(stream! {
        tracing::warn!(
            "[rhai-dispatch] open_stream ENTER: status={}",
            status
        );
        if !(200..300).contains(&status) {
            let mut body = String::new();
            let mut drained = Box::pin(body_stream);
            while let Some(chunk) = drained.next().await {
                if let Ok(bytes) = chunk {
                    body.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
            tracing::warn!(
                "[rhai-dispatch] open_stream non-2xx status={} body_len={} body_preview={:?}",
                status,
                body.len(),
                body.chars().take(500).collect::<String>()
            );
            let err = map_error_fn(status, body).await;
            yield Err(err);
            return;
        }

        let mut events = Box::pin(body_stream.eventsource());
        let mut processor = processor;
        let mut sse_events = 0usize;

        while let Some(event_result) = events.next().await {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "[rhai-dispatch] open_stream: sse frame parse error");
                    continue;
                }
            };
            sse_events += 1;
            tracing::warn!(
                "[rhai-dispatch] open_stream: received SSE event #{} type={:?} data_len={}",
                sse_events,
                event.event,
                event.data.len()
            );
            if event.data.trim() == "[DONE]" {
                tracing::warn!(
                    "[rhai-dispatch] open_stream: got [DONE] total_events={} flushing processor",
                    sse_events
                );
                for chunk in processor.mark_done() {
                    yield Ok(chunk);
                }
                return;
            }
            match processor.process_event(&event.data).await {
                Ok(chunks) => {
                    for c in chunks {
                        yield Ok(c);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "[rhai-dispatch] open_stream: process_event returned ERROR"
                    );
                    yield Err(e);
                    return;
                }
            }
        }

        tracing::warn!(
            "[rhai-dispatch] open_stream: byte stream ended total_events={} — calling processor.finish()",
            sse_events
        );
        for chunk in processor.finish() {
            yield Ok(chunk);
        }
    })
}
