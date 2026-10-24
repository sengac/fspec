//! PROV-116 — profile delete restores cursor to the parent provider row
//! (PROV-036 parity).
//!
//! Feature: spec/features/provider-settings-profile-delete-navigation.feature
//!
//! App-level dispatch tests against the shared `MockBackend` (scripted Ok/Err +
//! call counters). The success path must wire `set_navigate_target(provider_id)`
//! so the reload fold's `apply_pending_navigate()` lands the cursor on the
//! parent provider row; the Err path must NOT jump; the save path must NOT be
//! changed (TS only navigates on delete).
//!
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::{
    ProfileDefinition, ProviderCredentialInfo, ProviderProfileConfig, ProviderSettingsConfig,
};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn seed_backend_profiles(mock: &MockBackend, profiles: &[&str]) {
    mock.seed_provider_settings(vec![ProviderSettingsConfig {
        provider_id: "openai".to_string(),
        settings: ProfileDefinition::default(),
        profiles: profiles
            .iter()
            .map(|name| ProviderProfileConfig {
                name: (*name).to_string(),
                definition: profile_def(),
            })
            .collect(),
    }])
    .expect("backend provider settings fixture must be available");
}

// ─────────────────────────── dispatch helpers ───────────────────────────

fn openai_pinfo() -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: "openai".to_string(),
        display_name: "OpenAI".to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 0,
        masked_key: None,
        source: None,
    }
}

/// A second/third top-level (non-OAuth) provider. Having more than one
/// top-level provider keeps the `set_providers` safety clamp (which pins
/// `selected_index` to the top-level provider count) from masking the navigate
/// behaviour we are actually testing.
fn other_pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 0,
        masked_key: Some("sk-…key".to_string()),
        source: Some("env".to_string()),
    }
}

/// The full backend credential list: openai first (the provider under test)
/// plus two simple api-key providers so the top-level count is 3.
fn cred_list() -> Vec<ProviderCredentialInfo> {
    vec![openai_pinfo(), other_pinfo("alpha"), other_pinfo("beta")]
}

fn profile_def() -> ProfileDefinition {
    ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
        streaming: None,
        auto_continue: None,
        preserve_thinking: None,
        max_images: None,
        loop_detection_enabled: None,
        loop_detection_window: None,
        loop_detection_max_repeats: None,
        loop_detection_max_retries: None,
    }
}

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

/// True when the cursor is on the openai PROVIDER row.
fn cursor_on_openai_provider_row(view: &ProviderSettingsView) -> bool {
    matches!(
        view.focused_nav_item()
            .map(|i| (&i.kind, i.provider_id.as_str())),
        Some((NavItemKind::Provider { .. }, "openai"))
    )
}

/// Profile-name child rows currently shown under openai. The nav label carries
/// a `"name → baseUrl"` display string, so strip the suffix to the bare name.
fn profile_rows(view: &ProviderSettingsView) -> Vec<String> {
    view.nav_items
        .iter()
        .filter_map(|i| match &i.kind {
            NavItemKind::Profile { profile_name } => Some(
                profile_name
                    .split(" → ")
                    .next()
                    .unwrap_or(profile_name)
                    .to_string(),
            ),
            _ => None,
        })
        .collect()
}

/// Index of the first `Profile` row (the cursor's pre-delete landing spot).
fn first_profile_index(view: &ProviderSettingsView) -> usize {
    view.nav_items
        .iter()
        .position(|i| matches!(i.kind, NavItemKind::Profile { .. }))
        .expect("a profile row must be present in the fixture")
}

async fn app_expanded_on_first_profile(mock: Arc<MockBackend>, profiles: &[&str]) -> App {
    seed_backend_profiles(&mock, profiles);
    mock.seed_provider_credentials(cred_list());
    let mut app = fresh_app(mock);
    app.dispatch(Action::ProviderCredentialsLoaded(cred_list()));
    drain_pending(&mut app).await;
    let view = &mut app.navigator_mut().provider_settings;
    view.toggle_expansion("openai");
    view.selected_index = first_profile_index(view);
    app
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Deleting one of several profiles returns the cursor to the
// provider row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_one_of_several_profiles_returns_cursor_to_provider_row() {
    // @step Given the "openai" provider is expanded with profiles "fireworks" and "home"
    let mock = Arc::new(MockBackend::new());
    let mut app = app_expanded_on_first_profile(mock.clone(), &["fireworks", "home"]).await;
    assert_eq!(
        profile_rows(&app.navigator().provider_settings),
        vec!["fireworks", "home"]
    );

    // @step And the cursor is on the "fireworks" profile row
    assert!(!cursor_on_openai_provider_row(
        &app.navigator().provider_settings
    ));

    // The delete removes "fireworks" from the store: the reload now sees only
    // "home".
    seed_backend_profiles(&mock, &["home"]);

    // @step When the user presses "d" and confirms the delete with "y"
    app.dispatch(Action::ConfirmDeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "fireworks".to_string(),
    });

    // @step And the backend delete succeeds and the nav tree refreshes
    drain_pending(&mut app).await;
    wait_until(|| mock.delete_profile_calls() >= 1, "delete_profile called").await;

    // @step Then the cursor is on the "openai" provider row
    assert!(
        cursor_on_openai_provider_row(&app.navigator().provider_settings),
        "after a successful delete the cursor must return to the openai provider row"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Deleting the only profile returns the cursor to the provider row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_the_only_profile_returns_cursor_to_provider_row() {
    // @step Given the "openai" provider is expanded with a single profile "fireworks"
    let mock = Arc::new(MockBackend::new());
    let mut app = app_expanded_on_first_profile(mock.clone(), &["fireworks"]).await;
    assert_eq!(
        profile_rows(&app.navigator().provider_settings),
        vec!["fireworks"]
    );

    // @step And the cursor is on the "fireworks" profile row
    assert!(!cursor_on_openai_provider_row(
        &app.navigator().provider_settings
    ));

    // The delete removes the profile from the store: the reload must now see an
    // openai with no profiles.
    seed_backend_profiles(&mock, &[]);

    // @step When the user presses "d" and confirms the delete with "y"
    app.dispatch(Action::ConfirmDeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "fireworks".to_string(),
    });

    // @step And the backend delete succeeds and the nav tree refreshes
    drain_pending(&mut app).await;
    wait_until(|| mock.delete_profile_calls() >= 1, "delete_profile called").await;

    // @step Then the cursor is on the "openai" provider row
    assert!(
        cursor_on_openai_provider_row(&app.navigator().provider_settings),
        "after deleting the only profile the cursor must return to the openai provider row"
    );

    // @step And Settings and "+ Add Profile" are the only children shown
    assert!(
        profile_rows(&app.navigator().provider_settings).is_empty(),
        "no profile rows must remain after deleting the only profile"
    );
    let children: Vec<_> = app
        .navigator()
        .provider_settings
        .nav_items
        .iter()
        .filter(|item| {
            item.provider_id == "openai" && !matches!(item.kind, NavItemKind::Provider { .. })
        })
        .map(|item| item.kind.clone())
        .collect();
    assert_eq!(
        children,
        vec![NavItemKind::Settings, NavItemKind::AddProfile]
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A failed delete does not move the cursor and preserves the profiles
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_delete_does_not_move_cursor_and_preserves_profiles() {
    let mock = Arc::new(MockBackend::new());
    // @step Given the "openai" provider is expanded with profiles "fireworks" and "home"
    // @step And the cursor is on the "fireworks" profile row
    mock.set_delete_profile_error("delete failed".to_string());
    let mut app = app_expanded_on_first_profile(mock.clone(), &["fireworks", "home"]).await;
    assert!(!cursor_on_openai_provider_row(
        &app.navigator().provider_settings
    ));

    // @step When the user presses "d" and confirms the delete with "y"
    app.dispatch(Action::ConfirmDeleteProfile {
        provider_id: "openai".to_string(),
        profile_name: "fireworks".to_string(),
    });

    // @step And the backend delete returns an error
    drain_pending(&mut app).await;
    wait_until(
        || mock.delete_profile_calls() >= 1,
        "delete_profile attempted",
    )
    .await;

    // @step Then the cursor does not jump to the "openai" provider row
    assert!(
        !cursor_on_openai_provider_row(&app.navigator().provider_settings),
        "a failed delete must NOT move the cursor to the openai provider row"
    );

    // @step And both profiles "fireworks" and "home" are still present
    assert_eq!(
        profile_rows(&app.navigator().provider_settings),
        vec!["fireworks", "home"],
        "a failed delete must preserve both profiles"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Saving a profile does not move the cursor to the provider row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_a_profile_does_not_move_cursor_to_provider_row() {
    // @step Given the "openai" provider is expanded with profiles "fireworks" and "home"
    let mock = Arc::new(MockBackend::new());
    let mut app = app_expanded_on_first_profile(mock.clone(), &["fireworks", "home"]).await;

    // @step And the cursor is on the "fireworks" profile row
    assert!(!cursor_on_openai_provider_row(
        &app.navigator().provider_settings
    ));

    // @step When the user edits and saves the "fireworks" profile
    app.dispatch(Action::SaveProfile {
        provider_id: "openai".to_string(),
        profile_name: "fireworks".to_string(),
        old_profile_name: None,
        definition: profile_def(),
    });

    // @step And the backend save succeeds and the nav tree refreshes
    drain_pending(&mut app).await;
    wait_until(|| mock.save_profile_calls() >= 1, "save_profile called").await;

    // @step Then the cursor is not forced onto the "openai" provider row
    assert!(
        !cursor_on_openai_provider_row(&app.navigator().provider_settings),
        "saving a profile must NOT force the cursor onto the provider row (TS navigates only on delete)"
    );
}
