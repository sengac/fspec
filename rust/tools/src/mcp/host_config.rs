//! MCP-003: a daemon client's configuration of a session's MCP servers.
//!
//! Feature: spec/features/session-mcp-server-configuration.feature
//! Feature: spec/features/declarative-session-mcp-server-sets.feature
//!
//! Shares the ConnectMCP path: the same per-session registry
//! (`get_mcp_session_state`) and the same `establish_*` connectors, under
//! fspec's own `MCP_HANDSHAKE_TIMEOUT_SECS`. MCP-008: the request is the
//! session's whole MCP set — every new or changed server is established
//! first, and only when all of them succeeded is the swap committed (unlisted
//! servers disconnected, changed ones replaced, identical ones kept), so a
//! failure leaves the previous set intact. Commands, urls, environment values
//! and header values never appear in errors or logs.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::stdio_command::McpStdioCommand;
use super::{
    establish_http, establish_sse, establish_stdio, get_mcp_session_state, qualified_mcp_tool_name,
    McpConnection, McpConnectionMap, McpEstablished, McpInjectionTx, MCP_HANDSHAKE_TIMEOUT_SECS,
};

/// One server of the session's declared MCP set.
pub struct HostMcpServer {
    pub name: String,
    pub transport: HostMcpTransport,
}

/// How to reach a [`HostMcpServer`]. Two servers are identical when their
/// names and transports are equal (MCP-008).
#[derive(Clone, PartialEq, Eq)]
pub enum HostMcpTransport {
    /// MCP-005: `command` is the executable; `args` are passed literally.
    Stdio {
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    },
    Http {
        url: String,
        /// MCP-004: sent on every request of the connection.
        headers: BTreeMap<String, String>,
    },
    /// MCP-006: a legacy HTTP+SSE endpoint.
    Sse {
        url: String,
        headers: BTreeMap<String, String>,
    },
}

/// A server of the resulting set and its qualified tool names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostMcpServerTools {
    pub name: String,
    pub tools: Vec<String>,
}

/// Why a configuration request was refused or failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HostMcpConfigError {
    #[error("session {0} has no MCP registry (it is not live)")]
    SessionNotLive(uuid::Uuid),
    #[error("MCP server name '{0}' is invalid: use letters, digits, '-' or '_' and no '__'")]
    InvalidName(String),
    #[error("MCP server '{0}' is named more than once in the request")]
    DuplicateName(String),
    #[error("MCP server '{0}' has an empty stdio command")]
    EmptyCommand(String),
    #[error("MCP server '{0}' needs an http:// or https:// url")]
    InvalidUrl(String),
    #[error("MCP server '{name}' {source}")]
    InvalidHeader {
        name: String,
        source: super::http_headers::McpHttpHeaderError,
    },
    /// MCP-010: every server that failed, in request order.
    #[error("{}", describe_failures(.0))]
    ConnectFailed(Vec<HostMcpConnectFailure>),
    #[error("MCP server '{0}' changed while the session's MCP set was being configured")]
    ChangedDuringConfiguration(String),
}

/// One server of a request that failed to connect, with its redacted reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostMcpConnectFailure {
    pub name: String,
    pub reason: String,
}

fn describe_failures(failures: &[HostMcpConnectFailure]) -> String {
    failures
        .iter()
        .map(|failure| {
            format!(
                "MCP server '{}' failed to connect: {}",
                failure.name, failure.reason
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("__")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// MCP-008: an empty list is valid (it declares no servers).
fn validate(servers: &[HostMcpServer]) -> Result<(), HostMcpConfigError> {
    let mut seen = HashSet::new();
    for server in servers {
        let name = server.name.clone();
        if !valid_name(&name) {
            return Err(HostMcpConfigError::InvalidName(name));
        }
        if !seen.insert(name.clone()) {
            return Err(HostMcpConfigError::DuplicateName(name));
        }
        match &server.transport {
            HostMcpTransport::Stdio { command, .. } if command.trim().is_empty() => {
                return Err(HostMcpConfigError::EmptyCommand(name));
            }
            HostMcpTransport::Http { url, .. } | HostMcpTransport::Sse { url, .. }
                if !(url.starts_with("http://") || url.starts_with("https://")) =>
            {
                return Err(HostMcpConfigError::InvalidUrl(name));
            }
            HostMcpTransport::Http { headers, .. } | HostMcpTransport::Sse { headers, .. } => {
                super::http_headers::parse_mcp_http_headers(headers)
                    .map_err(|source| HostMcpConfigError::InvalidHeader { name, source })?;
            }
            HostMcpTransport::Stdio { .. } => {}
        }
    }
    Ok(())
}

/// Whether `connection` was configured with exactly `server`'s transport.
fn identical(connection: &McpConnection, server: &HostMcpServer) -> bool {
    connection.configured_transport.as_ref() == Some(&server.transport)
}

fn qualified_tools(connection: &McpConnection) -> Vec<String> {
    connection
        .tools
        .iter()
        .map(|tool| qualified_mcp_tool_name(&connection.name, &tool.name))
        .collect()
}

fn cancel(connections: impl IntoIterator<Item = McpConnection>) {
    for connection in connections {
        connection.service.cancellation_token().cancel();
    }
}

/// Make the session's MCP set exactly `servers`, or leave it as it was.
pub async fn configure_host_mcp_servers(
    session_id: uuid::Uuid,
    servers: Vec<HostMcpServer>,
) -> Result<Vec<HostMcpServerTools>, HostMcpConfigError> {
    let (injection_tx, connections) =
        get_mcp_session_state(session_id).ok_or(HostMcpConfigError::SessionNotLive(session_id))?;
    validate(&servers)?;
    let kept: HashSet<String> = {
        let map = connections.read().await;
        servers
            .iter()
            .filter(|server| map.get(&server.name).is_some_and(|c| identical(c, server)))
            .map(|server| server.name.clone())
            .collect()
    };
    // MCP-010: every new and changed server connects at once, each bounded
    // by MCP_HANDSHAKE_TIMEOUT_SECS, so the whole step is bounded by one.
    let pending: Vec<&HostMcpServer> = servers
        .iter()
        .filter(|server| !kept.contains(&server.name))
        .collect();
    let outcomes = futures::future::join_all(
        pending
            .iter()
            .map(|server| establish_bounded(server, &injection_tx, &connections)),
    )
    .await;
    let mut established: HashMap<String, McpConnection> = HashMap::new();
    let mut failures = Vec::new();
    for (server, outcome) in pending.into_iter().zip(outcomes) {
        match outcome {
            Ok(mut done) => {
                done.connection.configured_transport = Some(server.transport.clone());
                established.insert(server.name.clone(), done.connection);
            }
            Err(reason) => failures.push(HostMcpConnectFailure {
                name: server.name.clone(),
                reason,
            }),
        }
    }
    if !failures.is_empty() {
        cancel(established.into_values());
        let failed: Vec<&str> = failures
            .iter()
            .map(|failure| failure.name.as_str())
            .collect();
        tracing::warn!(servers = ?failed, "MCP-010: MCP set unchanged; servers failed to connect");
        return Err(HostMcpConfigError::ConnectFailed(failures));
    }
    commit(&connections, &servers, established).await
}

/// Swap the established connections in and every unlisted one out, under one
/// write lock; a kept server that changed meanwhile aborts the swap.
async fn commit(
    connections: &McpConnectionMap,
    servers: &[HostMcpServer],
    mut established: HashMap<String, McpConnection>,
) -> Result<Vec<HostMcpServerTools>, HostMcpConfigError> {
    let mut map = connections.write().await;
    let mut configured = Vec::with_capacity(servers.len());
    let mut incoming = Vec::with_capacity(established.len());
    for server in servers {
        let tools = match established.remove(&server.name) {
            Some(connection) => {
                let tools = qualified_tools(&connection);
                incoming.push(connection);
                tools
            }
            None => match map.get(&server.name).filter(|c| identical(c, server)) {
                Some(connection) => qualified_tools(connection),
                None => {
                    drop(map);
                    cancel(incoming);
                    return Err(HostMcpConfigError::ChangedDuringConfiguration(
                        server.name.clone(),
                    ));
                }
            },
        };
        configured.push(HostMcpServerTools {
            name: server.name.clone(),
            tools,
        });
    }
    let listed: HashSet<&str> = servers.iter().map(|server| server.name.as_str()).collect();
    let unlisted: Vec<String> = map
        .keys()
        .filter(|name| !listed.contains(name.as_str()))
        .cloned()
        .collect();
    let mut replaced: Vec<McpConnection> = unlisted
        .iter()
        .filter_map(|name| map.remove(name))
        .collect();
    for connection in incoming {
        tracing::info!(server = %connection.name, "MCP-008: host MCP server connected");
        replaced.extend(map.insert(connection.name.clone(), connection));
    }
    drop(map);
    for name in &unlisted {
        tracing::info!(server = %name, "MCP-008: unlisted host MCP server disconnected");
    }
    cancel(replaced);
    Ok(configured)
}

include!("host_establish.rs");
