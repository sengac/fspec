use super::*;

/// RPC-443: workspace listing for multi-workspace daemons.
pub(super) mod list_workspaces {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {}

    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }

    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        Ok(())
    }

    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        let workspaces = client
            .list_workspaces(tarpc::context::current())
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(workspaces)
            .map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-443: a named workspace's work units.
pub(super) mod list_workspace_work_units {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        workspace: String,
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
        let units = client
            .list_workspace_work_units(tarpc::context::current(), args.workspace)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(units).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-444: create a session in a named workspace.
pub(super) mod create_session_in_workspace {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        workspace: String,
        role: Option<String>,
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
        let id = client
            .create_session_in_workspace(tarpc::context::current(), args.workspace, args.role)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(id).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-440: the daemon's default model ("provider/model"), or null.
pub(super) mod get_default_model {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {}

    pub fn schema() -> Result<Value, serde_json::Error> {
        serde_json::to_value(schemars::schema_for!(Args))
    }

    pub fn validate(value: Value) -> Result<(), InvokeError> {
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        Ok(())
    }

    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        let model = client
            .get_default_model(tarpc::context::current())
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?;
        serde_json::to_value(model).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RIG-016: submit input to a session as a steer, queue entry or interrupt.
pub(super) mod submit_input {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
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
            .submit_input(
                tarpc::context::current(),
                args.session_id,
                args.text,
                args.mode,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?;
        serde_json::to_value(result).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}
