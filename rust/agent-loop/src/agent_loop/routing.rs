use crate::thinking_config::ThinkingTarget;
use codelet_providers::local_reasoning::LiveCheck;
use codelet_rpc_types::{NotificationSeverity, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;

/// The dispatched provider name, the selected model and the turn's thinking
/// target. MODEL-010: the thinking wire is chosen from the dispatched
/// provider, never from the model id. MODEL-016: the first turn after a
/// local profile model is selected runs its live check.
pub(super) async fn resolve_turn_routing(
    session: &BackgroundSession,
) -> (String, Option<String>, ThinkingTarget) {
    let mut inner = session.inner.lock().await;
    let provider = inner
        .provider_manager()
        .facade_override()
        .map(|value| value.to_string())
        .unwrap_or_else(|| inner.current_provider_name().to_string());
    let model = inner.current_model_id().map(|value| value.to_string());
    let thinking_target = ThinkingTarget::for_provider_manager(inner.provider_manager());
    let live_check = inner.provider_manager_mut().take_live_check();
    drop(inner);
    tracing::debug!(
        "[AGENT-LOOP] current_provider={}, current_model={:?}, thinking_wire={:?}",
        provider,
        model,
        thinking_target.wire()
    );
    if let Some(check) = live_check {
        report_live_check(session, &check).await;
    }
    (provider, model, thinking_target)
}

/// MODEL-016: compare the engine's live report with the model's declared
/// levels and tell the user about a mismatch or a check that could not be
/// made. The declaration still applies either way; nothing sent changes.
///
/// Async home: the turn path is the first async place with the selected
/// model's declaration and the session's output; model selection is
/// synchronous and runs before a session can show a notice. It runs once
/// per selection (the check is taken from the provider manager).
async fn report_live_check(session: &BackgroundSession, check: &LiveCheck) {
    let report = check.run().await;
    match report.notice() {
        Some(notice) => {
            tracing::warn!(endpoint = check.endpoint(), "{notice}");
            session.handle_output(StreamChunk::user_notification(
                notice.to_string(),
                NotificationSeverity::Warning,
            ));
        }
        None => tracing::info!(
            endpoint = check.endpoint(),
            "live check: the engine reports the declared thinking levels"
        ),
    }
}
