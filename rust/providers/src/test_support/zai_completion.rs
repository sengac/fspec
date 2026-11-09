use super::{openai, ProviderError, ZAIProvider};
use rig::client::CompletionClient;

impl ZAIProvider {
    pub fn from_test_client(
        client: openai::CompletionsClient,
        model: &str,
    ) -> Result<Self, ProviderError> {
        crate::native_test_endpoint::validate("zai", client.base_url())?;
        Ok(Self {
            completion_model: client.completion_model(model),
            rig_client: client,
            model_name: model.to_owned(),
            is_plan_endpoint: false,
        })
    }

    /// BUG-517: build the provider exactly as [`ZAIProvider::new_with_model`]
    /// does (same key and endpoint selection from the environment, same
    /// required model) but send its requests to a numeric loopback
    /// `endpoint`. [`ZAIProvider::base_url`] still reports the endpoint the
    /// key selected.
    pub fn new_with_model_at_test_endpoint(
        model: Option<&str>,
        endpoint: &str,
    ) -> Result<Self, ProviderError> {
        crate::native_test_endpoint::validate("zai", endpoint)?;
        let (api_key, is_plan) = Self::env_credentials()?;
        let model_name = Self::required_model(model)?;
        let client = openai::CompletionsClient::builder()
            .api_key(&api_key)
            .base_url(endpoint)
            .build()
            .map_err(|e| {
                ProviderError::config("zai", format!("Failed to build Z.AI client: {e}"))
            })?;
        Ok(Self {
            completion_model: client.completion_model(&model_name),
            rig_client: client,
            model_name,
            is_plan_endpoint: is_plan,
        })
    }
}
