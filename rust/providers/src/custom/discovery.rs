//! Discovery of custom provider configs from global and project-local
//! directories (PROV-062).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::config::ProviderConfig;
use super::error::CustomProviderError;

/// Default subdirectory name inside `~/.fspec/` that holds custom provider
/// configs.
const PROVIDERS_SUBDIR: &str = "providers";

/// Default subdirectory name for credentials.
const CREDENTIALS_SUBDIR: &str = "credentials";

/// Discover all custom provider configs from global and project-local
/// directories.
///
/// Search order:
///   1. `~/.fspec/providers/*.json` (or `FSPEC_HOME`-derived base)
///   2. `.fspec/providers/*.json` (project-local, CWD-relative)
///
/// Project-local configs override global configs with the same `name`.
/// Returns an empty `Vec` if neither directory exists.
pub fn discover_provider_configs() -> Result<Vec<ProviderConfig>, CustomProviderError> {
    let mut configs: HashMap<String, ProviderConfig> = HashMap::new();

    let global_dir = get_global_providers_dir();
    load_configs_from_dir(&global_dir, &mut configs)?;

    let local_dir = PathBuf::from(".fspec").join(PROVIDERS_SUBDIR);
    load_configs_from_dir(&local_dir, &mut configs)?;

    Ok(configs.into_values().collect())
}

/// Compute the global providers directory.
///
/// Honours `FSPEC_HOME` (which points at the credentials directory — the
/// providers directory is its sibling). Falls back to
/// `$HOME/.fspec/providers`.
fn get_global_providers_dir() -> PathBuf {
    if let Ok(fspec_home) = std::env::var("FSPEC_HOME") {
        let credentials_dir = PathBuf::from(&fspec_home);
        // `FSPEC_HOME` traditionally points at <base>/credentials. The
        // providers dir is a sibling — use the parent when available.
        if credentials_dir
            .file_name()
            .map(|n| n == CREDENTIALS_SUBDIR)
            .unwrap_or(false)
        {
            if let Some(parent) = credentials_dir.parent() {
                return parent.join(PROVIDERS_SUBDIR);
            }
        }
        // Fallback: treat FSPEC_HOME itself as the base.
        credentials_dir.join(PROVIDERS_SUBDIR)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        PathBuf::from(home).join(".fspec").join(PROVIDERS_SUBDIR)
    }
}

/// Scan `dir` for `*.json` files and insert each parsed
/// [`ProviderConfig`] into `configs`, overwriting any existing entry with
/// the same `name`. Silently returns `Ok(())` when `dir` does not exist.
fn load_configs_from_dir(
    dir: &Path,
    configs: &mut HashMap<String, ProviderConfig>,
) -> Result<(), CustomProviderError> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|source| CustomProviderError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| CustomProviderError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            match ProviderConfig::from_file(&path) {
                Ok(cfg) => {
                    configs.insert(cfg.name.clone(), cfg);
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "skipping invalid provider config"
                    );
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}
