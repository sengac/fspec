use super::*;

/// RPC-453: `checkpoint_counts` for a named workspace through the daemon.
pub(super) mod checkpoint_counts_in_workspace {
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
        let answer = client
            .checkpoint_counts_in_workspace(tarpc::context::current(), args.workspace)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(answer).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-453: `list_checkpoints` for a named workspace through the daemon.
pub(super) mod list_checkpoints_in_workspace {
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
        let answer = client
            .list_checkpoints_in_workspace(tarpc::context::current(), args.workspace)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(answer).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-453: `checkpoint_diff_files` for a named workspace through the daemon.
pub(super) mod checkpoint_diff_files_in_workspace {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    pub struct Args {
        workspace: String,
        work_unit_id: String,
        name: String,
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
        let answer = client
            .checkpoint_diff_files_in_workspace(
                tarpc::context::current(),
                args.workspace,
                args.work_unit_id,
                args.name,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(answer).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-453: `checkpoint_file_diff` for a named workspace through the daemon.
pub(super) mod checkpoint_file_diff_in_workspace {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    pub struct Args {
        workspace: String,
        work_unit_id: String,
        name: String,
        path: String,
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
        let answer = client
            .checkpoint_file_diff_in_workspace(
                tarpc::context::current(),
                args.workspace,
                args.work_unit_id,
                args.name,
                args.path,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(answer).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// RPC-453: `restore_checkpoint_file` for a named workspace through the daemon.
pub(super) mod restore_checkpoint_file_in_workspace {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    pub struct Args {
        workspace: String,
        work_unit_id: String,
        name: String,
        path: String,
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
        client
            .restore_checkpoint_file_in_workspace(
                tarpc::context::current(),
                args.workspace,
                args.work_unit_id,
                args.name,
                args.path,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        Ok(Value::Null)
    }
}

/// RPC-453: `restore_checkpoint_all` for a named workspace through the daemon.
pub(super) mod restore_checkpoint_all_in_workspace {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    pub struct Args {
        workspace: String,
        work_unit_id: String,
        name: String,
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
        client
            .restore_checkpoint_all_in_workspace(
                tarpc::context::current(),
                args.workspace,
                args.work_unit_id,
                args.name,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        Ok(Value::Null)
    }
}

/// RPC-453: `delete_checkpoint` for a named workspace through the daemon.
pub(super) mod delete_checkpoint_in_workspace {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    pub struct Args {
        workspace: String,
        work_unit_id: String,
        name: String,
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
        client
            .delete_checkpoint_in_workspace(
                tarpc::context::current(),
                args.workspace,
                args.work_unit_id,
                args.name,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        Ok(Value::Null)
    }
}

/// RPC-453: `delete_all_checkpoints` for a named workspace through the daemon.
pub(super) mod delete_all_checkpoints_in_workspace {
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
        client
            .delete_all_checkpoints_in_workspace(tarpc::context::current(), args.workspace)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        Ok(Value::Null)
    }
}
