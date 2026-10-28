use super::*;
pub(super) mod set_debug_directory {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        path: String,
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
        client
            .set_debug_directory(tarpc::context::current(), args.path)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod pause_resume {
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
        client
            .pause_resume(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod pause_confirm {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        accept: bool,
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
        client
            .pause_confirm(tarpc::context::current(), args.session_id, args.accept)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
