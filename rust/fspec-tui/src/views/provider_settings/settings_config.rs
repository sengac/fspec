use codelet_rpc_types::{ProviderCredentialInfo, ProviderSettingsConfig};

use super::ProviderSettingsView;

impl ProviderSettingsView {
    pub(crate) fn apply_provider_credentials(&mut self, list: Vec<ProviderCredentialInfo>) {
        let focused = self.focused_nav_item().cloned();
        let mut display = super::projection::project_display_infos(&list, &[]);
        for provider in &mut display {
            provider.profiles = self
                .profile_configs
                .iter()
                .filter(|((provider_id, _), _)| provider_id == &provider.id)
                .map(|((_, name), definition)| {
                    if definition.base_url.is_empty() {
                        name.clone()
                    } else {
                        format!("{name} → {}", definition.base_url)
                    }
                })
                .collect();
            provider.profiles.sort();
        }
        self.set_providers(list);
        self.set_provider_display_infos(display);
        if let Some(index) =
            focused.and_then(|focused| self.nav_items.iter().position(|item| item == &focused))
        {
            self.selected_index = index;
        }
    }

    pub fn set_provider_settings_configs(&mut self, configs: Vec<ProviderSettingsConfig>) {
        let focused = self.focused_nav_item().cloned();
        self.provider_configs.clear();
        self.profile_configs.clear();
        for provider in &mut self.display_providers {
            provider.profiles.clear();
        }
        for config in configs {
            let mut labels = Vec::new();
            for profile in config.profiles {
                labels.push(if profile.definition.base_url.is_empty() {
                    profile.name.clone()
                } else {
                    format!("{} → {}", profile.name, profile.definition.base_url)
                });
                self.profile_configs.insert(
                    (config.provider_id.clone(), profile.name),
                    profile.definition,
                );
            }
            labels.sort();
            if let Some(provider) = self
                .display_providers
                .iter_mut()
                .find(|provider| provider.id == config.provider_id)
            {
                provider.profiles = labels;
            }
            self.provider_configs
                .insert(config.provider_id, config.settings);
        }
        self.rebuild_nav_items();
        if let Some(index) =
            focused.and_then(|focused| self.nav_items.iter().position(|item| item == &focused))
        {
            self.selected_index = index;
        }
        self.apply_pending_navigate();
    }
}
