use super::{gemini, GeminiProvider, ProviderError};
use rig::client::CompletionClient;

impl GeminiProvider {
    pub fn from_test_client(client: gemini::Client, model: &str) -> Result<Self, ProviderError> {
        crate::native_test_endpoint::validate("gemini", client.base_url())?;
        Ok(Self {
            completion_model: client.completion_model(model),
            rig_client: client,
            model_name: model.to_owned(),
        })
    }
}
