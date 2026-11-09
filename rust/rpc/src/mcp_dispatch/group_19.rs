use super::*;
pub(super) mod set_work_unit_focus {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        #[serde(rename = "workUnitId")]
        work_unit_id: Option<String>,
        source: String,
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
            .set_work_unit_focus(
                tarpc::context::current(),
                args.session_id,
                args.work_unit_id,
                args.source,
            )
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod get_work_unit_context {
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
            .get_work_unit_context(tarpc::context::current(), args.session_id)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(value).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
pub(super) mod set_work_unit_context {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        context: Option<WorkUnitContext>,
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
        client
            .set_work_unit_context(tarpc::context::current(), args.session_id, args.context)
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
