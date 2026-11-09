/// Get injection_tx + connections for a session (used internally by ConnectMcpTool).
fn get_mcp_session_state(session_id: uuid::Uuid) -> Option<(McpInjectionTx, McpConnectionMap)> {
    let sessions = MCP_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    sessions
        .get(&session_id)
        .map(|s| (s.injection_tx.clone(), s.connections.clone()))
}

/// MCP-002: Store the running agent's ToolServerHandle in per-session MCP state.
///
/// Called by run_with_provider! after building the agent, so that ConnectMcpTool
/// can register newly connected tools mid-turn via `handle.add_tool()`.
pub fn set_mcp_tool_server_handle(session_id: uuid::Uuid, handle: ToolServerHandle) {
    let mut sessions = MCP_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(state) = sessions.get_mut(&session_id) {
        state.tool_server_handle = Some(handle);
    }
}

/// MCP-002: Retrieve the ToolServerHandle for a session (for mid-turn tool registration).
fn get_mcp_tool_server_handle(session_id: uuid::Uuid) -> Option<ToolServerHandle> {
    let sessions = MCP_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    sessions
        .get(&session_id)
        .and_then(|s| s.tool_server_handle.clone())
}

// =============================================================================
// Gathering MCP tools for agent building
// =============================================================================

/// Data needed to register one MCP tool with rig's agent builder.
pub struct McpToolRegistration {
    /// rmcp tool definition with namespaced name (mcp__<server>__<tool>)
    pub tool: rmcp::model::Tool,
    /// The server's peer sink for making tool calls
    pub client: rmcp::service::ServerSink,
}

/// Gather all MCP tools from active connections for a session, namespaced and
/// ready for registration with rig's `rmcp_tool()` agent builder method.
///
/// Called fresh each turn by `create_rig_agent()` so newly connected tools
/// appear immediately in the next turn.
pub async fn gather_mcp_tool_registrations(session_id: uuid::Uuid) -> Vec<McpToolRegistration> {
    let connections = match get_mcp_connections(session_id) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let map = connections.read().await;
    let mut registrations = Vec::new();

    for conn in map.values() {
        let client = conn.service.peer().clone();
        for raw_tool in &conn.raw_tools {
            // Clone and namespace the tool name
            let mut namespaced = raw_tool.clone();
            namespaced.name =
                std::borrow::Cow::Owned(qualified_mcp_tool_name(&conn.name, &raw_tool.name));
            registrations.push(McpToolRegistration {
                tool: namespaced,
                client: client.clone(),
            });
        }
    }

    registrations
}

// =============================================================================
// McpToolWrapper — rig::tool::Tool for individual MCP server tools
// =============================================================================

/// Wrapper that makes an MCP server tool callable through rig's Tool trait.
///
/// Each MCP tool from a connected server gets one McpToolWrapper instance.
/// When called, the wrapper routes through the cached connection via `route_mcp_tool_call`.
///
/// Follows the same pattern as `FacadeToolWrapper`: uses a dummy `const NAME` and
/// overrides `fn name()` to return the dynamic qualified name at runtime.
#[derive(Clone, Debug)]
pub struct McpToolWrapper {
    /// Qualified name: mcp__<server>__<tool>
    qualified_name: String,
    /// Tool description from the MCP server
    description: String,
    /// JSON Schema for tool input parameters (from MCP server's tools/list)
    input_schema: serde_json::Value,
    /// Session ID for looking up the connection map at call time
    session_id: uuid::Uuid,
}

impl McpToolWrapper {
    /// Build a wrapper from an MCP tool definition.
    ///
    /// Shared constructor used by both `gather_mcp_tool_wrappers` (turn start)
    /// and `register_new_tools_with_handle` (mid-turn connect).
    fn from_tool_def(server_name: &str, tool: &McpToolDef, session_id: uuid::Uuid) -> Self {
        Self {
            qualified_name: qualified_mcp_tool_name(server_name, &tool.name),
            description: tool.description.clone().unwrap_or_default(),
            input_schema: tool.input_schema.clone(),
            session_id,
        }
    }
}

impl rig::tool::Tool for McpToolWrapper {
    // Dummy const — we override name() to return the dynamic qualified name
    const NAME: &'static str = "mcp_tool_wrapper";

    type Error = crate::ToolError;
    type Args = serde_json::Value;
    type Output = String;

    fn name(&self) -> String {
        self.qualified_name.clone()
    }

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: self.qualified_name.clone(),
            description: self.description.clone(),
            parameters: self.input_schema.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-013: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            &self.qualified_name,
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(crate::ToolError::Blocked {
                tool: "mcp",
                message: reason,
            });
        }

        let connections =
            get_mcp_connections(self.session_id).ok_or_else(|| crate::ToolError::Execution {
                tool: "mcp",
                message: "MCP session not initialized".to_string(),
            })?;

        route_mcp_tool_call(&self.qualified_name, args, &connections)
            .await
            .map_err(|e| crate::ToolError::Execution {
                tool: "mcp",
                message: e,
            })
    }
}

/// Gather all MCP tools from active connections as rig-compatible wrappers.
///
/// Called each turn by the agent builder so newly connected MCP tools appear
/// immediately. Uses `try_read()` to avoid blocking — if the connection map
/// lock is briefly held (during connect/disconnect), MCP tools simply appear
/// on the next turn.
pub fn gather_mcp_tool_wrappers(session_id: uuid::Uuid) -> Vec<McpToolWrapper> {
    let connections = match get_mcp_connections(session_id) {
        Some(c) => c,
        None => return Vec::new(),
    };

    // Non-blocking read: if lock is held, return empty and tools appear next turn
    let map = match connections.try_read() {
        Ok(m) => m,
        Err(_) => {
            debug!("[MCP] Connection map locked — MCP tools will appear next turn");
            return Vec::new();
        }
    };

    let mut wrappers = Vec::new();
    for conn in map.values() {
        for tool in &conn.tools {
            wrappers.push(McpToolWrapper::from_tool_def(&conn.name, tool, session_id));
        }
    }
    wrappers
}

// =============================================================================
// ConnectMcpTool — rig::tool::Tool implementation
// =============================================================================

/// The ConnectMCP tool that the LLM calls to manage MCP server connections.
///
/// Supports three actions: connect (stdio/http), disconnect, and list.
/// Uses global per-session state to manage connections across turns.
#[derive(Clone, Debug)]
pub struct ConnectMcpTool {
    session_id: uuid::Uuid,
}

impl ConnectMcpTool {
    /// Create a new ConnectMcpTool bound to a session.
    pub fn new(session_id: uuid::Uuid) -> Self {
        Self { session_id }
    }

    /// MCP-002: Register newly connected server's tools with the running agent's ToolServerHandle.
    ///
    /// Called after a successful connect. Reads the connection map for the named server,
    /// builds McpToolWrapper instances, and adds them to the handle so the LLM can call
    /// them in the same turn.
    ///
    /// Graceful degradation: if no handle is set yet (e.g. agent not fully initialized),
    /// tools simply appear on the next turn via gather_mcp_tool_wrappers().
    async fn register_new_tools_with_handle(
        &self,
        server_name: &str,
    ) -> Result<(), rig::tool::server::ToolServerError> {
        let handle = match get_mcp_tool_server_handle(self.session_id) {
            Some(h) => h,
            None => {
                debug!(
                    "[MCP-002] No ToolServerHandle for session {} — tools will appear next turn",
                    self.session_id,
                );
                return Ok(());
            }
        };

        let connections = match get_mcp_connections(self.session_id) {
            Some(c) => c,
            None => return Ok(()),
        };

        // Read connection map to get the newly connected server's tools
        let wrappers = {
            let map = connections.read().await;
            let conn = match map.get(server_name) {
                Some(c) => c,
                None => return Ok(()),
            };
            conn.tools
                .iter()
                .map(|tool| McpToolWrapper::from_tool_def(&conn.name, tool, self.session_id))
                .collect::<Vec<_>>()
        };

        add_wrappers_to_handle(&handle, wrappers, server_name, self.session_id).await
    }

    /// MCP-002: Remove a disconnected server's tools from the running agent's ToolServerHandle.
    ///
    /// Called after a successful disconnect. The connection has already been removed
    /// from the map by disconnect_mcp(), so we enumerate the handle's current tool
    /// definitions and remove those matching the server name prefix.
    async fn remove_tools_from_handle(&self, server_name: &str) {
        let handle = match get_mcp_tool_server_handle(self.session_id) {
            Some(h) => h,
            None => return,
        };

        remove_server_tools_from_handle(&handle, server_name, self.session_id).await;
    }
}
