//! RPC-054 — App-level dispatch tests for the ProviderSettingsView.
//!
//! Feature: spec/features/rpc054-provider-settings-dispatch.feature
//!
//! Drives the App::dispatch state machine end-to-end through the new
//! `Action::OpenProviderSettingsView` / `CloseProviderSettingsView` /
//! `SaveProviderCredentials` / `DeleteProviderCredentials` flow against
//! the shared `MockBackend` (scripted `list_provider_credentials` +
//! per-call counters from `tests/common/mod.rs`).

use crate::common::TaskOutcome;
use codelet_test_helpers::OrFail;
use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{ProviderCredentialInfo, SessionId};

mod common;
use common::MockBackend;

#[path = "provider_dispatch_rpc054/credentials.rs"]
mod credentials;
#[path = "provider_dispatch_rpc054/lifecycle.rs"]
mod lifecycle;
#[path = "provider_dispatch_rpc054/status.rs"]
mod status;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: ctype.to_string(),
        model_count: models,
        masked_key: None,
        source: None,
    }
}

/// Drain every pending tokio task spawned by `App::dispatch` AND fold
/// any queued action_tx messages back into the App. Mirrors the helper
/// in `pending_input_durability_rpc052.rs`.
async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

async fn wait_until<F: FnMut() -> bool>(
    mut predicate: F,
    label: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    codelet_test_helpers::deadlock_guard(label, async {
        loop {
            if predicate() {
                return;
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await
    .or_fail(&format!("timed out waiting for: {label}"))?;
    Ok(())
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}
