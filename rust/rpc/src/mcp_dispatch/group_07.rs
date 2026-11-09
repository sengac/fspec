use super::*;
pub(super) mod get_thinking_level {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
    }
    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let value = client
            .get_thinking_level(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod get_thinking_options {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
    }
    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let value = client
            .get_thinking_options(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod get_workspace_info {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {}
    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let _args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let value = client
            .get_workspace_info(tarpc::context::current())
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod search_files {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        prefix: String,
        limit: u32,
    }
    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let value = client
            .search_files(tarpc::context::current(), args.prefix, args.limit)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
