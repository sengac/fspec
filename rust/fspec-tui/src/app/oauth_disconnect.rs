use super::{Action, App};

pub(super) fn start(app: &mut App, provider_id: String) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    app.navigator
        .provider_settings
        .set_navigate_target(provider_id.clone());
    let backend = app.backend.clone();
    let action_tx = app.action_tx.clone();
    let handle = tokio::spawn(async move {
        if let Err(error) = backend.oauth_clear_tokens(provider_id.clone()).await {
            tracing::warn!(%error, provider = %provider_id, "OAuth logout failed");
            let _ = action_tx.send(Action::ProviderSettingsStatus(
                "✗ OAuth logout failed; credentials may remain. Retry after checking storage access."
                    .into(),
            ));
        }
        if let Ok(list) = backend.list_provider_credentials().await {
            let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
        }
    });
    app.pending_tasks.push(handle);
}
