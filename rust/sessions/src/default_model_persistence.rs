//! PROV-119 — persist the user-selected default model across process restarts.
//!
//! Feature: spec/features/default-model-persists-across-restarts.feature
//!
//! The default model chosen in `/model` is held in-memory on `SessionManager`
//! (`RwLock<Option<String>>`). Before PROV-119 it was never written to disk, so
//! every fresh process started with `default_model = None` and the first
//! `create_session` was declined (PROV-101: no silent anthropic fallback). This
//! module persists the choice to `<data_dir>/default-model.json` — the same
//! user-scoped `~/.fspec` store (resolved via [`codelet_common::get_data_dir`])
//! that the provider credentials writer uses — and loads it at `SessionManager`
//! construction so a fresh process is pre-populated.
//!
//! Invariants:
//!   * Empty / whitespace-only model strings are NEVER persisted (PROV-101).
//!   * Persistence is best-effort: the `_with_dir` core surfaces an `Err` that
//!     the `SessionManager` caller logs and swallows (non-fatal).
//!   * A missing / malformed / keyless file loads as `None` (graceful
//!     degradation — unchanged pre-PROV-119 behaviour).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name (under the data directory) holding the persisted default model.
const DEFAULT_MODEL_FILE: &str = "default-model.json";

/// On-disk shape: a single optional `model` string. `camelCase`-free because
/// it is a one-field document; the field name doubles as the JSON key.
#[derive(Debug, Default, Serialize, Deserialize)]
struct DefaultModelFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
}

/// Path to the default-model file under a given data directory.
fn default_model_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DEFAULT_MODEL_FILE)
}

/// Persist a non-empty default `model` under `data_dir`.
///
/// An empty / whitespace-only `model` is a no-op success (PROV-101: an empty
/// string must never become a stored default). Creates the data directory on
/// demand. Path-injectable core used by both the global convenience wrapper and
/// the tests.
pub fn save_default_model_with_dir(data_dir: &Path, model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Ok(());
    }
    let path = default_model_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create data directory: {e}"))?;
    }
    let file = DefaultModelFile {
        model: Some(model.to_string()),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("Failed to serialize default model: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write default model file: {e}"))
}

/// Load a persisted default model from `data_dir`.
///
/// BUG-463: a missing file, or an absent / empty `model` key, is `None`; an
/// unreadable or malformed file is an error naming it.
pub fn load_default_model_with_dir(data_dir: &Path) -> Result<Option<String>, String> {
    let path = default_model_path(data_dir);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let file: DefaultModelFile = serde_json::from_str(&content).map_err(|error| {
        format!(
            "{} is not a valid default model file: {error}",
            path.display()
        )
    })?;
    Ok(file.model.filter(|m| !m.trim().is_empty()))
}

/// Convenience: persist using the globally configured data directory
/// ([`codelet_common::get_data_dir`]). Surfaces the data-dir / write error.
pub fn save_default_model(model: &str) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    save_default_model_with_dir(&data_dir, model)
}

/// Convenience: load using the globally configured data directory. BUG-463:
/// an unresolvable data directory is an error, not "no default".
pub fn load_default_model() -> Result<Option<String>, String> {
    let data_dir = codelet_common::get_data_dir()?;
    load_default_model_with_dir(&data_dir)
}

/// BUG-613: forget the persisted default model (a missing file is already
/// forgotten). Path-injectable core.
pub fn clear_default_model_with_dir(data_dir: &Path) -> Result<(), String> {
    let path = default_model_path(data_dir);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot remove {}: {error}", path.display())),
    }
}

/// BUG-613: [`clear_default_model_with_dir`] for the configured data directory.
pub fn clear_default_model() -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    clear_default_model_with_dir(&data_dir)
}
