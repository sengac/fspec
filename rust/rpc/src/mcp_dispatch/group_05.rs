use super::*;
pub(super) mod restore_checkpoint_all {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        work_unit_id: String,
        name: String,
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
            .restore_checkpoint_all(tarpc::context::current(), args.work_unit_id, args.name)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod delete_checkpoint {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        work_unit_id: String,
        name: String,
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
            .delete_checkpoint(tarpc::context::current(), args.work_unit_id, args.name)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod delete_all_checkpoints {
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
        client
            .delete_all_checkpoints(tarpc::context::current())
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
