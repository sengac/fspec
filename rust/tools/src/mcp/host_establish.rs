// REFAC-035: host_config's per-server connect helpers, include!d by host_config.rs
// so they stay private to codelet_tools::mcp::host_config.

/// MCP-010: establish one server with its handshake and tool discovery
/// together bounded by MCP_HANDSHAKE_TIMEOUT_SECS (establish_* bound only the
/// handshake). A timed-out attempt is dropped, which ends its service.
async fn establish_bounded(
    server: &HostMcpServer,
    injection_tx: &McpInjectionTx,
    connections: &McpConnectionMap,
) -> Result<McpEstablished, String> {
    let limit = std::time::Duration::from_secs(MCP_HANDSHAKE_TIMEOUT_SECS);
    tokio::time::timeout(limit, establish(server, injection_tx, connections))
        .await
        .map_err(|_| {
            format!(
                "did not complete its handshake and tool discovery within {MCP_HANDSHAKE_TIMEOUT_SECS}s"
            )
        })?
}

/// Handshake one server under fspec's MCP timeout, without caching it; the
/// error is the redacted reason.
async fn establish(
    server: &HostMcpServer,
    injection_tx: &McpInjectionTx,
    connections: &McpConnectionMap,
) -> Result<McpEstablished, String> {
    let result = match &server.transport {
        HostMcpTransport::Stdio { command, args, env } => {
            let env: HashMap<String, String> = env.clone().into_iter().collect();
            let command = McpStdioCommand {
                program: command.clone(),
                args: args.clone(),
            };
            establish_stdio(
                &server.name,
                &command,
                Some(&env),
                MCP_HANDSHAKE_TIMEOUT_SECS,
                injection_tx.clone(),
                connections.clone(),
            )
            .await
        }
        HostMcpTransport::Http { url, headers } => {
            let headers: HashMap<String, String> = headers.clone().into_iter().collect();
            establish_http(
                &server.name,
                url,
                Some(&headers),
                MCP_HANDSHAKE_TIMEOUT_SECS,
                injection_tx.clone(),
                connections.clone(),
            )
            .await
        }
        HostMcpTransport::Sse { url, headers } => {
            let headers: HashMap<String, String> = headers.clone().into_iter().collect();
            establish_sse(
                &server.name,
                url,
                Some(&headers),
                MCP_HANDSHAKE_TIMEOUT_SECS,
                injection_tx.clone(),
                connections.clone(),
            )
            .await
        }
    };
    result.map_err(|failed| redact(&failed.message, &server.transport))
}

/// Remove the executable, its arguments, the url, every environment value and every header value
/// from a connect message before it leaves the daemon.
fn redact(message: &str, transport: &HostMcpTransport) -> String {
    let mut secrets: Vec<&str> = match transport {
        HostMcpTransport::Stdio { command, args, env } => std::iter::once(command.as_str())
            .chain(args.iter().map(String::as_str))
            .chain(env.values().map(String::as_str))
            .collect(),
        HostMcpTransport::Http { url, headers } | HostMcpTransport::Sse { url, headers } => {
            std::iter::once(url.as_str())
                .chain(headers.values().map(String::as_str))
                .collect()
        }
    };
    secrets.retain(|secret| !secret.is_empty());
    secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
    secrets.iter().fold(message.to_string(), |text, secret| {
        text.replace(secret, "<redacted>")
    })
}
