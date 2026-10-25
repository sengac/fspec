use super::CopilotProvider;
use rig::client::CompletionClient;

impl CopilotProvider {
    pub(crate) fn bind_session(mut self, state: &crate::session::ProviderSession) -> Self {
        if let Some(extension) =
            state.extension::<crate::copilot::session::CopilotSession>("github-copilot")
        {
            self.rig_client.bind_session(extension.clone());
            self.completion_model = self.rig_client.completion_model(&self.model_name);
        }
        self
    }
}
