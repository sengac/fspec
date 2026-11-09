//! OpenAIProvider model discovery, client access and rig agent construction
//! (REFAC-031: moved from openai.rs).

use super::{anti_ramble_additional_params, discovery, OpenAIProvider, ANTI_RAMBLE_TEMPERATURE};
use crate::ProviderError;
use rig::providers::openai;

impl OpenAIProvider {
    /// Fetch available models from a local OpenAI-compatible server
    ///
    /// Makes HTTP GET request to {base_url}/v1/models and parses the response.
    /// Returns a list of model IDs available on the server.
    ///
    /// PROV-006: This method is used by the TUI to populate the model selection dialog
    /// when OPENAI_BASE_URL is set.
    ///
    /// PROV-040: Added optional api_key parameter for servers that require authentication
    /// (e.g., Fireworks AI, Together AI). Local servers (vLLM, Ollama) typically don't
    /// require auth for the /models endpoint.
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the local server (e.g., "http://localhost:8888")
    /// * `api_key` - Optional API key for servers requiring authentication
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of model IDs
    /// * `Err(ProviderError)` - If server is unreachable or response is invalid
    pub async fn list_local_models(base_url: &str) -> Result<Vec<String>, ProviderError> {
        Self::list_local_models_with_auth(base_url, None).await
    }

    /// Fetch available models from an OpenAI-compatible server with optional authentication
    ///
    /// PROV-040: Same as list_local_models but accepts an optional API key for servers
    /// that require authentication (Fireworks AI, Together AI, etc.).
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the server (e.g., "https://api.fireworks.ai/inference")
    /// * `api_key` - Optional API key for servers requiring authentication
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of model IDs
    /// * `Err(ProviderError)` - If server is unreachable or response is invalid
    pub async fn list_local_models_with_auth(
        base_url: &str,
        api_key: Option<&str>,
    ) -> Result<Vec<String>, ProviderError> {
        discovery::list_models(base_url, api_key).await
    }

    /// Get the configured rig openai::CompletionsClient
    ///
    /// This client is fully configured with:
    /// - API key authentication
    /// - Custom base URL (if OPENAI_BASE_URL is set)
    ///
    /// Use this client to create rig agents with tools.
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Create a rig Agent with all 10 tools configured for this provider
    ///
    /// This method encapsulates all OpenAI-specific configuration:
    /// - Model name (gpt-4-turbo or custom)
    /// - Max tokens (OPENAI_MAX_OUTPUT_TOKENS or the session's resolved limit, when declared)
    /// - All 10 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, AstGrepRefactor, WebSearchTool)
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    /// * `thinking_config` - Optional thinking parameters (BUG-440: `reasoning_effort`), merged into the request
    ///
    /// # WEB-001 Note
    /// WebSearchTool is now included as a rig tool that provides web search capabilities
    /// with Search, OpenPage, and FindInPage actions.
    ///
    /// # TOOL-012 Note
    /// Session ID is now required as the first parameter. This ensures tools know which
    /// session's handler to use at call time, eliminating reliance on thread-local state.
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::facade::{openai_bridge_tool, openai_fspec_tool};
        use codelet_tools::{
            AgentManagerTool, AstGrepRefactorTool, AstGrepTool, BashTool, ConnectMcpTool,
            DecisionTool, DeepSearchTool, EditTool, GenerateCompactionTool, GlobTool,
            GraphSearchTool, GrepTool, InjectSummaryTool, LsTool, ReadTool, RequestUserInputTool,
            ScheduleTool, SessionSearchTool, WebSearchTool, WriteTool,
        };

        // Build agent with all 11 tools using rig's builder pattern (WEB-001: Added WebSearchTool)
        // TOOL-012: Pass session_id to fspec and bridge tools
        // TOOL-014: All tools require session_id for worktree isolation
        // PROV-140: build the completion model explicitly so the per-profile
        // streaming flag is applied to the agent's model (not just re-read from
        // env), keeping streaming vs non-streaming selection deterministic.
        let mut agent_builder =
            openai::completion::CompletionModel::new(self.rig_client.clone(), &self.model_name)
                .with_stream(self.streaming)
                .into_agent_builder()
                .tool(ReadTool::new(session_id))
                .tool(WriteTool::new(session_id))
                .tool(EditTool::new(session_id))
                .tool(BashTool::new(session_id))
                .tool(GrepTool::new(session_id))
                .tool(GlobTool::new(session_id))
                .tool(LsTool::new(session_id))
                .tool(AstGrepTool::new(session_id)) // TOOL-014: AstGrepTool with session_id for worktree isolation
                .tool(AstGrepRefactorTool::new(session_id)) // TOOL-014: AstGrepRefactorTool with session_id for worktree isolation
                .tool(openai_fspec_tool(session_id)) // TOOL-012: FspecTool with explicit session association
                .tool(openai_bridge_tool(session_id)) // TOOL-012: BridgeTool with explicit session association
                .tool(WebSearchTool::new(session_id)) // WEB-001, TOOL-014: WebSearchTool with session_id
                .tool(ConnectMcpTool::new(session_id)) // MCP-001: Dynamic MCP connections
                .tool(SessionSearchTool::new(session_id)) // AMGR-001: SessionSearch tool
                .tool(GraphSearchTool::new(session_id)) // KGRAPH-003: GraphSearch tool
                .tool(InjectSummaryTool::new(session_id))
                .tool(DeepSearchTool::new(session_id)) // RLM-001: DeepSearch tool
                .tool(DecisionTool::new(session_id)) // RLCD-002: Decision tool
                .tool(GenerateCompactionTool::new(session_id)) // CMPCT-045: GenerateCompaction tool
                .tool(AgentManagerTool::new(session_id)) // AMGR-009: AgentManager tool
                .tool(RequestUserInputTool::new(session_id)) // TOOL-017: HITL tool
                .tool(ScheduleTool::new(session_id)); // SCHED-009: Schedule AI tool

        // PROV-171: cap the output only with a declared limit; the session path
        // (`ProviderManager::get_openai`) refuses to build without one.
        if let Some(max_output_tokens) = self.max_output_tokens {
            agent_builder = agent_builder.max_tokens(max_output_tokens as u64);
        }

        // CONT-002: done() is registered only while auto-continue is armed
        if codelet_tools::is_continue_armed(session_id) {
            agent_builder = agent_builder.tool(codelet_tools::DoneTool::new(session_id));
        }

        // BUG-120: Always set a system prompt with fspec guidance.
        // When a role is provided, it is appended after fspec guidance.
        // Uses the same transform as OpenAISystemPromptFacade for consistency.
        {
            use codelet_tools::facade::prepend_fspec_guidance;
            let preamble_text = preamble.unwrap_or("");
            let effective_preamble = prepend_fspec_guidance(preamble_text);
            agent_builder = agent_builder.preamble(&effective_preamble);
        }

        // Anti-ramble sampling for local endpoints (Ollama / vLLM / LM Studio /
        // LocalAI / any OpenAI-compatible local server). Cloud OpenAI keeps
        // provider defaults so behaviour there is unchanged.
        let mut params = serde_json::json!({});
        if self.is_local_endpoint() {
            agent_builder = agent_builder.temperature(ANTI_RAMBLE_TEMPERATURE);
            params = anti_ramble_additional_params();
        }
        // BUG-440: the selected thinking level (`reasoning_effort`).
        let params = super::merge_thinking_params(params, thinking_config);
        if params != serde_json::json!({}) {
            agent_builder = agent_builder.additional_params(params);
        }

        agent_builder.build()
    }
}
