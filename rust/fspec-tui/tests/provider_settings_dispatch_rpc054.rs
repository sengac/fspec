//! RPC-054 — App-level dispatch tests for the ProviderSettingsView.
//!
//! Feature: spec/features/rpc054-provider-settings-dispatch.feature
//!
//! Drives the App::dispatch state machine end-to-end through the new
//! `Action::OpenProviderSettingsView` / `CloseProviderSettingsView` /
//! `SaveProviderCredentials` / `TestProviderConnection` /
//! `RefreshProviderModels` / `DeleteProviderCredentials` flow against
//! the shared `MockBackend` (scripted `list_provider_credentials` +
//! per-call counters from `tests/common/mod.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{ModelEntry, ProviderCredentialInfo, SessionId, TestConnectionResult};
use tokio::time::timeout;

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
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}
