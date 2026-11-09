use super::*;

/// TOOL-027: send an interrupt or end of input to a live exec session.
pub(super) mod control_exec_stdin {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
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
            .control_exec_stdin(
                tarpc::context::current(),
                args.session_id,
                args.exec_session_id,
                args.control,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(()).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}
