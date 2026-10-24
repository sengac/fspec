use super::RigAgent;
use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::SelectedRequestPreparer;
use rig::completion::{Completion, CompletionModel};

impl<M: CompletionModel + 'static> RigAgent<M> {
    pub fn with_output_limit(mut self, limit: Option<u64>) -> Self {
        self.agent.max_tokens = limit;
        self
    }

    pub async fn selected_request_preparer(
        &self,
        prompt: &str,
        limits: AdmissionLimits,
    ) -> Result<SelectedRequestPreparer, RequestAdmissionError> {
        limits.validate()?;
        let template = self
            .agent
            .completion(prompt, Vec::new())
            .await
            .map_err(|error| RequestAdmissionError::Preparation(error.to_string()))?
            .build();
        let preserve = self.preserve_thinking;
        Ok(
            SelectedRequestPreparer::new(self.agent.model.as_ref().clone(), template, limits)
                .map_history(move |messages| {
                    crate::history_strip::strip_reasoning_from_history(messages, preserve)
                }),
        )
    }
}
