// rmcp ClientHandler trait requires `impl Future` return types (not `async fn`).
// Suppressing manual_async_fn for the trait impl.
#[allow(clippy::manual_async_fn)]
impl rmcp::handler::client::ClientHandler for DynMcpHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("rust", env!("CARGO_PKG_VERSION")),
        )
    }

    fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<CreateMessageResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            let (response_tx, response_rx) = tokio::sync::oneshot::channel();
            let injection = McpInjection::SamplingRequest {
                params,
                response_tx,
            };

            if let Err(e) = self.injection_tx.send(injection).await {
                error!("[MCP:{}] Failed to inject sampling request: {e}", self.name);
                return Err(rmcp::ErrorData::internal_error(
                    "Failed to inject sampling request into session",
                    None,
                ));
            }

            match response_rx.await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(e)) => {
                    error!("[MCP:{}] Sampling request failed: {e}", self.name);
                    Err(rmcp::ErrorData::internal_error(
                        "Sampling request failed",
                        None,
                    ))
                }
                Err(_) => {
                    error!("[MCP:{}] Sampling response channel dropped", self.name);
                    Err(rmcp::ErrorData::internal_error(
                        "Response channel dropped",
                        None,
                    ))
                }
            }
        }
    }

    fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            debug!(
                "[MCP:{}] Tool list changed notification received",
                self.name
            );

            // Re-fetch tools from the server
            match context.peer.list_all_tools().await {
                Ok(tools) => {
                    let tool_defs: Vec<McpToolDef> = tools
                        .iter()
                        .map(|t| McpToolDef {
                            name: t.name.to_string(),
                            description: t
                                .description
                                .as_ref()
                                .map(std::string::ToString::to_string),
                            input_schema: t.schema_as_json_value(),
                        })
                        .collect();

                    let tool_count = tool_defs.len();

                    // Update cached tools (both simplified and raw)
                    {
                        let mut map = self.connections.write().await;
                        if let Some(conn) = map.get_mut(&self.name) {
                            conn.tools = tool_defs;
                            conn.raw_tools = tools;
                        }
                    }

                    // Inject notification
                    let msg = format!(
                        "[MCP:{}] Server tools list changed — refreshed {tool_count} tools",
                        self.name
                    );
                    let _ = self
                        .injection_tx
                        .send(McpInjection::Notification(msg))
                        .await;
                }
                Err(e) => {
                    warn!(
                        "[MCP:{}] Failed to re-fetch tools after list_changed: {e}",
                        self.name
                    );
                }
            }
        }
    }

    fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            let msg = format!("[MCP:{}] Resource updated: {}", self.name, params.uri);
            let _ = self
                .injection_tx
                .send(McpInjection::Notification(msg))
                .await;
        }
    }

    fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            let msg = format!(
                "[MCP:{}] Log ({:?}): {}",
                self.name, params.level, params.data
            );
            let _ = self
                .injection_tx
                .send(McpInjection::Notification(msg))
                .await;
        }
    }
}
