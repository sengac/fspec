//! RPC-069: `rig::completion::CompletionModel` adapter for the
//! deterministic stub provider.
//!
//! [`StubModel`] is the keystone type that lets the agent-loop dispatch
//! macro treat the test-only stub provider exactly like any other
//! built-in provider — same `rig::agent::Agent<M>` shape, same
//! `RigAgent::with_default_depth` wrap, same
//! `run_agent_stream_with_images` driver. Without this adapter, the
//! `Arc<dyn LlmProvider>` returned by
//! [`crate::stub_provider::get_stub_provider`] cannot reach the
//! streaming agent driver because `create_rig_agent` is an inherent
//! method (not a trait method) on every other provider type.
//!
//! ## Wiring
//!
//! - `completion()` returns a deterministic
//!   `OneOrMany::one(AssistantContent::text("hi back"))` choice with a
//!   default token usage. Used by rig's non-streaming code path.
//! - `stream()` yields a `RawStreamingChoice::Message("hi back")` chunk
//!   followed by `RawStreamingChoice::FinalResponse(StubCompletion)` so
//!   the stream terminates with a clean `StopReason::EndTurn`. Used by
//!   the standard `run_agent_stream_with_images` driver.
//!
//! Gated entirely behind the `test-support` Cargo feature so production
//! builds never compile this module into release artifacts.
//!
//! Modelled after [`crate::custom::rig_model::RhaiCustomProviderModel`]
//! but with no HTTP, no Rhai engine, no tool flow — just a canned
//! `[Text("hi back"), FinalResponse]` stream.

use async_stream::stream;
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse as RigCompletionResponse, GetTokenUsage, Usage,
};
use rig::message::AssistantContent;
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse, StreamingResult};
use rig::OneOrMany;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

/// Raw-response payload exposed to rig consumers.
///
/// Carries the stop reason (always `"end_turn"` for the stub) plus
/// zero-valued token counts so [`GetTokenUsage::token_usage`] returns a
/// valid (but empty) `Usage`. Mirrors the shape of
/// [`crate::custom::rig_model::RhaiCustomCompletion`] minus the
/// Rhai-specific cache/reasoning fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StubCompletion {
    /// Stop reason — always `"end_turn"` for the deterministic stub.
    #[serde(default)]
    pub stop_reason: String,
}

impl StubCompletion {
    /// Build a terminal stub completion with `stop_reason = "end_turn"`.
    fn end_turn() -> Self {
        Self {
            stop_reason: "end_turn".to_string(),
        }
    }
}

impl GetTokenUsage for StubCompletion {
    fn token_usage(&self) -> Option<Usage> {
        // The stub never consumes or emits tokens. Return a fresh
        // zero-valued `Usage` so rig's aggregator treats this as
        // "provider supplied empty metrics" rather than `None` (which
        // is reserved for "provider failed to supply metrics").
        Some(Usage::new())
    }

    fn stop_reason(&self) -> Option<&str> {
        Some(self.stop_reason.as_str())
    }
}

/// Deterministic [`rig::completion::CompletionModel`] used by the
/// cross-frontend parity tests. Yields exactly one text chunk
/// (`"hi back"`) followed by a terminal `FinalResponse`.
#[derive(Debug, Default, Clone)]
pub struct StubModel;

impl StubModel {
    /// Construct a fresh [`StubModel`]. Zero-sized; every instance is
    /// identical so this is effectively a constant.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl CompletionModel for StubModel {
    type Response = StubCompletion;
    type StreamingResponse = StubCompletion;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>) -> Self {
        // rig's [`CompletionModel::make`] factory signature
        // (`-> Self`) forces an infallible constructor. The stub model
        // is constructed exclusively via
        // [`crate::stub_provider::StubProvider::create_rig_agent`], so
        // this factory is unreachable via the public API. Mirrors the
        // same pattern as
        // [`crate::custom::rig_model::RhaiCustomProviderModel::make`].
        #[allow(clippy::panic)]
        {
            panic!(
                "StubModel must be constructed via StubProvider::create_rig_agent — \
                 rig's `make` factory is not supported because the stub model is \
                 not parameterised by a client."
            );
        }
    }

    async fn completion(
        &self,
        _request: CompletionRequest,
    ) -> Result<RigCompletionResponse<StubCompletion>, CompletionError> {
        // Non-streaming path: return a single text choice. Used when
        // a caller invokes `agent.prompt(...)` instead of
        // `agent.stream_prompt(...)`. The cross-frontend parity tests
        // always go through the streaming path, but we keep
        // non-streaming faithful so future callers (e.g. PROV-103
        // token-usage smoke tests) work uniformly.
        Ok(RigCompletionResponse {
            choice: OneOrMany::one(AssistantContent::text("hi back")),
            usage: Usage::new(),
            raw_response: StubCompletion::end_turn(),
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<StubCompletion>, CompletionError> {
        // RIG-015: record the request chat history (test-only seam) so a
        // behavioral test can assert the NEXT turn's context actually
        // carries the loop-abort corrective note. Best-effort: a poisoned
        // lock is a no-op.
        record_request_history(&request);

        // RIG-015: if the test-only looping stream hook is active, yield
        // each configured word as a separate text delta (incrementing the
        // shared poll counter) followed by a FinalResponse. This lets a
        // behavioral test drive a real looping stream through the full
        // production agent loop and prove the RIG-014 detector abort
        // cancels the in-flight stream mid-way.
        let hook = LOOPING_STREAM_HOOK.lock().ok().and_then(|g| g.clone());
        if let Some((words, poll)) = hook {
            let rig_stream: StreamingResult<StubCompletion> = Box::pin(stream! {
                for word in words {
                    poll.fetch_add(1, Ordering::AcqRel);
                    yield Ok(RawStreamingChoice::Message(word));
                }
                yield Ok(RawStreamingChoice::FinalResponse(StubCompletion::end_turn()));
            });
            return Ok(StreamingCompletionResponse::stream(rig_stream));
        }

        // Streaming path: yield exactly the canned chunk sequence the
        // cross-frontend parity tests assert against. The text chunk
        // becomes a `StreamChunk::Text { text: "hi back", .. }`
        // downstream; the `FinalResponse` triggers rig's
        // end-of-stream bookkeeping and the agent-loop's per-turn
        // exit (which in turn emits the trailing `StreamChunk::Done`
        // via `BackgroundSession::handle_output`).
        let rig_stream: StreamingResult<StubCompletion> = Box::pin(stream! {
            yield Ok(RawStreamingChoice::Message("hi back".to_string()));
            yield Ok(RawStreamingChoice::FinalResponse(StubCompletion::end_turn()));
        });

        Ok(StreamingCompletionResponse::stream(rig_stream))
    }
}

// ---------------------------------------------------------------------
// RIG-015: test-only looping stream hook + request-history recording.
// ---------------------------------------------------------------------
//
// Both seams are process-global and test-only (this whole module is
// gated behind `test-support`). The hook lets a behavioral test make the
// stub emit a real looping stream (instead of the canned "hi back") so
// the RIG-014 detector can fire mid-stream; the poll counter proves the
// stream was actually cancelled (polled fewer times than its full
// length). The request-history recording lets a test assert the next
// turn's completion request actually carries the corrective note.

/// RIG-015: the active looping stream source, if set.
///
/// A `Mutex<Option<...>>` (NOT a `OnceLock`) so the hook can be set and
/// cleared repeatedly across tests — the whole module is test-only and the
/// behavioral tests run serially in one process.
type LoopingStreamHook = std::sync::LazyLock<
    std::sync::Mutex<Option<(Vec<String>, Arc<AtomicUsize>)>>,
>;

static LOOPING_STREAM_HOOK: LoopingStreamHook =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

/// RIG-015: the most recent completion-request chat history (rendered as
/// one string per message), recorded for test assertions.
static LAST_REQUEST_HISTORY: OnceLock<std::sync::Mutex<Vec<String>>> = OnceLock::new();

fn history_slot() -> &'static std::sync::Mutex<Vec<String>> {
    LAST_REQUEST_HISTORY.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// RIG-015: render a rig `Message` to a plain-text string (best-effort).
fn message_to_text(msg: &rig::message::Message) -> String {
    match msg {
        rig::message::Message::User { content } => {
            let mut out = String::new();
            for item in content.iter() {
                if let rig::message::UserContent::Text(t) = item {
                    out.push_str(&t.text);
                    out.push(' ');
                }
            }
            out
        }
        rig::message::Message::Assistant { content, .. } => {
            let mut out = String::new();
            for item in content.iter() {
                if let rig::message::AssistantContent::Text(t) = item {
                    out.push_str(&t.text);
                    out.push(' ');
                }
            }
            out
        }
    }
}

/// RIG-015: record the current request's chat history (best-effort).
fn record_request_history(request: &CompletionRequest) {
    let mut rendered = Vec::new();
    if let Some(preamble) = &request.preamble {
        rendered.push(preamble.clone());
    }
    for msg in request.chat_history.iter() {
        rendered.push(message_to_text(msg));
    }
    if let Ok(mut guard) = history_slot().lock() {
        *guard = rendered;
    }
}

/// RIG-015: activate the looping stream source. `words` are yielded one
/// per delta; `poll` is incremented per yielded delta so a test can prove
/// how far the stream was consumed before cancellation.
pub fn set_looping_stream_hook(words: Vec<String>, poll: Arc<AtomicUsize>) {
    if let Ok(mut guard) = LOOPING_STREAM_HOOK.lock() {
        *guard = Some((words, poll));
    }
}

/// RIG-015: clear the looping stream hook (restores the canned stream).
pub fn clear_looping_stream_hook() {
    if let Ok(mut guard) = LOOPING_STREAM_HOOK.lock() {
        *guard = None;
    }
}

/// RIG-015: the most recent completion-request chat history (one string
/// per message, preamble first if present). Empty when no request has
/// been recorded yet.
pub fn last_request_history() -> Vec<String> {
    history_slot()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}
