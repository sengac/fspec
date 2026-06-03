//! RPC-007: Deterministic test-only LLM stub provider.
//! RPC-066: Widened with `impl LlmProvider for StubProvider` and a
//!          process-global registration helper so the cross-frontend
//!          parity test can drive the real SessionManager + agent loop
//!          against deterministic chunks without any network egress.
//!
//! Gated entirely behind the `test-support` Cargo feature so production
//! builds never compile a stub provider into release artifacts.
//!
//! Architecture notes [B], [C], [J], [L] from RPC-066.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use async_trait::async_trait;
use codelet_common::{Message, MessageContent};
use codelet_rpc_types::StreamChunk;
use codelet_tools::ToolDefinition;

use crate::{CompletionResponse, LlmProvider, ProviderError, StopReason};

/// Minimal deterministic provider used by RPC-007 cross-transport tests.
///
/// RPC-007: exposes [`StubProvider::canned_chunks`] for the
/// cross-transport parity tests that drive chunks directly into the
/// shared service.
///
/// RPC-066: also implements [`LlmProvider`] so the cross-frontend
/// parity test can route through the full SessionManager + agent loop
/// without any network I/O.
#[derive(Debug, Default, Clone)]
pub struct StubProvider;

impl StubProvider {
    pub fn new() -> Self {
        Self
    }

    /// Return the deterministic chunk sequence emitted on any input.
    pub fn canned_chunks() -> Vec<StreamChunk> {
        vec![StreamChunk::text("hi back".to_string()), StreamChunk::done()]
    }

    /// RPC-069: Build a real `rig::agent::Agent<StubModel>` so the
    /// agent-loop dispatch macro can stream through
    /// `codelet_cli::interactive::run_agent_stream_with_images` without
    /// special-casing the stub. Mirrors the signature of every other
    /// provider's inherent `create_rig_agent` (see
    /// [`crate::ClaudeProvider::create_rig_agent`],
    /// [`crate::custom::CustomProvider::create_rig_agent`], etc.) so
    /// the agent-loop's `"stub" =>` arm can construct it uniformly.
    ///
    /// # Arguments
    ///
    /// * `session_id` — session UUID. Unused by the stub model itself
    ///   but accepted for signature parity with real providers (real
    ///   providers thread this through `TOOL-012` / `TOOL-014` to bind
    ///   tools to session-scoped state).
    /// * `preamble` — optional system prompt. Forwarded to the rig
    ///   `AgentBuilder` so the stub's chat history still reflects the
    ///   preamble shape real providers would see; the stub model
    ///   itself ignores all message content and always emits
    ///   `"hi back"`.
    /// * `_thinking_config` — optional thinking-budget JSON. Accepted
    ///   for parity with the agent-loop dispatch macro's call site but
    ///   currently unused — the stub has no thinking primitive (per
    ///   RPC-069 architecture assumption: silence on reasoning chunks
    ///   is the Rust-pinned golden baseline).
    ///
    /// # Returns
    ///
    /// A fully built `rig::agent::Agent<StubModel>` ready to be
    /// wrapped in `codelet_core::RigAgent::with_default_depth` and
    /// fed into `run_agent_stream_with_images`. No tools are attached
    /// — the stub never returns `ToolUse`, so a tool surface would be
    /// dead weight and would only obscure the canned chunk stream.
    #[must_use]
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        _thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<crate::stub_model::StubModel> {
        let _ = session_id;
        let model = crate::stub_model::StubModel::new();
        let mut builder = rig::agent::AgentBuilder::new(model);
        if let Some(p) = preamble {
            if !p.is_empty() {
                builder = builder.preamble(p);
            }
        }
        builder.build()
    }
}

#[async_trait]
impl LlmProvider for StubProvider {
    fn name(&self) -> &str {
        "stub"
    }

    fn model(&self) -> &str {
        "canned"
    }

    fn context_window(&self) -> usize {
        200_000
    }

    fn max_output_tokens(&self) -> usize {
        4_096
    }

    fn supports_caching(&self) -> bool {
        false
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn complete(&self, _messages: &[Message]) -> Result<String, ProviderError> {
        Ok("hi back".to_string())
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        // RPC-066: scripted-run branch — inputs containing
        // "trigger-tool" emit a deterministic text response so the
        // agent loop reaches the StopReason::EndTurn exit and the
        // captured stream stays compact.
        //
        // Note: the original architecture note [B] called for a
        // MessageContent::ToolUse return shape. That requires
        // codelet_common::ContentPart::ToolUse + ToolResultPart wiring
        // plus a registered `noop_tool` in codelet_tools (architecture
        // note [L]). Per architecture note [I] (risk acknowledgement),
        // both deliverables are deferred to a sibling card under
        // RPC-030 — RPC-067 is dependency-rule regression tests and
        // RPC-068 is the final TS-frontend regression audit, so neither
        // addresses this wiring. A new sibling card must be created to
        // close out architecture notes [B] and [L].
        let _ = messages;
        Ok(CompletionResponse {
            content: MessageContent::Text("hi back".to_string()),
            stop_reason: StopReason::EndTurn,
        })
    }
}

// ---------------------------------------------------------------------
// RPC-066: in-memory registry for the stub provider.
// ---------------------------------------------------------------------

/// Process-global registry mapping a slug to a stub `LlmProvider`
/// instance. Populated by [`register_stub_provider`] (idempotent).
///
/// Architecture note [J]: this is a NEW slot — the existing
/// `custom_provider_registered` (`crate::manager::custom_provider_registered`)
/// only consults disk-resident JSON. The manager-side lookup is
/// extended to consult this map as well so `ProviderType::Custom("stub")`
/// resolves at session-creation time without any file I/O.
static STUB_REGISTRY: OnceLock<RwLock<HashMap<String, Arc<dyn LlmProvider>>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<String, Arc<dyn LlmProvider>>> {
    STUB_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// RPC-066: idempotently register the deterministic stub provider
/// under the slug `"stub"`. Safe to call from multiple threads and
/// from multiple boot paths; only the first call has any effect.
///
/// After this call:
///   - [`is_stub_registered`] returns `true`.
///   - The manager's `custom_provider_registered("stub")` returns
///     `true` (extended in `crate::manager` to consult this map).
///   - `ProviderType::from_str("stub")` returns
///     `Ok(ProviderType::Custom("stub"))`.
pub fn register_stub_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Ok(mut map) = registry().write() else {
            return;
        };
        map.insert("stub".to_string(), Arc::new(StubProvider::new()));
    });
}

/// RPC-066: returns `true` when the given slug has a stub provider
/// installed in the in-memory registry. Used by
/// `crate::manager::custom_provider_registered` to extend the disk-only
/// discovery with the in-memory map.
pub fn is_stub_registered(slug: &str) -> bool {
    let map = match registry().read() {
        Ok(g) => g,
        Err(_) => return false,
    };
    map.contains_key(slug)
}

/// RPC-066: borrow the registered stub provider for a slug, if any.
/// Returned as `Arc<dyn LlmProvider>` so the manager's match arm can
/// dispatch through it.
pub fn get_stub_provider(slug: &str) -> Option<Arc<dyn LlmProvider>> {
    let map = registry().read().ok()?;
    map.get(slug).cloned()
}
