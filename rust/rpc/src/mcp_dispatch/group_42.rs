use super::*;

/// MCP-003: set one session's whole MCP server set (MCP-008; daemon-bearer only).
pub(super) mod configure_session_mcp_servers {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    }

    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }

    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        Ok(())
    }

    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        let result = client
            .configure_session_mcp_servers(tarpc::context::current(), args.session_id, args.servers)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?;
        serde_json::to_value(result).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}
