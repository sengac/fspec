use super::*;
pub(super) mod create_session_checked {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        role: Option<String>,
    }

    pub fn schema() -> Value {
        serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default()
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
            .create_session_checked(tarpc::context::current(), args.role)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(result).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

pub(super) mod session_manifest_issues {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {}

    pub fn schema() -> Value {
        serde_json::to_value(schemars::schema_for!(Args)).unwrap_or_default()
    }

    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        Ok(())
    }

    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        validate(value)?;
        let issues = client
            .session_manifest_issues(tarpc::context::current())
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(issues).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

pub(super) mod reply_interaction_checked {
    use super::*;
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        method: String,
        request_id: String,
        arguments: String,
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
            .reply_interaction_checked(
                tarpc::context::current(),
                args.session_id,
                args.method,
                args.request_id,
                args.arguments,
            )
            .await
            .map_err(|e| InvokeError::Transport(e.to_string()))?
            .map_err(|e| InvokeError::Daemon(format!("{e:?}")))?;
        serde_json::to_value(()).map_err(|e| InvokeError::Serialization(e.to_string()))
    }
}
