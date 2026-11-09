//! BUG-432: an offline models.dev catalog and sentinel Anthropic credentials
//! in the current data directory, so creating an anthropic session in a test
//! never depends on the developer's environment or the network.

/// Seed `<data dir>/cache/models.json` and a sentinel ANTHROPIC_API_KEY.
/// Call after the test has pointed the process-global data directory at its
/// own temp directory.
pub fn seed_offline_catalog() -> Result<(), String> {
    let cache = codelet_common::get_data_dir()?.join("cache");
    std::fs::create_dir_all(&cache).map_err(|e| format!("create {}: {e}", cache.display()))?;
    let models = cache.join("models.json");
    std::fs::write(&models, include_str!("prov101_models.json"))
        .map_err(|e| format!("write {}: {e}", models.display()))?;
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-offline-bug432");
    Ok(())
}
