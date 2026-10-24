//! Feature: spec/features/provider-settings-parity-and-automatic-model-limits.feature

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::{NavItemKind, ProviderDisplayInfo};
use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use codelet_rpc_types::{ProfileDefinition, ProviderProfileConfig, ProviderSettingsConfig};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn definition() -> ProfileDefinition {
    ProfileDefinition {
        base_url: String::new(),
        api_key: String::new(),
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

fn view_for(provider_ids: &[&str]) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(
        provider_ids
            .iter()
            .map(|provider_id| ProviderDisplayInfo {
                id: (*provider_id).to_string(),
                name: (*provider_id).to_string(),
                configured: true,
                credential_type: "oauth".to_string(),
                is_oauth_provider: matches!(*provider_id, "codex" | "anthropic"),
                has_oauth_tokens: true,
                ..ProviderDisplayInfo::default()
            })
            .collect(),
    );
    view
}

fn open_row(view: &mut ProviderSettingsView, provider_id: &str, kind: &NavItemKind) -> TestResult {
    let Some(index) = view
        .nav_items
        .iter()
        .position(|item| item.provider_id == provider_id && &item.kind == kind)
    else {
        return Err(format!("missing {provider_id} row: {kind:?}").into());
    };
    view.selected_index = index;
    view.handle_key(key(KeyCode::Enter));
    Ok(())
}

fn rendered(view: &mut ProviderSettingsView) -> String {
    let area = Rect::new(0, 0, 150, 30);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn scenario_every_provider_exposes_settings_and_named_profiles() -> TestResult {
    // @step Given built-in and custom providers are shown in provider settings
    let providers = ["openai", "codex", "anthropic", "gemini", "private-facade"];
    let mut view = view_for(&providers);

    // @step When each provider is expanded
    for provider_id in providers {
        view.toggle_expansion(provider_id);
    }

    // @step Then each provider offers Settings and provider-scoped profile creation
    for provider_id in providers {
        assert!(view.nav_items.iter().any(|item| {
            item.provider_id == provider_id && matches!(item.kind, NavItemKind::Settings)
        }));
        open_row(&mut view, provider_id, &NavItemKind::AddProfile)?;
        assert!(matches!(
            &view.mode,
            ProviderSettingsMode::CreateProfile { provider_id: actual, .. } if actual == provider_id
        ));
        view.handle_key(key(KeyCode::Esc));
    }
    Ok(())
}

#[test]
fn scenario_oauth_profiles_inherit_authentication() -> TestResult {
    // @step Given a Codex provider authenticated with OAuth
    let mut view = view_for(&["codex"]);
    view.toggle_expansion("codex");
    open_row(&mut view, "codex", &NavItemKind::AddProfile)?;

    // @step When a named runtime profile is saved without connection fields
    for character in "careful".chars() {
        view.handle_key(key(KeyCode::Char(character)));
    }
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the profile saves successfully without replacing OAuth authentication
    let ProviderSettingsEvent::Emit(action) = event else {
        return Err("inherited OAuth profile did not emit a save".into());
    };
    let Action::SaveProfile {
        provider_id,
        profile_name,
        definition,
        ..
    } = *action
    else {
        return Err("profile save emitted a credential-changing action".into());
    };
    assert_eq!(provider_id, "codex");
    assert_eq!(profile_name, "careful");
    assert!(definition.base_url.is_empty());
    assert!(definition.api_key.is_empty());
    assert!(view
        .display_providers
        .iter()
        .any(|provider| { provider.id == "codex" && provider.has_oauth_tokens }));
    Ok(())
}

#[test]
fn scenario_automatic_token_limits_remain_independent_in_the_editor() -> TestResult {
    // @step Given a provider settings form with an explicit context limit and automatic output limit
    let mut view = view_for(&["codex"]);
    let mut settings = definition();
    settings.context_window = Some(128000);
    view.set_provider_settings_configs(vec![ProviderSettingsConfig {
        provider_id: "codex".to_string(),
        settings,
        profiles: Vec::new(),
    }]);
    view.toggle_expansion("codex");
    open_row(&mut view, "codex", &NavItemKind::Settings)?;
    let ProviderSettingsMode::EditSettings { form, .. } = &mut view.mode else {
        return Err("Settings did not open the shared form".into());
    };
    assert_eq!(form.context_window, "128000");
    assert!(form.max_output_tokens.is_empty());

    // @step When the context limit is cleared and the form is saved
    form.field_index = 2;
    for _ in 0..6 {
        view.handle_key(key(KeyCode::Backspace));
    }
    let display = rendered(&mut view);
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then both token overrides are absent and the editor labels automatic limits as model-specific
    let ProviderSettingsEvent::Emit(action) = event else {
        return Err("Settings form did not emit a save".into());
    };
    let Action::SaveProviderSettings {
        provider_id,
        definition,
    } = *action
    else {
        return Err("Settings form emitted the wrong save action".into());
    };
    assert_eq!(provider_id, "codex");
    assert_eq!(definition.context_window, None);
    assert_eq!(definition.max_output_tokens, None);
    assert!(display.matches("Automatic").count() >= 2, "{display}");
    assert!(display.contains("model-specific"), "{display}");
    Ok(())
}

#[test]
fn scenario_invalid_token_overrides_cannot_silently_become_automatic() {
    // @step Given a provider settings form with an invalid token limit
    for field_index in [2, 3] {
        for invalid in ["not-a-number", "0", "-1", "4294967296", "1.5"] {
            let mut form = ProfileForm::new_create();
            form.name = "careful".to_string();
            form.api_key = "existing-key".to_string();
            form.is_editing_name = false;
            form.field_index = field_index;
            if field_index == 2 {
                form.context_window = invalid.to_string();
            } else {
                form.max_output_tokens = invalid.to_string();
            }
            let mut view = ProviderSettingsView::new();
            view.mode = ProviderSettingsMode::EditProfile {
                provider_id: "openai".to_string(),
                profile_name: "careful".to_string(),
                form,
            };

            // @step When the user attempts to save
            let event = view.handle_key(key(KeyCode::Enter));

            // @step Then the form shows a validation error and emits no save action
            assert!(matches!(event, ProviderSettingsEvent::Consumed));
            assert!(matches!(
                view.mode,
                ProviderSettingsMode::EditProfile { .. }
            ));
            assert!(!view.status.is_empty(), "missing error for {invalid}");
        }
    }
}

#[test]
fn scenario_same_named_profiles_remain_isolated_in_the_editor() -> TestResult {
    // @step Given Codex and Claude each have a profile named careful with different settings
    let mut view = view_for(&["codex", "anthropic"]);
    let mut configs = Vec::new();
    for (provider_id, context, budget) in [("codex", 200000, 12), ("anthropic", 1000000, 37)] {
        let mut profile = definition();
        profile.context_window = Some(context);
        profile.auto_continue = Some(budget);
        configs.push(ProviderSettingsConfig {
            provider_id: provider_id.to_string(),
            settings: definition(),
            profiles: vec![ProviderProfileConfig {
                name: "careful".to_string(),
                definition: profile,
            }],
        });
    }
    view.set_provider_settings_configs(configs);
    view.toggle_expansion("codex");
    view.toggle_expansion("anthropic");

    // @step When each profile is opened from backend-loaded provider settings
    let mut opened = Vec::new();
    for provider_id in ["codex", "anthropic"] {
        open_row(
            &mut view,
            provider_id,
            &NavItemKind::Profile {
                profile_name: "careful".to_string(),
            },
        )?;
        let ProviderSettingsMode::EditProfile {
            provider_id, form, ..
        } = &view.mode
        else {
            return Err("backend-loaded profile did not open its editor".into());
        };
        opened.push((
            provider_id.clone(),
            form.context_window.clone(),
            form.auto_continue.clone(),
        ));
        view.handle_key(key(KeyCode::Esc));
    }

    // @step Then each editor shows only its own provider settings
    assert_eq!(
        opened,
        vec![
            ("codex".to_string(), "200000".to_string(), "12".to_string()),
            (
                "anthropic".to_string(),
                "1000000".to_string(),
                "37".to_string()
            ),
        ]
    );
    Ok(())
}
