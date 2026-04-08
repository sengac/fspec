//! `CopilotProvider::create_rig_agent` — rig agent builder for the NAPI
//! agent loop dispatch macro (PROV-057 Layer 2).
//!
//! This module keeps the heavy tool-wiring logic out of `provider.rs` so
//! that the composition file stays close to the 300-line SoC budget set by
//! the PROV-053 rules. The single public entry point here is
//! [`CopilotProvider::create_rig_agent`], which mirrors the signature of
//! every other provider in this crate
//! ([`ClaudeProvider::create_rig_agent`](crate::ClaudeProvider::create_rig_agent),
//! [`OpenAIProvider::create_rig_agent`](crate::OpenAIProvider::create_rig_agent),
//! [`GeminiProvider::create_rig_agent`](crate::GeminiProvider::create_rig_agent),
//! etc.) so the NAPI agent loop can dispatch the `github-copilot` arm of
//! its `run_with_provider!` macro without special-casing.
//!
//! # Contract
//!
//! - The returned agent is specialised to
//!   `openai::completion::CompletionModel<CopilotHttpClient>` so every
//!   outbound request is routed through the Copilot HTTP middleware, which
//!   in turn re-injects the refreshed Bearer token built by
//!   [`CopilotProvider::ensure_fresh_copilot_token`].
//! - Copilot is OpenAI-wire-compatible for the `/chat/completions` path, so
//!   tool wiring mirrors [`OpenAIProvider::create_rig_agent`] exactly —
//!   same 18 tools, same `openai_fspec_tool` / `openai_bridge_tool`
//!   facades, same `prepend_fspec_guidance` preamble transform.
//! - `thinking_config` is accepted for parity with the agent loop macro
//!   but is currently unused — Copilot's chat/completions endpoint does
//!   not expose a thinking budget primitive.

use crate::copilot::provider::CopilotProvider;
use crate::copilot::refreshing_client::CopilotHttpClient;
use rig::providers::openai;

impl CopilotProvider {
    /// Create a rig `Agent` wired with the full fspec tool set for use by
    /// the NAPI agent loop dispatch macro (`run_with_provider!`).
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session UUID every tool must be scoped to
    ///   (TOOL-012 / TOOL-014 — tools look up their callback handler and
    ///   worktree by `session_id` at call time).
    /// * `preamble` - Optional system prompt to append after the fspec
    ///   guidance header. `None` means "fspec guidance only".
    /// * `_thinking_config` - Optional thinking configuration JSON (TOOL-010).
    ///   Accepted for parity with the macro's call site but currently
    ///   unused — Copilot's chat/completions endpoint has no thinking
    ///   budget primitive.
    ///
    /// # Returns
    ///
    /// A fully built `rig::agent::Agent` parameterised by
    /// `openai::completion::CompletionModel<CopilotHttpClient>`, ready to
    /// be wrapped in `RigAgent` by the dispatch macro.
    #[must_use]
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        _thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel<CopilotHttpClient>> {
        use codelet_tools::facade::{
            openai_bridge_tool, openai_fspec_tool, prepend_fspec_guidance,
        };
        use codelet_tools::{
            AgentManagerTool, AstGrepRefactorTool, AstGrepTool, BashTool, ConnectMcpTool,
            DeepSearchTool, EditTool, GlobTool, GraphSearchTool, GrepTool, InjectSummaryTool,
            LsTool, ReadTool, RequestUserInputTool, ScheduleTool, SessionSearchTool, WebSearchTool,
            WriteTool,
        };
        use rig::client::CompletionClient;

        // Build the agent against the provider's rig client — NOT a fresh
        // one — so every outbound request flows through `CopilotHttpClient`
        // and therefore honours the refreshed Copilot Bearer token.
        //
        // Tool set mirrors OpenAIProvider::create_rig_agent because Copilot
        // is OpenAI-wire-compatible on the /chat/completions path. If the
        // tool set here drifts from the OpenAI provider's, the two paths
        // will silently disagree about which tools are available — keep
        // them in lock-step.
        let mut agent_builder = self
            .client()
            .agent(self.model())
            .max_tokens(crate::copilot::MAX_OUTPUT_TOKENS as u64)
            .tool(ReadTool::new(session_id))
            .tool(WriteTool::new(session_id))
            .tool(EditTool::new(session_id))
            .tool(BashTool::new(session_id))
            .tool(GrepTool::new(session_id))
            .tool(GlobTool::new(session_id))
            .tool(LsTool::new(session_id))
            .tool(AstGrepTool::new(session_id))
            .tool(AstGrepRefactorTool::new(session_id))
            .tool(openai_fspec_tool(session_id))
            .tool(openai_bridge_tool(session_id))
            .tool(WebSearchTool::new(session_id))
            .tool(ConnectMcpTool::new(session_id))
            .tool(SessionSearchTool::new(session_id))
            .tool(GraphSearchTool::new(session_id))
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id))
            .tool(AgentManagerTool::new(session_id))
            .tool(RequestUserInputTool::new(session_id))
            .tool(ScheduleTool::new(session_id));

        // Match OpenAIProvider's BUG-120 behaviour: always prepend fspec
        // guidance so the agent loop has a stable system prompt contract
        // regardless of whether the caller supplied a role preamble.
        let preamble_text = preamble.unwrap_or("");
        let effective_preamble = prepend_fspec_guidance(preamble_text);
        agent_builder = agent_builder.preamble(&effective_preamble);

        agent_builder.build()
    }
}

// Trait import for `provider.model()` inside this impl block — needs to be
// in scope so the `self.model()` call resolves to `LlmProvider::model`.
use crate::LlmProvider;
