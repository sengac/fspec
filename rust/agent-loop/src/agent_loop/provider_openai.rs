use crate::background_output::BackgroundOutput;
use codelet_cli::{interactive::BridgeImage, session::Session};
use codelet_sessions::background_session::BackgroundSession;

pub(super) async fn run_openai_turn(
    inner_session: &mut Session,
    input: &str,
    bridge_images: Option<Vec<BridgeImage>>,
    session: &BackgroundSession,
    output: &BackgroundOutput,
    thinking_config_value: Option<serde_json::Value>,
) -> anyhow::Result<()> {
    match inner_session.provider_manager_mut().get_openai(session.id) {
        Ok(provider) => {
            tracing::debug!(
                "[run_with_provider] Creating agent - session={}, getter=get_openai",
                session.id
            );
            let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
            let role_preamble = session.get_role();
            let agent = provider.create_rig_agent(
                session.id,
                role_preamble.as_deref(),
                thinking_config_value,
            );
            if !mcp_wrappers.is_empty() {
                tracing::info!(
                    "[MCP] Adding {} MCP tool wrappers to agent for session {}",
                    mcp_wrappers.len(),
                    session.id
                );
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
            tracing::warn!("[run_with_provider] Failed to get provider: {}", error);
            Err(anyhow::anyhow!("Failed to get provider: {}", error))
        }
    }
}
