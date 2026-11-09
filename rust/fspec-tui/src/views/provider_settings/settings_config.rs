use codelet_rpc_types::{ProfileDefinition, ProviderCredentialInfo, ProviderSettingsConfig};

use super::ProviderSettingsView;

impl ProviderSettingsView {
    /// Fold the stored settings and profiles into the tree, keeping the
    /// cursor's row. BUG-575: the Settings form opens only after this.
    pub fn set_provider_settings_configs(&mut self, configs: Vec<ProviderSettingsConfig>) {
        let focused = self.focused_nav_item().cloned();
        self.store_settings_configs(configs);
        self.mark_settings_loaded();
        self.relabel_profiles();
        self.rebuild_nav_items();
        self.restore_focus(focused);
    }

    /// Rebuild the providers from `list`, with the stored profiles.
    pub(super) fn fold_provider_credentials(&mut self, list: Vec<ProviderCredentialInfo>) {
        self.display_providers = super::projection::project_display_infos(&list, &[]);
        self.providers = list;
        self.relabel_profiles();
        self.rebuild_nav_items();
    }

    /// Replace the stored settings and profile definitions.
    pub(super) fn store_settings_configs(&mut self, configs: Vec<ProviderSettingsConfig>) {
        self.provider_configs.clear();
        self.profile_configs.clear();
        for config in configs {
            for profile in config.profiles {
                self.profile_configs.insert(
                    (config.provider_id.clone(), profile.name),
                    profile.definition,
                );
            }
            self.provider_configs
                .insert(config.provider_id, config.settings);
        }
    }

    /// Give every provider the labels of its stored profiles, sorted.
    pub(super) fn relabel_profiles(&mut self) {
        for provider in &mut self.display_providers {
            provider.profiles = self
                .profile_configs
                .iter()
                .filter(|((provider_id, _), _)| provider_id == &provider.id)
                .map(|((provider_id, name), definition)| {
                    profile_label(provider_id, name, definition)
                })
                .collect();
            provider.profiles.sort();
        }
    }
}

/// BUG-547: separates a profile row's label from the limits it is missing.
pub(super) const MISSING_LIMITS_SEPARATOR: &str = " — missing ";

/// A profile row's label: `"{name} → {baseUrl}"` (or `"{name}"`), followed
/// for a local-server (openai) profile by the limits it does not declare
/// (BUG-547), e.g. `" — missing max output tokens"`.
fn profile_label(provider_id: &str, name: &str, definition: &ProfileDefinition) -> String {
    let mut label = if definition.base_url.is_empty() {
        name.to_string()
    } else {
        format!("{name} → {}", definition.base_url)
    };
    if provider_id == "openai" {
        let missing: Vec<&str> = [
            (definition.context_window.is_none(), "context window"),
            (definition.max_output_tokens.is_none(), "max output tokens"),
        ]
        .into_iter()
        .filter_map(|(missing, limit)| missing.then_some(limit))
        .collect();
        if !missing.is_empty() {
            label.push_str(MISSING_LIMITS_SEPARATOR);
            label.push_str(&missing.join(", "));
        }
    }
    label
}
