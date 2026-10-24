use codelet_rpc_types::ProviderInfo;

pub fn build_inherited_profile_sections(providers: &[ProviderInfo]) -> Vec<ProviderInfo> {
    let settings = match crate::provider_settings::list_provider_settings() {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(%error, "Cannot load inherited provider profiles");
            return Vec::new();
        }
    };
    settings
        .into_iter()
        .flat_map(|settings| {
            let provider = providers.iter().find(|provider| {
                crate::provider_settings::canonical_provider_id(&provider.key)
                    == settings.provider_id
            });
            settings.profiles.into_iter().filter_map(move |profile| {
                if settings.provider_id == "openai" && !profile.definition.base_url.is_empty() {
                    return None;
                }
                let provider = provider?;
                Some(crate::profile_sections::build_profile_provider_info(
                    &settings.provider_id,
                    &profile.name,
                    provider.models.clone(),
                    provider.is_unreachable,
                ))
            })
        })
        .collect()
}
