//! BUG-664: a Provider Settings sign-in success refreshes the view through
//! the BUG-575 numbered refresh, so an older refresh still in flight never
//! shows stale rows or moves the cursor; the old unnumbered navigate path
//! (ProviderCredentialsLoaded + set_navigate_target) is gone.
//!
//! Feature: spec/features/provider-settings-sign-in-success-uses-the-numbered-refresh.feature
//!
//! Deterministic: MockBackend's call gates (`tests/common/call_gates.rs`)
//! hold each credentials read open, so the test decides in which order the
//! reads answer; waits are bounded by `deadlock_guard`, never a clock.

use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::ProviderCredentialInfo;
use codelet_test_helpers::{deadlock_guard, OrFail, TestError, TestResult};

mod common;
use common::{MockBackend, TaskOutcome};

const CREDENTIALS: &str = "list_provider_credentials";

fn credential(id: &str, configured: bool) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: "oauth".to_string(),
        model_count: 0,
        masked_key: None,
        source: None,
    }
}

/// anthropic signed in or out; codex is signed in throughout.
fn credentials(anthropic_signed_in: bool) -> Vec<ProviderCredentialInfo> {
    vec![
        credential("anthropic", anthropic_signed_in),
        credential("codex", true),
    ]
}

fn view(app: &App) -> &ProviderSettingsView {
    &app.navigator().provider_settings
}

fn anthropic_signed_in(view: &ProviderSettingsView) -> bool {
    view.providers
        .iter()
        .any(|provider| provider.provider_id == "anthropic" && provider.configured)
}

fn on_anthropic_row(view: &ProviderSettingsView) -> bool {
    view.focused_nav_item().is_some_and(|item| {
        item.provider_id == "anthropic" && matches!(item.kind, NavItemKind::Provider { .. })
    })
}

/// Dispatch arriving actions until `ready` holds.
async fn pump_until(
    app: &mut App,
    what: &str,
    mut ready: impl FnMut(&App) -> bool,
) -> Result<(), TestError> {
    deadlock_guard(what, async {
        loop {
            while let Some(action) = app.try_recv_action() {
                app.dispatch(action);
            }
            if ready(app) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

/// Finish every task and dispatch everything (no gate may be holding).
async fn drain(app: &mut App) -> Result<(), TestError> {
    deadlock_guard("every task to finish", async {
        loop {
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
            let Some(action) = app.try_recv_action() else {
                return Ok::<(), TestError>(());
            };
            app.dispatch(action);
        }
    })
    .await??;
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_sign_in_success_while_an_older_refresh_is_in_flight_shows_the_newest_state() -> TestResult
{
    // @step Given Provider Settings is open with anthropic signed out and a refresh of the provider settings in flight
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(credentials(false));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::OpenProviderSettingsView);
    drain(&mut app).await?;
    let codex = view(&app)
        .nav_items
        .iter()
        .position(|item| item.provider_id == "codex")
        .or_fail("the codex row")?;
    app.navigator_mut().provider_settings.selected_index = codex;
    mock.gates.gate(CREDENTIALS);
    app.dispatch(Action::RefreshProviderSettings { navigate: None });
    pump_until(&mut app, "the older refresh's credentials read", |_| {
        mock.gates.held(CREDENTIALS) == 1
    })
    .await?;

    // @step When the anthropic sign-in succeeds and its own refresh answers with anthropic signed in
    mock.seed_provider_credentials(credentials(true));
    let generation = view(&app).oauth_generation();
    app.dispatch(Action::OAuthLoginSucceeded {
        provider_id: "anthropic".to_string(),
        generation,
    });
    pump_until(&mut app, "the sign-in's credentials read", |_| {
        mock.gates.held(CREDENTIALS) == 2
    })
    .await?;
    mock.gates.release(CREDENTIALS, 1);
    pump_until(&mut app, "the sign-in's refresh to be shown", |app| {
        anthropic_signed_in(view(app))
    })
    .await?;

    // @step And the older refresh then answers with anthropic signed out
    mock.seed_provider_credentials(credentials(false));
    mock.gates.open(CREDENTIALS);
    drain(&mut app).await?;

    // @step Then the view shows anthropic signed in
    assert!(
        anthropic_signed_in(view(&app)),
        "the older refresh must not replace the newer state: {:?}",
        view(&app).providers
    );

    // @step And the cursor is on the anthropic row
    assert!(
        on_anthropic_row(view(&app)),
        "{:?}",
        view(&app).focused_nav_item()
    );
    Ok(())
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> TestResult {
    for entry in std::fs::read_dir(dir).or_fail("read a source directory")? {
        let path = entry.or_fail("read a directory entry")?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[test]
fn the_old_unnumbered_navigate_path_is_gone() -> TestResult {
    // @step Given the fspec-tui source outside tests
    let mut files = Vec::new();
    rust_files(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    )?;

    // @step When it is scanned for the old sign-in navigate path
    let mut found = Vec::new();
    for path in files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "tests.rs" || name.ends_with("_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).or_fail("read a source file")?;
        for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
            for gone in [
                "ProviderCredentialsLoaded",
                "set_navigate_target",
                "apply_unnumbered_credentials",
                "pending_navigate",
            ] {
                if line.contains(gone) {
                    found.push(format!("{}: {gone}", path.display()));
                }
            }
        }
    }

    // @step Then it has no ProviderCredentialsLoaded, set_navigate_target, apply_unnumbered_credentials or pending_navigate
    assert!(found.is_empty(), "the old path is still there: {found:?}");
    Ok(())
}
