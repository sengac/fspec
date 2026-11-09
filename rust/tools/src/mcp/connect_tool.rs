/// MCP-011: ConnectMCP's parameter schema, serialized once.
fn connect_schema() -> Result<&'static serde_json::Value, String> {
    static SCHEMA: std::sync::OnceLock<Result<serde_json::Value, String>> =
        std::sync::OnceLock::new();
    SCHEMA
        .get_or_init(|| {
            serde_json::to_value(schemars::schema_for!(McpConnectArgs))
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)
}
impl rig::tool::Tool for ConnectMcpTool {
    const NAME: &'static str = "ConnectMCP";

    type Error = crate::ToolError;
    type Args = McpConnectArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "ConnectMCP".to_string(),
            description: concat!(
                "Connect to external MCP (Model Context Protocol) servers to gain additional tools at runtime. ",
                "Use action 'connect' with a required transport: 'stdio' + command (the executable only) + args (a list of literal arguments, [] for none), or transport 'http' or 'sse' + url. ",
                "Use action 'disconnect' with name to tear down a connection. ",
                "Use action 'list' to show all active MCP connections. ",
                "Connected server tools become available as mcp__<server>__<tool> in subsequent turns. ",
                "Tools from a newly connected server are also available immediately in the same turn."
            )
            .to_string(),
            // MCP-011: no error channel here — a schema that cannot be
            // serialized advertises a closed empty schema naming the error,
            // and call() refuses every request with it.
            parameters: match connect_schema() {
                Ok(schema) => schema.clone(),
                Err(error) => {
                    tracing::error!(%error, "ConnectMCP schema cannot be serialized");
                    json!({"type": "object", "additionalProperties": false,
                        "description": format!("ConnectMCP is unavailable: {error}")})
                }
            },
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // MCP-011: the advertised schema must exist for a call to be served.
        connect_schema().map_err(|error| crate::ToolError::Execution {
            tool: "ConnectMCP",
            message: format!("ConnectMCP is unavailable: {error}"),
        })?;
        // MCP-011: the hook sees the real arguments or the call is refused.
        let payload = serde_json::to_value(&args).map_err(|error| crate::ToolError::Validation {
            tool: "ConnectMCP",
            message: format!("ConnectMCP arguments cannot be serialized: {error}"),
        })?;
        // HOOK-013: Run pre_tool_use hooks before execution
        if let Err(reason) =
            crate::pre_tool_hook::pre_tool_hook_check(self.session_id, "ConnectMCP", &payload)
        {
            return Err(crate::ToolError::Blocked {
                tool: "ConnectMCP",
                message: reason,
            });
        }

        let (injection_tx, connections) =
            get_mcp_session_state(self.session_id).ok_or_else(|| crate::ToolError::Execution {
                tool: "ConnectMCP",
                message: "MCP session not initialized".to_string(),
            })?;

        let result = match args.action {
            McpAction::Connect => {
                let name = args.name.as_deref().ok_or(crate::ToolError::Validation {
                    tool: "ConnectMCP",
                    message: "name is required for connect action".to_string(),
                })?;
                // MCP-009: the transport is required; there is no stdio default.
                let transport = args.transport.ok_or(crate::ToolError::Validation {
                    tool: "ConnectMCP",
                    message: "transport is required for connect action".to_string(),
                })?;
                // MCP-008: fspec's handshake timeout, never the model's.
                let timeout = MCP_HANDSHAKE_TIMEOUT_SECS;

                match transport {
                    McpTransport::Stdio => {
                        let command = stdio_command::McpStdioCommand::from_connect_args(
                            args.command.as_deref(),
                            args.args.as_deref(),
                        )?;
                        connect_stdio(
                            name,
                            &command,
                            args.env.as_ref(),
                            timeout,
                            injection_tx,
                            connections,
                        )
                        .await
                    }
                    McpTransport::Http => {
                        let url = args.url.as_deref().ok_or(crate::ToolError::Validation {
                            tool: "ConnectMCP",
                            message: "url is required for http transport".to_string(),
                        })?;
                        connect_http(
                            name,
                            url,
                            args.headers.as_ref(),
                            timeout,
                            injection_tx,
                            connections,
                        )
                        .await
                    }
                    McpTransport::Sse => {
                        let url = args.url.as_deref().ok_or(crate::ToolError::Validation {
                            tool: "ConnectMCP",
                            message: "url is required for sse transport".to_string(),
                        })?;
                        connect_sse(
                            name,
                            url,
                            args.headers.as_ref(),
                            timeout,
                            injection_tx,
                            connections,
                        )
                        .await
                    }
                }
            }
            McpAction::Disconnect => {
                let name = args.name.as_deref().ok_or(crate::ToolError::Validation {
                    tool: "ConnectMCP",
                    message: "name is required for disconnect action".to_string(),
                })?;
                disconnect_mcp(name, &connections).await
            }
            McpAction::List => {
                let map = connections.read().await;
                let summaries = list_connections(&map);
                if summaries.is_empty() {
                    McpConnectResult {
                        success: true,
                        message: "No active MCP connections.".to_string(),
                        tools: None,
                        connections: Some(summaries),
                    }
                } else {
                    let lines: Vec<String> = summaries
                        .iter()
                        .map(|s| {
                            format!(
                                "  {} — connected {}, {} tools, {} calls [{}]",
                                s.name,
                                s.uptime,
                                s.tool_count,
                                s.call_count,
                                match s.transport {
                                    McpTransport::Stdio => "stdio",
                                    McpTransport::Http => "http",
                                    McpTransport::Sse => "sse",
                                },
                            )
                        })
                        .collect();
                    McpConnectResult {
                        success: true,
                        message: format!(
                            "Connected MCP servers ({}):\n{}",
                            summaries.len(),
                            lines.join("\n"),
                        ),
                        tools: None,
                        connections: Some(summaries),
                    }
                }
            }
        };

        // Return the message as the tool output string
        // MCP-002: After successful connect/disconnect, register or remove tools with
        // the running agent's ToolServerHandle so they are callable in the same turn.
        //
        // IMPORTANT: We must `tokio::spawn` the registration work instead of awaiting
        // it inline. The ToolServer processes messages sequentially via `handle_message`.
        // We are currently INSIDE a `CallTool` message handler. If we call
        // `handle.add_tool().await` here, it sends an `AddTool` message to the same
        // channel and blocks waiting for a response — but the message loop can't
        // dequeue it until THIS call returns. That's a re-entrancy deadlock.
        //
        // By spawning, ConnectMCP returns immediately, the `CallTool` handler finishes,
        // and the spawned task's `AddTool` messages are processed on the next loop
        // iterations.
        if result.success {
            let session_id = self.session_id;
            match &args.action {
                McpAction::Connect => {
                    if let Some(name) = args.name.clone() {
                        tokio::spawn(async move {
                            let tool = ConnectMcpTool::new(session_id);
                            // Remove old tools first — handles reconnect to same server name.
                            // Without this, reconnecting adds duplicate tools to the handle
                            // (old tools linger + new tools added = "Tool names must be unique" error).
                            tool.remove_tools_from_handle(&name).await;
                            if let Err(error) = tool.register_new_tools_with_handle(&name).await {
                                tool.remove_tools_from_handle(&name).await;
                                if let Some((injection_tx, connections)) =
                                    get_mcp_session_state(session_id)
                                {
                                    disconnect_mcp(&name, &connections).await;
                                    let _ = injection_tx
                                        .send(McpInjection::Notification(format!(
                                            "MCP tool registration failed for '{name}': {error}"
                                        )))
                                        .await;
                                }
                            }
                        });
                    }
                }
                McpAction::Disconnect => {
                    if let Some(name) = args.name.clone() {
                        tokio::spawn(async move {
                            let tool = ConnectMcpTool::new(session_id);
                            tool.remove_tools_from_handle(&name).await;
                        });
                    }
                }
                McpAction::List => {}
            }
            Ok(result.message)
        } else {
            // Return error as tool output (not a Rust error) so LLM can reason about it
            Ok(result.message)
        }
    }
}
