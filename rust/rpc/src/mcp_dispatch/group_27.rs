use super::*;
pub(super) mod get_hitl_request {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
    }
    pub fn schema() -> Value {
        serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default()
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let _ = &args;
        let value = client
            .get_hitl_request(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod get_exec_stdin_request {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
    }
    pub fn schema() -> Value {
        serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default()
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let _ = &args;
        let value = client
            .get_exec_stdin_request(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod write_exec_stdin {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    }
    pub fn schema() -> Value {
        serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default()
    }
    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        Ok(())
    }
    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args =
            serde_json::from_value(value).map_err(|e| InvokeError::Arguments(e.to_string()))?;
        let _ = &args;
        let value = client
            .write_exec_stdin(
                tarpc::context::current(),
                args.session_id,
                args.exec_session_id,
                args.text,
            )
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
