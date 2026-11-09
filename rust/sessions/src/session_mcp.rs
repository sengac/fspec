//! MCP-003: configure a live session's MCP servers for a daemon client.
//!
//! Feature: spec/features/session-mcp-server-configuration.feature

use codelet_rpc_types::{
    SessionMcpServer, SessionMcpServerTools, SessionMcpTransport, SessionStatus,
};
use codelet_tools::mcp::host_config::{
    configure_host_mcp_servers, HostMcpServer, HostMcpServerTools, HostMcpTransport,
};
use std::sync::Arc;

use crate::background_session::BackgroundSession;

fn to_host(server: SessionMcpServer) -> HostMcpServer {
    HostMcpServer {
        name: server.name,
        transport: match server.transport {
            SessionMcpTransport::Stdio { command, args, env } => {
                HostMcpTransport::Stdio { command, args, env }
            }
            SessionMcpTransport::Http { url, headers } => HostMcpTransport::Http { url, headers },
            SessionMcpTransport::Sse { url, headers } => HostMcpTransport::Sse { url, headers },
        },
    }
}

fn to_wire(server: HostMcpServerTools) -> SessionMcpServerTools {
    SessionMcpServerTools {
        name: server.name,
        tools: server.tools,
    }
}

/// MCP-008: make `servers` the idle live session's whole MCP set, or leave
/// the set unchanged.
pub(crate) async fn configure(
    session: Arc<BackgroundSession>,
    servers: Vec<SessionMcpServer>,
) -> Result<Vec<SessionMcpServerTools>, String> {
    let status = session.get_status();
    if status != SessionStatus::Idle {
        return Err(format!(
            "session {} is {status:?}; MCP servers can only be configured while it is idle",
            session.id
        ));
    }
    configure_host_mcp_servers(session.id, servers.into_iter().map(to_host).collect())
        .await
        .map(|configured| configured.into_iter().map(to_wire).collect())
        .map_err(|error| format!("session {}: {error}", session.id))
}
