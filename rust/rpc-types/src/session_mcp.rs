//! MCP-003: daemon-client configuration of a session's MCP servers.
//!
//! Feature: spec/features/session-mcp-server-configuration.feature

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One MCP server of a session's declared MCP set (MCP-008: the handshake
/// timeout is fspec's own, so a request naming one is refused).
///
/// Wire (bincode) field order: `name`, `transport`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionMcpServer {
    /// Server name; its tools are exposed as `mcp__<name>__<tool>`.
    pub name: String,
    /// How to reach the server.
    pub transport: SessionMcpTransport,
}

/// The transport of a [`SessionMcpServer`].
///
/// Wire (bincode) variant order: `Stdio { command, args, env }` = 0, `Http { url, headers }` = 1,
/// `Sse { url, headers }` = 2.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SessionMcpTransport {
    /// Spawn a local process that speaks MCP over stdio.
    Stdio {
        /// MCP-005: the executable itself (a PATH name or a path; never split).
        command: String,
        /// MCP-005: the ordered arguments, each passed literally (no shell).
        args: Vec<String>,
        /// Extra environment for the process (values are secrets: never logged).
        env: BTreeMap<String, String>,
    },
    /// Connect to a Streamable HTTP MCP endpoint.
    Http {
        /// The endpoint url (`http://` or `https://`).
        url: String,
        /// MCP-004: headers sent on every request of the session, for example
        /// `Authorization` (values are secrets: never logged).
        headers: BTreeMap<String, String>,
    },
    /// MCP-006: connect to a legacy (2024-11-05) HTTP+SSE MCP endpoint; never
    /// treated as Streamable HTTP.
    Sse {
        /// The event-stream url (`http://` or `https://`).
        url: String,
        /// Headers sent on the event-stream request and every message POST
        /// (values are secrets: never logged).
        headers: BTreeMap<String, String>,
    },
}

/// A server connected by [`SessionMcpServer`] configuration, with the
/// qualified names of the tools the session's agent can now call.
///
/// Wire (bincode) field order: `name`, `tools`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SessionMcpServerTools {
    /// The server name.
    pub name: String,
    /// Qualified tool names (`mcp__<server>__<tool>`).
    pub tools: Vec<String>,
}

/// Debug never prints the command, its arguments, the url, any environment value or any header value.
impl std::fmt::Debug for SessionMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionMcpServer")
            .field("name", &self.name)
            .field("transport", &self.transport)
            .finish()
    }
}

impl std::fmt::Debug for SessionMcpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio { args, env, .. } => f
                .debug_struct("Stdio")
                .field("command", &"<redacted>")
                .field("arg_count", &args.len())
                .field("env_keys", &env.keys().collect::<Vec<_>>())
                .finish(),
            Self::Http { headers, .. } => f
                .debug_struct("Http")
                .field("url", &"<redacted>")
                .field("header_names", &headers.keys().collect::<Vec<_>>())
                .finish(),
            Self::Sse { headers, .. } => f
                .debug_struct("Sse")
                .field("url", &"<redacted>")
                .field("header_names", &headers.keys().collect::<Vec<_>>())
                .finish(),
        }
    }
}
