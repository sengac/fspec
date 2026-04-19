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
        if !(200..300).contains(&status) {
            let mut body = String::new();
            let mut drained = Box::pin(body_stream);
            while let Some(chunk) = drained.next().await {
                if let Ok(bytes) = chunk {
                    body.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
            let err = map_error_fn(status, body).await;
            yield Err(err);
            return;
        }

        let mut events = Box::pin(body_stream.eventsource());
        let mut processor = processor;

        while let Some(event_result) = events.next().await {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "sse frame parse error");
                    continue;
                }
            };
            if event.data.trim() == "[DONE]" {
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
                    yield Err(e);
                    return;
                }
            }
        }

        for chunk in processor.finish() {
            yield Ok(chunk);
        }
    })
}
