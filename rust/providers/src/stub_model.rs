//! Deterministic test-support completion adapter.

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

#[path = "stub_request.rs"]
mod request;
pub use request::last_request_bytes;

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
        Self::new()
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        request::serialize(&request)
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<RigCompletionResponse<StubCompletion>, CompletionError> {
        // Non-streaming path: return a single text choice. Used when
        // a caller invokes `agent.prompt(...)` instead of
        // `agent.stream_prompt(...)`. The cross-frontend parity tests
        // always go through the streaming path, but we keep
        // non-streaming faithful so future callers (e.g. PROV-103
        // token-usage smoke tests) work uniformly.
        request::record(&request)?;
        let text = compactor_dag(&request)?.unwrap_or_else(|| "hi back".to_string());
        Ok(RigCompletionResponse {
            choice: OneOrMany::one(AssistantContent::text(text)),
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
        request::record(&request)?;
        if let Some(call) = request::tool_call(&request)? {
            let rig_stream: StreamingResult<StubCompletion> = Box::pin(stream! {
                yield Ok(RawStreamingChoice::ToolCall(call));
                yield Ok(RawStreamingChoice::FinalResponse(StubCompletion {
                    stop_reason: "tool_use".into(),
                }));
            });
            return Ok(StreamingCompletionResponse::stream(rig_stream));
        }

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
        //
        // CMPCT-045: honor the deterministic-response hook (text
        // substitution) so a streaming sub-agent run can also be
        // test-controlled.
        // BUG-543: a test-set deterministic response wins; otherwise a
        // compactor request gets a covering DAG.
        let text = match deterministic_response() {
            Some(text) => text,
            None => compactor_dag(&request)?.unwrap_or_else(|| "hi back".to_string()),
        };
        let rig_stream: StreamingResult<StubCompletion> = Box::pin(stream! {
            yield Ok(RawStreamingChoice::Message(text));
            yield Ok(RawStreamingChoice::FinalResponse(StubCompletion::end_turn()));
        });

        Ok(StreamingCompletionResponse::stream(rig_stream))
    }
}

/// BUG-543: answer a compactor sub-agent request with a valid summary DAG.
///
/// The compactor task names its target ("Target session: <id>") and the
/// number of captured turns ("This operation covers exactly N ..."); the
/// stub replies with one D1 node covering turns 0..N-1, so a stub-backed
/// session can run a real manual compaction end to end. Any other request
/// keeps the canned reply.
fn compactor_dag(request: &CompletionRequest) -> Result<Option<String>, CompletionError> {
    let text = String::from_utf8_lossy(&request::serialize(request)?).into_owned();
    if !text.contains("Target session: ") {
        return Ok(None);
    }
    let covered = text
        .split("This operation covers exactly ")
        .nth(1)
        .and_then(|tail| tail.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|count| count.parse::<usize>().ok())
        .filter(|count| *count > 0);
    Ok(covered.map(|count| {
        format!(
            "<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Stub summary\">\nThe stub provider summarised the earlier conversation.\n</dag-node>",
            count - 1
        )
    }))
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
type LoopingStreamHook =
    std::sync::LazyLock<std::sync::Mutex<Option<(Vec<String>, Arc<AtomicUsize>)>>>;

static LOOPING_STREAM_HOOK: LoopingStreamHook =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

/// CMPCT-045: test-only DETERMINISTIC completion-response hook.
///
/// When set, BOTH the non-streaming `completion()` and the streaming
/// `stream()` paths return this fixed text (wrapped in the
/// compactor-sub-agent's final-response shape) instead of the canned
/// "hi back". Lets a behavioral test drive the real compactor sub-agent
/// spawner through `CustomProvider::create_rig_agent` with a
/// deterministic, parseable DAG output (success path) or unparseable
/// text (fallback path) — the compactor sub-agent NEVER uses the agent
/// loop's streaming driver.
static DETERMINISTIC_RESPONSE: std::sync::LazyLock<std::sync::Mutex<Option<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

/// CMPCT-045: install a fixed completion response for the next (and all
/// subsequent) stub completions. Test-only; best-effort (a poisoned lock
/// is a no-op).
pub fn set_deterministic_response(text: String) {
    if let Ok(mut guard) = DETERMINISTIC_RESPONSE.lock() {
        *guard = Some(text);
    }
}

/// CMPCT-045: clear the fixed completion response (restores "hi back").
pub fn clear_deterministic_response() {
    if let Ok(mut guard) = DETERMINISTIC_RESPONSE.lock() {
        *guard = None;
    }
}

fn deterministic_response() -> Option<String> {
    DETERMINISTIC_RESPONSE.lock().ok().and_then(|g| g.clone())
}

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
