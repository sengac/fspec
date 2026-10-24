use super::*;
pub(super) mod checkpoint_diff_files {
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
        let value = client
            .checkpoint_diff_files(tarpc::context::current(), args.work_unit_id, args.name)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod checkpoint_file_diff {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        work_unit_id: String,
        name: String,
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
        let value = client
            .checkpoint_file_diff(
                tarpc::context::current(),
                args.work_unit_id,
                args.name,
                args.path,
            )
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod restore_checkpoint_file {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        work_unit_id: String,
        name: String,
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
        let value = client
            .restore_checkpoint_file(
                tarpc::context::current(),
                args.work_unit_id,
                args.name,
                args.path,
            )
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
