use super::*;

/// WT-018: `session_card_changes` through the daemon.
pub(super) mod session_card_changes {
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
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        Ok(())
    }

    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        let answer = client
            .session_card_changes(tarpc::context::current(), args.session_id)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(answer).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// WT-018: `move_session_cards_back` through the daemon.
pub(super) mod move_session_cards_back {
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
        let _: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        Ok(())
    }

    pub async fn call(client: &FspecServiceClient, value: Value) -> Result<Value, InvokeError> {
        let args: Args = serde_json::from_value(value)
            .map_err(|error| InvokeError::Arguments(error.to_string()))?;
        let answer = client
            .move_session_cards_back(tarpc::context::current(), args.session_id)
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(answer).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}

/// AMGR-023: `pause_human_action` through the daemon.
pub(super) mod pause_human_action {
    use super::*;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct Args {
        session_id: SessionId,
        /// The pending human action this reply answers.
        request_id: String,
        /// Done lets the waiting tool go on; Cancel stops it as an interrupt does.
        reply: HumanActionReply,
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
            .pause_human_action(
                tarpc::context::current(),
                args.session_id,
                args.request_id,
                args.reply,
            )
            .await
            .map_err(|error| InvokeError::Transport(error.to_string()))?
            .map_err(InvokeError::Daemon)?;
        serde_json::to_value(()).map_err(|error| InvokeError::Serialization(error.to_string()))
    }
}
