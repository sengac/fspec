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
}
