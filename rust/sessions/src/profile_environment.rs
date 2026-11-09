//! Profile environment: an openai profile's settings copied into the
//! process environment the OpenAI client reads (PROV-121). BUG-612: every
//! write goes through [`ledger`] so deleting the profile removes it.

pub(crate) mod ledger;

pub fn apply_profile_env_vars(
    provider: &str,
    profile_name: &str,
    model: &str,
) -> Result<(), String> {
    if provider != "openai" {
        return Ok(());
    }
    // PROV-175: an invalid profile config is the error.
    let Some(profile) = crate::profile_sections::load_local_server_profiles()?
        .into_iter()
        .find(|profile| profile.name == profile_name)
    else {
        tracing::warn!(
            provider,
            profile_name,
            model,
            "no stored profile found; leaving OPENAI_* environment unchanged"
        );
        return Ok(());
    };
    let owner = ledger::owner(provider, profile_name);
    if !profile.base_url.is_empty() {
        ledger::write(&owner, "OPENAI_BASE_URL", Some(&profile.base_url));
    }
    if let Some(api_key) = profile.api_key.as_deref().filter(|key| !key.is_empty()) {
        ledger::write(&owner, "OPENAI_API_KEY", Some(api_key));
    }
    // BUG-341: a profile without a window must not keep the previous one's.
    let context_window = profile.context_window.map(|window| window.to_string());
    ledger::write(&owner, "OPENAI_CONTEXT_WINDOW", context_window.as_deref());
    let streaming = profile.streaming.map(|enabled| enabled.to_string());
    ledger::write(&owner, "OPENAI_STREAMING", streaming.as_deref());
    tracing::info!(
        provider,
        profile_name,
        model,
        "applied OpenAI profile environment"
    );
    Ok(())
}
