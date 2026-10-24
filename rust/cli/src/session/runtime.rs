use super::Session;
use codelet_core::RigAgent;
use rig::completion::CompletionModel;

impl Session {
    pub fn apply_provider_runtime_settings(&mut self) {
        let continuation = self.provider_manager.runtime_options().auto_continue;
        self.continue_enabled = continuation.is_some_and(|budget| budget > 0);
        self.continue_budget = continuation
            .filter(|budget| *budget > 0)
            .unwrap_or(crate::interactive::auto_continue::DEFAULT_CONTINUE_BUDGET);
        self.preserve_thinking_enabled =
            crate::provider_runtime::preserve_thinking(&self.provider_manager);
    }

    pub fn configure_agent<M: CompletionModel + 'static>(&self, agent: RigAgent<M>) -> RigAgent<M> {
        agent
            .with_output_limit(crate::provider_runtime::output_limit(
                self.provider_manager(),
            ))
            .with_preserve_thinking(self.preserve_thinking_enabled)
    }
}
