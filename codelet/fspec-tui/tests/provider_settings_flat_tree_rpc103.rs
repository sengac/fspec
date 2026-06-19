//! RPC-103 — Flat tree NavItem model + expansion mechanics integration tests.
//!
//! Feature: spec/features/rpc103-provider-settings-flat-tree-nav-model.feature
//!
//! Validates the new `nav_item` module (NavItem, NavItemKind, OAuthMethod,
//! ProviderDisplayInfo, build_nav_items) plus the ProviderSettingsView
//! mechanics that consume it (`set_provider_display_infos`,
//! `toggle_expansion`, `focused_nav_item`, Enter dispatch on Provider /
//! ApiKey nav-items).
//!
//! Scope boundary: this card covers the data model + builder + view
//! mechanics. Row rendering (RPC-104), footer hints (RPC-106) and the
//! full DetailSub removal are deferred to sibling cards. Enter on
//! ApiKey routes directly to the existing `DetailSub::EditApiKey { draft }`
//! state for backward compatibility.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_fspec_tui::views::provider_settings::nav_item::{
    build_nav_items, NavItemKind, OAuthMethod, ProviderDisplayInfo,
};
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Construct an anthropic-shaped OAuth provider with two login methods
/// (Browser + Headless) and an api-key fallback.
fn anthropic(has_oauth_tokens: bool) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        configured: false,
        credential_type: "oauth".to_string(),
        model_count: 8,
        has_oauth_tokens,
        is_oauth_provider: true,
        requires_api_key: true,
        env_var: Some("ANTHROPIC_API_KEY".to_string()),
        profiles: Vec::new(),
        oauth_login_methods: vec![
            (OAuthMethod::Browser, "Sign in with browser".to_string()),
            (OAuthMethod::Headless, "Sign in with code".to_string()),
        ],
        oauth_status_label: if has_oauth_tokens {
            Some("Logout from OAuth".to_string())
        } else {
            None
        },
    }
}

/// Construct an openai-shaped profile-based provider with N profiles.
fn openai_with_profiles(profiles: &[&str]) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        configured: !profiles.is_empty(),
        credential_type: "api_key".to_string(),
        model_count: 4,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: false,
        env_var: None,
        profiles: profiles.iter().map(ToString::to_string).collect(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
    }
}

/// Construct a synthetic API-key-only provider (e.g. gemini) — no OAuth,
/// no profile sub-list. Used to verify the api-key gating path.
fn api_key_provider(id: &str, name: &str) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: name.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: true,
        env_var: Some(format!("{}_API_KEY", id.to_uppercase())),
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
    }
}

fn empty_expanded() -> HashSet<String> {
    HashSet::new()
}

fn expanded_set(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(ToString::to_string).collect()
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 1: Fresh view with collapsed providers yields one NavItem per
//             provider
// ────────────────────────────────────────────────────────────────────────

#[test]
fn fresh_view_with_collapsed_providers_yields_one_nav_item_per_provider() {
    // @step Given a fresh ProviderSettingsView with no expanded providers
    let expanded = empty_expanded();
    // @step And the providers list contains 17 entries in canonical registry order
    let providers: Vec<ProviderDisplayInfo> = (0..17)
        .map(|i| api_key_provider(&format!("p{i}"), &format!("Provider {i}")))
        .collect();
    // @step And the filter is empty
    let filter = "";

    // @step When build_nav_items is called
    let items = build_nav_items(&providers, &expanded, filter);

    // @step Then the result contains exactly 17 NavItems
    assert_eq!(items.len(), 17, "expected one NavItem per provider");
    // @step And every NavItem has kind NavItemKind::Provider { expanded: false }
    for item in &items {
        assert!(
            matches!(item.kind, NavItemKind::Provider { expanded: false }),
            "expected NavItemKind::Provider {{ expanded: false }}, got {:?}",
            item.kind
        );
    }
    // @step And the NavItems appear in canonical registry order
    for (idx, item) in items.iter().enumerate() {
        assert_eq!(item.provider_id, format!("p{idx}"));
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 2: Expanding anthropic without OAuth tokens injects
//             oauth-login and api-key children
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_anthropic_without_oauth_tokens_injects_oauth_login_and_api_key_children() {
    // @step Given a ProviderSettingsView containing the anthropic provider
    let providers = vec![anthropic(false)];
    // @step And the anthropic provider has no OAuth tokens (hasOAuthTokens = false)
    // (encoded in anthropic(false))
    // @step And anthropic is in the expanded set
    let expanded = expanded_set(&["anthropic"]);

    // @step When build_nav_items is called
    let items = build_nav_items(&providers, &expanded, "");

    // @step Then the row immediately after anthropic is NavItemKind::OAuthLogin { method: Browser }
    assert_eq!(
        items.len(),
        4,
        "expected provider + 2 oauth-login + 1 api-key"
    );
    assert!(matches!(
        items[0].kind,
        NavItemKind::Provider { expanded: true }
    ));
    match &items[1].kind {
        NavItemKind::OAuthLogin { method, .. } => {
            assert_eq!(*method, OAuthMethod::Browser, "first child must be Browser");
        }
        other => panic!("expected OAuthLogin, got {other:?}"),
    }
    // @step And the next row is NavItemKind::OAuthLogin { method: Headless }
    match &items[2].kind {
        NavItemKind::OAuthLogin { method, .. } => {
            assert_eq!(*method, OAuthMethod::Headless);
        }
        other => panic!("expected OAuthLogin, got {other:?}"),
    }
    // @step And the next row is NavItemKind::ApiKey
    assert!(matches!(items[3].kind, NavItemKind::ApiKey));
    // @step And no NavItemKind::OAuthStatus row appears in the anthropic child block
    for item in &items {
        assert!(
            !matches!(item.kind, NavItemKind::OAuthStatus { .. }),
            "no oauth-status row when has_oauth_tokens=false"
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 3: Expanding openai with profiles injects profile rows and a
//             trailing add-profile pseudo-row
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_openai_with_profiles_injects_profile_rows_and_add_profile() {
    // @step Given a ProviderSettingsView containing the openai provider
    // @step And openai has 2 profiles named "prof1" and "prof2"
    let providers = vec![openai_with_profiles(&["prof1", "prof2"])];
    // @step And openai is in the expanded set
    let expanded = expanded_set(&["openai"]);

    // @step When build_nav_items is called
    let items = build_nav_items(&providers, &expanded, "");

    // @step Then the rows immediately after openai are NavItemKind::Profile { profile_name: "prof1" } then NavItemKind::Profile { profile_name: "prof2" }
    assert_eq!(
        items.len(),
        4,
        "expected provider + 2 profiles + add-profile"
    );
    assert!(matches!(
        items[0].kind,
        NavItemKind::Provider { expanded: true }
    ));
    match &items[1].kind {
        NavItemKind::Profile { profile_name } => assert_eq!(profile_name, "prof1"),
        other => panic!("expected Profile, got {other:?}"),
    }
    match &items[2].kind {
        NavItemKind::Profile { profile_name } => assert_eq!(profile_name, "prof2"),
        other => panic!("expected Profile, got {other:?}"),
    }
    // @step And the next row is NavItemKind::AddProfile
    assert!(matches!(items[3].kind, NavItemKind::AddProfile));
    // @step And no NavItemKind::ApiKey row appears in the openai child block
    for item in &items {
        assert!(
            !matches!(item.kind, NavItemKind::ApiKey),
            "openai is profile-only, no api-key row"
        );
    }
    // @step And no NavItemKind::OAuthLogin or NavItemKind::OAuthStatus row appears in the openai child block
    for item in &items {
        assert!(
            !matches!(
                item.kind,
                NavItemKind::OAuthLogin { .. } | NavItemKind::OAuthStatus { .. }
            ),
            "openai is not an OAuth provider"
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 4: Expanding anthropic with OAuth tokens prepends oauth-status
//             before oauth-login rows
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_anthropic_with_oauth_tokens_prepends_oauth_status_row() {
    // @step Given a ProviderSettingsView containing the anthropic provider
    // @step And the anthropic provider has OAuth tokens (hasOAuthTokens = true)
    let providers = vec![anthropic(true)];
    // @step And anthropic is in the expanded set
    let expanded = expanded_set(&["anthropic"]);

    // @step When build_nav_items is called
    let items = build_nav_items(&providers, &expanded, "");

    // @step Then the row immediately after anthropic is NavItemKind::OAuthStatus
    assert_eq!(
        items.len(),
        5,
        "expected provider + oauth-status + 2 oauth-login + api-key"
    );
    assert!(matches!(
        items[0].kind,
        NavItemKind::Provider { expanded: true }
    ));
    assert!(
        matches!(items[1].kind, NavItemKind::OAuthStatus { .. }),
        "oauth-status must appear FIRST in the child block when has_oauth_tokens=true"
    );
    // @step And the next two rows are NavItemKind::OAuthLogin { method: Browser } then NavItemKind::OAuthLogin { method: Headless }
    match &items[2].kind {
        NavItemKind::OAuthLogin { method, .. } => assert_eq!(*method, OAuthMethod::Browser),
        other => panic!("expected OAuthLogin(Browser), got {other:?}"),
    }
    match &items[3].kind {
        NavItemKind::OAuthLogin { method, .. } => assert_eq!(*method, OAuthMethod::Headless),
        other => panic!("expected OAuthLogin(Headless), got {other:?}"),
    }
    // @step And the next row is NavItemKind::ApiKey
    assert!(matches!(items[4].kind, NavItemKind::ApiKey));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 5: Filter is parent-anchored — children of filtered-out
//             providers disappear
// ────────────────────────────────────────────────────────────────────────

#[test]
fn filter_is_parent_anchored() {
    // @step Given a ProviderSettingsView with anthropic and openai both expanded
    let providers = vec![anthropic(false), openai_with_profiles(&["prof1", "prof2"])];
    // @step And openai has 2 profiles
    // (encoded above)
    let expanded = expanded_set(&["anthropic", "openai"]);

    // @step When the filter is set to "anth"
    let filter = "anth";

    // @step And build_nav_items is called
    let items = build_nav_items(&providers, &expanded, filter);

    // @step Then the result contains the anthropic NavItem and its child rows
    assert!(
        items.iter().any(|i| i.provider_id == "anthropic"),
        "expected anthropic in result"
    );
    let anthropic_count = items
        .iter()
        .filter(|i| i.provider_id == "anthropic")
        .count();
    assert!(
        anthropic_count >= 2,
        "expected anthropic + at least one child row"
    );
    // @step And the result contains NO openai NavItem
    assert!(
        !items.iter().any(|i| i.provider_id == "openai"),
        "expected NO openai in result (filter is parent-anchored)"
    );
    // @step And the result contains NO child rows belonging to openai (no profile rows, no add-profile row)
    for item in &items {
        assert_ne!(
            item.provider_id, "openai",
            "no openai children should leak through the parent-anchored filter"
        );
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 6: Expansion state survives set_provider_display_infos reload
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expansion_state_survives_set_provider_display_infos() {
    // @step Given a ProviderSettingsView with the anthropic provider expanded
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![anthropic(false)]);
    view.toggle_expansion("anthropic");
    assert!(
        view.expanded.contains("anthropic"),
        "precondition: anthropic should be in expanded set"
    );

    // @step When set_providers is called with a freshly rebuilt providers Vec from disk
    // (using set_provider_display_infos as the equivalent reload path)
    view.set_provider_display_infos(vec![anthropic(false), openai_with_profiles(&[])]);

    // @step And build_nav_items is called
    // (rebuilt internally by set_provider_display_infos; inspect via view.nav_items)
    // @step Then anthropic remains in the expanded set
    assert!(
        view.expanded.contains("anthropic"),
        "expanded set must survive reload"
    );
    // @step And the anthropic NavItem has kind NavItemKind::Provider { expanded: true }
    let anthropic_nav = view
        .nav_items
        .iter()
        .find(|n| n.provider_id == "anthropic" && matches!(n.kind, NavItemKind::Provider { .. }))
        .expect("anthropic NavItem must be present after reload");
    assert!(matches!(
        anthropic_nav.kind,
        NavItemKind::Provider { expanded: true }
    ));
    // @step And anthropic's child rows are present immediately after it
    let anthropic_idx = view
        .nav_items
        .iter()
        .position(|n| n.provider_id == "anthropic")
        .expect("anthropic must be in nav_items");
    let next = view
        .nav_items
        .get(anthropic_idx + 1)
        .expect("expected at least one child row after anthropic");
    assert_eq!(
        next.provider_id, "anthropic",
        "the row immediately after anthropic must be one of its child rows"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 7: Enter on a provider row toggles expansion without mutating
//             selected_index
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_provider_toggles_expansion_without_mutating_selected_index() {
    // @step Given a ProviderSettingsView with selected_index pointing at the anthropic provider row
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![api_key_provider("gemini", "Gemini"), anthropic(false)]);
    view.selected_index = 1; // anthropic
                             // @step And anthropic is currently collapsed (not in the expanded set)
    assert!(!view.expanded.contains("anthropic"));
    let initial_selected = view.selected_index;

    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then anthropic is added to the expanded set
    assert!(
        view.expanded.contains("anthropic"),
        "Enter on a Provider NavItem must toggle expansion"
    );
    // @step And selected_index still points at the anthropic provider row
    assert_eq!(
        view.selected_index, initial_selected,
        "selected_index must not mutate on toggle"
    );
    // @step And the anthropic child rows now appear immediately below selected_index in nav_items
    let anth_idx = view
        .nav_items
        .iter()
        .position(|n| {
            n.provider_id == "anthropic" && matches!(n.kind, NavItemKind::Provider { .. })
        })
        .unwrap();
    let next = view
        .nav_items
        .get(anth_idx + 1)
        .expect("must have a child row");
    assert_eq!(next.provider_id, "anthropic");
    assert!(matches!(out, ProviderSettingsEvent::Consumed));

    // @step When the user presses Enter again
    let initial_selected_2 = view.selected_index;
    let out2 = view.handle_key(key(KeyCode::Enter));
    // @step Then anthropic is removed from the expanded set
    assert!(
        !view.expanded.contains("anthropic"),
        "second Enter must collapse"
    );
    // @step And selected_index still points at the anthropic provider row
    assert_eq!(view.selected_index, initial_selected_2);
    assert!(matches!(out2, ProviderSettingsEvent::Consumed));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario 8: Enter on an api-key child row transitions directly to
//             the EditApiKey state
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_api_key_transitions_to_edit_api_key_state() {
    // @step Given a ProviderSettingsView with anthropic expanded
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![anthropic(false)]);
    view.toggle_expansion("anthropic");
    // @step And selected_index points at the NavItemKind::ApiKey child row under anthropic
    let api_key_idx = view
        .nav_items
        .iter()
        .position(|n| matches!(n.kind, NavItemKind::ApiKey))
        .expect("anthropic expanded must yield an ApiKey row");
    view.selected_index = api_key_idx;

    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then the view's mode becomes ProviderSettingsMode::Detail with provider_id "anthropic" and sub DetailSub::EditApiKey with an empty draft
    match &view.mode {
        ProviderSettingsMode::Detail { provider_id, sub } => {
            assert_eq!(provider_id, "anthropic");
            match sub {
                DetailSub::EditApiKey { draft } => assert!(
                    draft.is_empty(),
                    "draft must start empty when first entering EditApiKey"
                ),
                other => panic!("expected DetailSub::EditApiKey, got {other:?}"),
            }
        }
        other => panic!("expected ProviderSettingsMode::Detail, got {other:?}"),
    }
    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    // @step And the view does not first land on a Summary sub-view — Enter routes directly to the edit form
    // (the assertion above verifies sub == EditApiKey, not Summary)
}
