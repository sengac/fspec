use codelet_sessions::background_session::BackgroundSession;

pub(super) async fn resolve_turn_routing(
    session: &BackgroundSession,
) -> (String, Option<String>, Option<String>) {
    let inner = session.inner.lock().await;
    let provider = inner
        .provider_manager()
        .facade_override()
        .map(|value| value.to_string())
        .unwrap_or_else(|| inner.current_provider_name().to_string());
    let model = inner.current_model_id().map(|value| value.to_string());
    let real_provider_name = inner.current_provider_name().to_string();
    let thinking_key = if matches!(
        inner.provider_manager().current_provider_type(),
        codelet_providers::ProviderType::Custom(_)
    ) {
        model.as_deref().and_then(|alias| {
            codelet_providers::custom::resolve_custom_model_id(&real_provider_name, alias)
        })
    } else {
        None
    };
    tracing::debug!(
        "[AGENT-LOOP] current_provider={}, current_model={:?}, thinking_model_key={:?}",
        provider,
        model,
        thinking_key
    );
    (provider, model, thinking_key)
}
