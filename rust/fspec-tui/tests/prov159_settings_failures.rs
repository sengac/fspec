//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//! Feature: spec/features/prov159-settings-failures.feature

mod common;

use crate::common::TaskOutcome;
use codelet_fspec_tui::{components::Action, App};
use std::sync::Arc;

#[tokio::test]
async fn scenario_a16_failed_logout_reports_safe_failure_in_settings() -> anyhow::Result<()> {
    // @step Given the OAuth logout backend returns an error containing a secret token
    // @step When provider settings completes the logout action
    // @step Then settings reports logout failure without exposing backend internals or the secret token
    let backend = Arc::new(common::MockBackend::new());
    backend.set_oauth_clear_tokens_error("oauth_clear_tokens failed: secret-access-token".into());
    let mut app = App::new(backend);
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "codex".into(),
    });
    while let Some(task) = app.next_pending_task() {
        codelet_test_helpers::deadlock_guard("the pending task", task)
            .await?
            .completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    let status = &app.navigator().provider_settings.status;
    assert!(
        status.contains('✗') && status.contains("logout"),
        "missing failure: {status}"
    );
    assert!(!status.contains("oauth_clear_tokens"));
    assert!(!status.contains("secret-access-token"));
    Ok(())
}
