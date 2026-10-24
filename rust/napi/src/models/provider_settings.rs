use codelet_rpc_types::{ProfileDefinition, ProviderSettingsConfig};
use napi::{Error, Result};
use napi_derive::napi;

#[napi]
pub async fn list_provider_settings() -> Result<Vec<ProviderSettingsConfig>> {
    codelet_sessions::provider_settings::list_provider_settings()
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub async fn save_provider_settings(
    provider_id: String,
    definition: ProfileDefinition,
) -> Result<()> {
    codelet_sessions::provider_settings::save_provider_settings(&provider_id, &definition)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub async fn rename_profile(
    provider_id: String,
    old_name: String,
    new_name: String,
    definition: ProfileDefinition,
) -> Result<()> {
    let definition = codelet_sessions::conversions::profile_def_from_wire(&definition);
    codelet_sessions::profile_persistence::rename_profile(
        &provider_id,
        &old_name,
        &new_name,
        &definition,
    )
    .map_err(|error| Error::from_reason(error.to_string()))
}
