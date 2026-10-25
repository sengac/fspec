use super::{AuthMode, ClaudeClient, ClaudeProvider, ProviderError};

impl ClaudeProvider {
    pub fn from_test_client(
        client: ClaudeClient,
        model: &str,
        auth_mode: AuthMode,
    ) -> Result<Self, ProviderError> {
        crate::native_test_endpoint::validate("claude", client.base_url())?;
        Ok(Self {
            completion_model: rig::providers::anthropic::completion::CompletionModel::new(
                client.clone(),
                model,
            )
            .with_prompt_caching(),
            rig_client: client,
            auth_mode,
            model_name: model.to_owned(),
        })
    }
}
