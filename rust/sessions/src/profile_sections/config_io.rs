//! Locating, reading and writing `~/.fspec/fspec-config.json`.

use std::path::Path;

use tracing::debug;

use super::LocalServerProfile;

/// Load every local-server profile from the user's
/// `~/.fspec/fspec-config.json` (`providers.openai.profiles`). Profiles are
/// only supported for the `openai` provider. PROV-175: a missing file or a
/// missing profiles section means no profiles; an unresolvable user
/// directory, an unreadable or malformed file and a profile that fails to
/// parse are errors naming the profile and field.
pub fn load_local_server_profiles() -> Result<Vec<LocalServerProfile>, String> {
    let directory = codelet_common::user_dir::fspec_user_dir()?;
    load_local_server_profiles_from(&directory.join("fspec-config.json"))
}

/// Path-injectable core of [`load_local_server_profiles`].
pub(super) fn load_local_server_profiles_from(
    config_path: &Path,
) -> Result<Vec<LocalServerProfile>, String> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot read {}: {error}", config_path.display())),
    };
    let root = serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|error| format!("{} is not valid JSON: {error}", config_path.display()))?;
    let Some(profiles) = root
        .get("providers")
        .and_then(|p| p.get("openai"))
        .and_then(|o| o.get("profiles"))
    else {
        return Ok(Vec::new());
    };
    let profiles = profiles.as_object().ok_or_else(|| {
        format!(
            "providers.openai.profiles in {} is not an object",
            config_path.display()
        )
    })?;
    let mut out = Vec::with_capacity(profiles.len());
    for (name, value) in profiles {
        let mut profile: LocalServerProfile =
            serde_path_to_error::deserialize(value).map_err(|error| {
                format!(
                    "profile '{name}' in {}: {}: {}",
                    config_path.display(),
                    error.path(),
                    error.inner()
                )
            })?;
        profile.name = name.clone();
        out.push(profile);
    }
    // Deterministic ordering (sort by name for stable output).
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Read the user config at `config_path` (BUG-461): a missing file is an
/// empty object; an unreadable, malformed or non-object file is an error
/// naming the file, so no caller ignores it or writes over it.
pub(crate) fn read_config_value(config_path: &Path) -> Result<serde_json::Value, String> {
    let value = codelet_common::fspec_config::load_config_object(config_path)?;
    debug!(path = %config_path.display(), "config read: parsed OK");
    Ok(value)
}
