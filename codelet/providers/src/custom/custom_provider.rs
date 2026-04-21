//! PROV-067 + PROV-092: Generic `CustomProvider` agent construction for
//! custom providers with `facade = null`.
//!
//! When a custom provider does not declare a facade, the agent-loop
//! cannot route through an existing match arm; instead it goes through
//! this generic path, wiring:
//! - [`RhaiCustomProvider`] wrapped in [`RhaiCustomProviderModel`] as
//!   the `rig::completion::CompletionModel`,
//! - [`RhaiSystemPromptFacade`] for system-prompt composition (preamble
//!   transform + identity prefix), and
//! - [`RhaiToolWrapper`] instances around each [`RhaiToolFacadeAdapter`]
//!   so script-defined tools surface to rig's `ToolServer` with their
//!   dynamic names.
//!
//! [`CustomProvider::create_rig_agent`] returns a
//! [`CustomRigAgent`] handle whose [`CustomRigAgent::into_inner`] method
//! exposes the fully-built [`rig::agent::Agent<RhaiCustomProviderModel>`]
//! ready to wrap in `codelet_core::RigAgent` and stream through
//! `run_agent_stream_with_images`.

use std::sync::Arc;

use rig::agent::{Agent, AgentBuilder};

use super::config::ProviderConfig;
use super::discovery::discover_provider_configs;
use super::error::CustomProviderError;
use super::provider::RhaiCustomProvider;
use super::rig_model::RhaiCustomProviderModel;
use super::rig_tool::RhaiToolWrapper;
use super::script_loader::ScriptLoader;
use super::system_prompt::RhaiSystemPromptFacade;
use super::tool_facade::{RhaiToolDef, RhaiToolFacadeAdapter};
use super::tool_resolve::resolve_tools;
use crate::oauth::building_blocks::register_all_modules;
use crate::oauth::engine::build_sandboxed_engine;
use codelet_tools::facade::SystemPromptFacade;

/// Handle returned by [`CustomProvider::create_rig_agent`]. Carries the
/// real rig agent plus introspection metadata so existing tests can
/// continue to assert facade wiring without poking at rig's private
/// fields.
pub struct CustomRigAgent {
    provider_name: String,
    agent: Agent<RhaiCustomProviderModel>,
    system_prompt_facade: Arc<RhaiSystemPromptFacade>,
    tool_adapter_count: usize,
}

impl CustomRigAgent {
    /// Provider name as configured in the JSON.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// `true` — the generic path always wires a [`RhaiSystemPromptFacade`].
    pub fn uses_rhai_system_prompt_facade(&self) -> bool {
        let _ = &self.system_prompt_facade;
        true
    }

    /// Borrow the system-prompt facade (useful for direct rendering tests).
    pub fn system_prompt_facade(&self) -> &Arc<RhaiSystemPromptFacade> {
        &self.system_prompt_facade
    }

    /// `true` when at least one [`RhaiToolFacadeAdapter`] was resolved
    /// from the script's `define_tools` output or from the tool_style
    /// preset fallback.
    pub fn uses_rhai_tool_facade_adapter(&self) -> bool {
        self.tool_adapter_count > 0
    }

    /// Number of resolved tool adapters wired into the agent.
    pub fn tool_adapter_count(&self) -> usize {
        self.tool_adapter_count
    }

    /// Borrow the inner [`rig::agent::Agent`].
    pub fn agent(&self) -> &Agent<RhaiCustomProviderModel> {
        &self.agent
    }

    /// Consume the wrapper and return the inner [`rig::agent::Agent`].
    /// The agent-loop dispatch arm uses this to wrap the agent in
    /// `codelet_core::RigAgent` for streaming.
    pub fn into_inner(self) -> Agent<RhaiCustomProviderModel> {
        self.agent
    }
}

/// PROV-067 + PROV-092: Façade-null entry point. Builds the
/// Rhai-backed completion model, the system-prompt façade, the tool
/// wrappers, and assembles them into a real `rig::agent::Agent`.
pub struct CustomProvider;

impl CustomProvider {
    /// Construct a generic Rhai-backed custom provider agent.
    ///
    /// `project_root` anchors config-file resolution when the caller
    /// isn't running from the project's CWD. Discovery still honours
    /// the user-global + project-local override chain.
    ///
    /// `thinking_config` mirrors the shape of
    /// [`crate::ClaudeProvider::create_rig_agent`] so the agent-loop can
    /// thread adaptive-thinking configuration uniformly across providers.
    /// It is forwarded into the agent's `additional_params` so the
    /// rig-driven `CompletionRequest` carries it through to the Rhai
    /// `build_request` call (PROV-090).
    pub fn create_rig_agent(
        project_root: &std::path::Path,
        name: &str,
        model_alias: &str,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> Result<CustomRigAgent, CustomProviderError> {
        let configs = discover_provider_configs()?;
        let cfg = configs.into_iter().find(|c| c.name == name).ok_or_else(|| {
            CustomProviderError::RhaiRuntimeError(format!(
                "custom provider '{name}' not discovered"
            ))
        })?;
        let config_dir = find_config_dir(project_root, name).ok_or_else(|| {
            CustomProviderError::RhaiRuntimeError(format!(
                "could not locate config directory for '{name}'"
            ))
        })?;
        let mut cfg = cfg;
        if !cfg.script.is_empty() {
            let resolved = config_dir.join(&cfg.script);
            cfg.script = resolved.to_string_lossy().into_owned();
        }

        let loader = build_loader();
        let backend = Arc::new(RhaiCustomProvider::new(
            Arc::new(cfg.clone()),
            loader.clone(),
            model_alias.to_string(),
        )?);

        let tool_defs: Vec<RhaiToolDef> = resolve_tools(&mut cfg, &loader)?;
        let config_arc = Arc::new(cfg);
        let tool_adapters: Vec<RhaiToolFacadeAdapter> = tool_defs
            .into_iter()
            .filter_map(|def| {
                RhaiToolFacadeAdapter::new(Arc::new(def), Arc::clone(&config_arc), loader.clone())
                    .ok()
            })
            .collect();
        let tool_adapter_count = tool_adapters.len();

        let system_prompt_facade = Arc::new(build_system_prompt_facade(
            name,
            &config_arc,
            loader.clone(),
        )?);

        let agent = build_rig_agent(
            backend.clone(),
            session_id,
            preamble,
            thinking_config,
            &system_prompt_facade,
            tool_adapters,
            &config_arc,
        );

        Ok(CustomRigAgent {
            provider_name: name.to_string(),
            agent,
            system_prompt_facade,
            tool_adapter_count,
        })
    }
}

/// Build the rig agent: wrap the backend in [`RhaiCustomProviderModel`],
/// compose the preamble through the system-prompt facade, attach every
/// [`RhaiToolWrapper`] as a static tool, and merge `thinking_config` into
/// the agent's `additional_params` so it round-trips through every rig
/// `CompletionRequest`.
fn build_rig_agent(
    backend: Arc<RhaiCustomProvider>,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
    system_prompt_facade: &Arc<RhaiSystemPromptFacade>,
    tool_adapters: Vec<RhaiToolFacadeAdapter>,
    config: &Arc<ProviderConfig>,
) -> Agent<RhaiCustomProviderModel> {
    let model = RhaiCustomProviderModel::new(backend);
    let mut agent_builder = AgentBuilder::new(model);

    // Preamble: facade transform_preamble first, then prefix the
    // identity prefix if the script declares one.
    let preamble_text = preamble.unwrap_or("");
    let transformed = system_prompt_facade.transform_preamble(preamble_text);
    let effective_preamble = match system_prompt_facade.identity_prefix() {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}\n{transformed}"),
        _ => transformed,
    };
    agent_builder = agent_builder.preamble(&effective_preamble);

    // additional_params carries thinking_config — RhaiCustomProviderModel
    // forwards it into the Rhai build_request call as request.thinking_config.
    if let Some(thinking) = thinking_config {
        agent_builder = agent_builder.additional_params(thinking);
    }

    // Attach every script-defined tool. AgentBuilder::tool consumes
    // the AgentBuilder and returns an AgentBuilderSimple. Subsequent
    // .tool() calls append to that simple builder.
    //
    // PROV-098: In addition to the Rhai-customisable surface (the
    // nine baseline preset tools — Read/Write/Edit/Bash/Grep/Glob/LS/
    // AstGrep/WebSearch — whose schemas users can override via the
    // script's `define_tools`), we also attach the fspec "infrastructure"
    // tools directly as native `rig::tool::Tool` implementations. These
    // tools are tied to session-scoped state (agent sessions, session
    // search index, graph store, scheduler, HITL, MCP connections) and
    // are not customisable via Rhai — attaching them directly mirrors
    // the full tool surface that `ClaudeProvider::create_rig_agent` and
    // every other built-in provider exposes, so a `claude-rhai` user
    // gets parity with a native `claude` user.
    //
    // PROV-099: Fspec and Bridge tools are also attached here. These
    // are facade-wrapped because their tool name, schema, and argument
    // mapping differ per provider family (Claude uses `Fspec` +
    // `Bridge`, Gemini uses `fspec_command` + `bridge_event`, etc.).
    // We pick the facade based on the config's `tool_style` so that a
    // `tool_style = "claude"` custom provider advertises `Fspec`/
    // `Bridge` exactly the same way the native `ClaudeProvider` does.
    use codelet_tools::facade::{
        bridge_tool_for_provider, claude_bridge_tool, claude_fspec_tool, fspec_tool_for_provider,
    };
    use codelet_tools::{
        AgentManagerTool, AstGrepRefactorTool, ConnectMcpTool, DeepSearchTool, GraphSearchTool,
        InjectSummaryTool, RequestUserInputTool, ScheduleTool, SessionSearchTool,
    };

    // Map ToolStyle → provider string used by the facade registrations.
    // Anthropic is an alias for Claude (same rationale as
    // `tool_presets::preset_tools`).
    let facade_provider: &str = match config.tool_style {
        super::config::ToolStyle::Claude | super::config::ToolStyle::Anthropic => "claude",
        super::config::ToolStyle::Openai => "openai",
        super::config::ToolStyle::Gemini => "gemini",
        super::config::ToolStyle::Codex => "codex",
    };
    let fspec_tool = fspec_tool_for_provider(facade_provider, session_id)
        .unwrap_or_else(|| claude_fspec_tool(session_id));
    let bridge_tool = bridge_tool_for_provider(facade_provider, session_id)
        .unwrap_or_else(|| claude_bridge_tool(session_id));

    let mut iter = tool_adapters.into_iter();
    if let Some(first) = iter.next() {
        let mut simple = agent_builder.tool(RhaiToolWrapper::new(first, session_id));
        for adapter in iter {
            simple = simple.tool(RhaiToolWrapper::new(adapter, session_id));
        }
        simple
            .tool(AstGrepRefactorTool::new(session_id))
            .tool(SessionSearchTool::new(session_id))
            .tool(GraphSearchTool::new(session_id))
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id))
            .tool(AgentManagerTool::new(session_id))
            .tool(RequestUserInputTool::new(session_id))
            .tool(ScheduleTool::new(session_id))
            .tool(ConnectMcpTool::new(session_id))
            .tool(fspec_tool)
            .tool(bridge_tool)
            .build()
    } else {
        // No Rhai-defined tools at all — wire the infrastructure tools
        // on their own. This branch should be rare (an empty
        // `define_tools` return) but we must still expose the native
        // surface so the agent has something to work with.
        agent_builder
            .tool(AstGrepRefactorTool::new(session_id))
            .tool(SessionSearchTool::new(session_id))
            .tool(GraphSearchTool::new(session_id))
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id))
            .tool(AgentManagerTool::new(session_id))
            .tool(RequestUserInputTool::new(session_id))
            .tool(ScheduleTool::new(session_id))
            .tool(ConnectMcpTool::new(session_id))
            .tool(fspec_tool)
            .tool(bridge_tool)
            .build()
    }
}

/// Locate the directory containing `<name>.json` — project-local first,
/// then user-global. Mirrors [`crate::custom::discover_provider_configs`]
/// precedence but returns just the directory.
fn find_config_dir(project_root: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let local = project_root.join(".fspec").join("providers");
    if local.join(format!("{name}.json")).is_file() {
        return Some(local);
    }
    let home = std::env::var("FSPEC_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .and_then(|p| {
            if p.file_name().map(|n| n == "credentials").unwrap_or(false) {
                p.parent().map(std::path::Path::to_path_buf)
            } else {
                Some(p)
            }
        })
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".fspec"))
        });
    if let Some(base) = home {
        let global = base.join("providers");
        if global.join(format!("{name}.json")).is_file() {
            return Some(global);
        }
    }
    None
}

fn build_loader() -> Arc<ScriptLoader> {
    let engine = build_sandboxed_engine(register_all_modules());
    Arc::new(ScriptLoader::new(engine))
}

fn build_system_prompt_facade(
    name: &str,
    config: &Arc<ProviderConfig>,
    loader: Arc<ScriptLoader>,
) -> Result<RhaiSystemPromptFacade, CustomProviderError> {
    let script_path = std::path::PathBuf::from(&config.script);
    let ast = loader.load(&script_path)?;
    let engine = loader.engine_arc();
    let config_dyn = rhai::Dynamic::from_map({
        let mut map = rhai::Map::new();
        map.insert("name".into(), rhai::Dynamic::from(config.name.clone()));
        map.insert(
            "base_url".into(),
            rhai::Dynamic::from(config.base_url.clone()),
        );
        map
    });
    Ok(RhaiSystemPromptFacade::new(
        name.to_string(),
        engine,
        ast,
        config_dyn,
    ))
}
