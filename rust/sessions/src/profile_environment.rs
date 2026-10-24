pub fn apply_profile_env_vars(
    provider: &str,
    profile_name: &str,
    model: &str,
) -> Result<(), String> {
    if provider != "openai" {
        return Ok(());
    }
    let Some(profile) = crate::profile_sections::load_local_server_profiles()
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
    if !profile.base_url.is_empty() {
        std::env::set_var("OPENAI_BASE_URL", &profile.base_url);
    }
    if let Some(api_key) = profile.api_key.as_deref().filter(|key| !key.is_empty()) {
        std::env::set_var("OPENAI_API_KEY", api_key);
    }
    if let Some(context_window) = profile.context_window {
        std::env::set_var("OPENAI_CONTEXT_WINDOW", context_window.to_string());
    }
    match profile.streaming {
        Some(enabled) => std::env::set_var("OPENAI_STREAMING", enabled.to_string()),
        None => std::env::remove_var("OPENAI_STREAMING"),
    }
    tracing::info!(
        provider,
        profile_name,
        model,
        "applied OpenAI profile environment"
    );
    Ok(())
}
