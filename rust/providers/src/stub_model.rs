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
        _request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<StubCompletion>, CompletionError> {
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
