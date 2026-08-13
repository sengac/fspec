//! RPC-105 — Provider settings header line shows total nav items, not configured count.
//!
//! Feature: spec/features/rpc105-provider-settings-header-nav-item-count.feature
//!
//! Verifies that `ProviderSettingsView::title_text()` returns
//! "Provider Settings (N items)" where `N == nav_items.len()`, and that
//! the count is reactive to expand/collapse and to filter mutations.
//!
//! Tests are written test-first per ACDD: they FAIL against the current
//! mod.rs (which still renders "(M configured)" via `configured_count()`)
//! and PASS once RPC-105 lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::nav_item::{OAuthMethod, ProviderDisplayInfo};
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsView};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

const CANONICAL_PROVIDER_IDS: [&str; 17] = [
    "openai",
    "anthropic",
    "cohere",
    "gemini",
    "mistral",
    "xai",
    "together",
    "huggingface",
    "openrouter",
    "groq",
    "deepseek",
    "moonshot",
    "galadriel",
    "azure",
    "zai",
    "codex",
    "github-copilot",
];

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Build a plain non-oauth ProviderDisplayInfo with NO children when collapsed.
fn plain_info(id: &str) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: id.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: false,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

/// Anthropic shaped to inject exactly 3 child rows on expansion:
/// oauth-status (1) + oauth-login×1 (1) + api-key (1) = 3.
fn anthropic_with_3_children() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: true,
        is_oauth_provider: true,
        requires_api_key: true,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: vec![(OAuthMethod::Browser, "Browser login".to_string())],
        oauth_status_label: Some("Logout from OAuth [Anthropic]".to_string()),
        masked_key: None,
        source: None,
    }
}

/// 17 canonical providers — all plain (no children when collapsed),
/// except `anthropic` which has 3 child rows when expanded.
fn seventeen_provider_infos() -> Vec<ProviderDisplayInfo> {
    CANONICAL_PROVIDER_IDS
        .iter()
        .map(|id| {
            if *id == "anthropic" {
                anthropic_with_3_children()
            } else {
                plain_info(id)
            }
        })
        .collect()
}

fn view_with_nav_items(infos: Vec<ProviderDisplayInfo>) -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_provider_display_infos(infos);
    // PROV-113: browser OAuth-login rows are gated out of the nav tree unless
    // the transport supports the local OAuth server. These RPC-105 count
    // fixtures shape anthropic to contribute a Browser oauth-login child row,
    // so enable browser login to count the full tree (matches the embedded
    // transport the App runs under).
    v.set_browser_login_enabled(true);
    v
}

fn type_filter(view: &mut ProviderSettingsView, text: &str) {
    // Enter filter mode via the `/` keybind handled by `view.handle_key`,
    // then dispatch each character through the same public entry point
    // so the filter mutations flow through whatever reactivity the view
    // wires up (RPC-105: rebuild_nav_items on every filter delta).
    let _ = view.handle_key(key(KeyCode::Char('/')));
    for c in text.chars() {
        let _ = view.handle_key(key(KeyCode::Char(c)));
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Title shows nav item count for all collapsed providers
// ────────────────────────────────────────────────────────────────────────
#[test]
fn title_shows_nav_item_count_for_collapsed_providers() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let infos = seventeen_provider_infos();
    let view = view_with_nav_items(infos);

    // @step And no providers are expanded
    assert!(view.expanded.is_empty(), "expanded set should be empty");

    // @step And no filter is applied
    assert!(view.filter.is_empty(), "filter should be empty");

    // @step When I read the title text
    let title = view.title_text();

    // @step Then the title text equals "Provider Settings (17 items)"
    assert_eq!(title, "Provider Settings (17 items)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Title count grows when a provider is expanded
// ────────────────────────────────────────────────────────────────────────
#[test]
fn title_count_grows_when_provider_expanded() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let mut view = view_with_nav_items(seventeen_provider_infos());

    // @step And no providers are expanded
    assert!(view.expanded.is_empty());

    // @step And no filter is applied
    assert!(view.filter.is_empty());

    // @step When I toggle expansion of "anthropic" which has 3 children
    view.toggle_expansion("anthropic");

    // @step Then the title text equals "Provider Settings (20 items)"
    assert_eq!(view.title_text(), "Provider Settings (20 items)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Title count shrinks when a filter is applied
// ────────────────────────────────────────────────────────────────────────
#[test]
fn title_count_shrinks_when_filter_applied() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let mut view = view_with_nav_items(seventeen_provider_infos());

    // @step And no providers are expanded
    assert!(view.expanded.is_empty());

    // @step When the user types the filter character sequence "openai"
    type_filter(&mut view, "openai");

    // @step Then the title text equals "Provider Settings (1 items)"
    assert_eq!(view.title_text(), "Provider Settings (1 items)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Title count reflects both filter and expansion
// ────────────────────────────────────────────────────────────────────────
#[test]
fn title_count_reflects_filter_and_expansion_together() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let mut view = view_with_nav_items(seventeen_provider_infos());

    // @step And "anthropic" is expanded with 3 children
    view.toggle_expansion("anthropic");
    assert_eq!(view.title_text(), "Provider Settings (20 items)");

    // @step When the user types the filter character sequence "anth"
    type_filter(&mut view, "anth");

    // @step Then the title text equals "Provider Settings (4 items)"
    assert_eq!(view.title_text(), "Provider Settings (4 items)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Title text never contains the substring "configured"
// ────────────────────────────────────────────────────────────────────────
#[test]
fn title_text_never_contains_configured() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let view = view_with_nav_items(seventeen_provider_infos());

    // @step When I read the title text
    let title = view.title_text();

    // @step Then the title text does not contain "configured"
    assert!(
        !title.contains("configured"),
        "title must NOT contain 'configured', got {title:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Title uses nav_items length not configured count
// ────────────────────────────────────────────────────────────────────────
#[test]
fn title_uses_nav_items_length_not_configured_count() {
    // @step Given a ProviderSettingsView with 5 providers loaded as raw credential infos where 2 are configured
    let mut view = ProviderSettingsView::new();
    let raw = vec![
        ProviderCredentialInfo {
            provider_id: "openai".into(),
            display_name: "OpenAI".into(),
            configured: true,
            credential_type: "api_key".into(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
        ProviderCredentialInfo {
            provider_id: "anthropic".into(),
            display_name: "Anthropic".into(),
            configured: true,
            credential_type: "api_key".into(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
        ProviderCredentialInfo {
            provider_id: "cohere".into(),
            display_name: "Cohere".into(),
            configured: false,
            credential_type: "api_key".into(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
        ProviderCredentialInfo {
            provider_id: "gemini".into(),
            display_name: "Gemini".into(),
            configured: false,
            credential_type: "api_key".into(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
        ProviderCredentialInfo {
            provider_id: "mistral".into(),
            display_name: "Mistral".into(),
            configured: false,
            credential_type: "api_key".into(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
    ];
    view.set_providers(raw);

    // @step And 5 providers loaded as display infos
    let infos: Vec<ProviderDisplayInfo> = ["openai", "anthropic", "cohere", "gemini", "mistral"]
        .iter()
        .map(|id| plain_info(id))
        .collect();
    view.set_provider_display_infos(infos);

    // @step When I read the title text
    let title = view.title_text();

    // @step Then the title text equals "Provider Settings (5 items)"
    assert_eq!(title, "Provider Settings (5 items)");

    // @step And the title text does not contain "2 configured"
    assert!(!title.contains("2 configured"));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Backspacing a filter character grows the title count back
// ────────────────────────────────────────────────────────────────────────
#[test]
fn backspacing_filter_grows_title_count() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let mut view = view_with_nav_items(seventeen_provider_infos());

    // @step And the user has typed the filter "openai"
    type_filter(&mut view, "openai");
    assert_eq!(view.filter, "openai");
    assert_eq!(view.title_text(), "Provider Settings (1 items)");

    // @step When the user presses Backspace once
    let _ = view.handle_key(key(KeyCode::Backspace));

    // @step Then the filter equals "openai" minus the last character
    assert_eq!(view.filter, "openai".strip_suffix('i').unwrap());

    // @step And the title text equals "Provider Settings (N items)" where N matches the new filtered nav_items length
    let expected = format!("Provider Settings ({} items)", view.nav_items.len());
    assert_eq!(view.title_text(), expected);
    // The shorter prefix "opena" still only matches "openai" — count 1.
    assert_eq!(view.title_text(), "Provider Settings (1 items)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Clearing the filter via Esc restores the full count
// ────────────────────────────────────────────────────────────────────────
#[test]
fn esc_in_filter_mode_restores_full_count() {
    // @step Given a ProviderSettingsView with 17 canonical providers loaded as display infos
    let mut view = view_with_nav_items(seventeen_provider_infos());

    // @step And the user has typed the filter "openai"
    type_filter(&mut view, "openai");
    assert_eq!(view.filter, "openai");
    assert_eq!(view.title_text(), "Provider Settings (1 items)");

    // @step When the user presses Esc while in filter mode
    let event = view.handle_key(key(KeyCode::Esc));
    assert!(matches!(event, ProviderSettingsEvent::Consumed));

    // @step Then the filter is empty
    assert!(view.filter.is_empty());

    // @step And the title text equals "Provider Settings (17 items)"
    assert_eq!(view.title_text(), "Provider Settings (17 items)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: configured_count method is removed
// ────────────────────────────────────────────────────────────────────────
#[test]
fn configured_count_method_is_removed_from_source() {
    // @step Given the ProviderSettingsView source file
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/views/provider_settings/mod.rs"
    ))
    .expect("mod.rs must exist");

    // @step When I search for the method name "configured_count"
    let has_method = source.contains("fn configured_count");

    // @step Then the source file does not declare a "configured_count" method
    assert!(
        !has_method,
        "mod.rs must no longer declare 'fn configured_count'"
    );
}
