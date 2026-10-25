use crate::background_output::BackgroundOutput;
use codelet_cli::{interactive::BridgeImage, session::Session};
use codelet_sessions::background_session::BackgroundSession;

pub(super) async fn run_custom_turn(
    inner_session: &mut Session,
    input: &str,
    bridge_images: Option<Vec<BridgeImage>>,
    session: &BackgroundSession,
    output: &BackgroundOutput,
    thinking_config_value: Option<serde_json::Value>,
    current_provider: &str,
    current_model: Option<String>,
) -> anyhow::Result<()> {
    let project_root = std::path::PathBuf::from(&session.project);
    let model_alias = current_model.unwrap_or_else(|| "default".to_string());
    let role_preamble = session.get_role();
    let agent_result = codelet_providers::custom::CustomProvider::create_rig_agent(
        &project_root,
        current_provider,
        &model_alias,
        session.id,
        role_preamble.as_deref(),
        thinking_config_value,
        false,
    );
    match agent_result {
        Ok(handle) => {
            tracing::debug!(
                "[run_with_provider] Creating custom-provider agent - session={}, provider={}",
                session.id,
                current_provider
            );
            let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
            let agent = handle.into_inner();
            if !mcp_wrappers.is_empty() {
                for wrapper in mcp_wrappers {
                    if let Err(error) = agent.tool_server_handle.add_tool(wrapper).await {
                        tracing::warn!("[MCP] Failed to add MCP tool: {}", error);
                    }
                }
            }
            codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone());
            let agent =
                inner_session.configure_agent(codelet_core::RigAgent::with_default_depth(agent));
            codelet_cli::interactive::run_agent_stream_with_images(
                agent,
                input,
                bridge_images,
                inner_session,
                session.is_interrupted.clone(),
                session.compaction_in_progress.clone(),
                session.interrupt_notify.clone(),
                output,
                session.id,
            )
            .await
        }
        Err(error) => {
            tracing::error!("Unsupported provider '{}' (not a built-in and custom-provider dispatch failed: {})", current_provider, error);
            Err(anyhow::anyhow!(
                "Unsupported provider: {}",
                current_provider
            ))
        }
    }
}

#[cfg(feature = "test-support")]
pub(super) async fn run_stub_turn(
    inner_session: &mut Session,
    input: &str,
    bridge_images: Option<Vec<BridgeImage>>,
    session: &BackgroundSession,
    output: &BackgroundOutput,
    thinking_config_value: Option<serde_json::Value>,
    current_provider: &str,
) -> anyhow::Result<()> {
    if codelet_providers::stub_provider::is_stub_registered(current_provider) {
        tracing::debug!(
            "[run_with_provider] Creating stub agent - session={}, provider={}",
            session.id,
            current_provider
        );
        let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
        let role_preamble = session.get_role();
        let stub = codelet_providers::stub_provider::StubProvider::new();
        let agent =
            stub.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value);
        if !mcp_wrappers.is_empty() {
            for wrapper in mcp_wrappers {
                if let Err(error) = agent.tool_server_handle.add_tool(wrapper).await {
                    tracing::warn!("[MCP] Failed to add MCP tool: {}", error);
                }
            }
        }
        codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone());
        let agent =
            inner_session.configure_agent(codelet_core::RigAgent::with_default_depth(agent));
        codelet_cli::interactive::run_agent_stream_with_images(
            agent,
            input,
            bridge_images,
            inner_session,
            session.is_interrupted.clone(),
            session.compaction_in_progress.clone(),
            session.interrupt_notify.clone(),
            output,
            session.id,
        )
        .await
    } else {
        tracing::error!(
            "Stub provider '{}' not in in-memory registry — was register_stub_provider() called?",
            current_provider
        );
        Err(anyhow::anyhow!(
            "Stub provider '{}' not registered",
            current_provider
        ))
    }
}
