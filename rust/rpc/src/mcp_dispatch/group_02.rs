use super::*;
pub(super) mod get_session_status {
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
            .get_session_status(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod health {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {}
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
            .health(tarpc::context::current())
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod checkpoint_counts {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {}
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
            .checkpoint_counts(tarpc::context::current())
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
