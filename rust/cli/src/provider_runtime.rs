use codelet_providers::ProviderManager;

pub(crate) fn output_limit(manager: &ProviderManager) -> Option<u64> {
    // PROV-165: an unavailable output limit is `None`, not a filtered 0.
    manager
        .should_send_max_output_tokens()
        .then(|| manager.max_output_tokens())
        .flatten()
        .map(|limit| limit as u64)
}

pub(crate) fn preserve_thinking(manager: &ProviderManager) -> bool {
    manager
        .runtime_options()
        .preserve_thinking
        .unwrap_or_else(|| {
            !(manager.current_provider_name() == "openai"
                && manager.selected_model_string().is_some_and(|selection| {
                    selection
                        .split_once('/')
                        .is_some_and(|(provider, _)| provider.contains(':'))
                }))
        })
}
