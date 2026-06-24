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
/// A missing / malformed file, or an absent / empty `model` key, yields `None`
/// (graceful degradation). Path-injectable core used by the global convenience
/// wrapper and the tests.
#[must_use]
pub fn load_default_model_with_dir(data_dir: &Path) -> Option<String> {
    let path = default_model_path(data_dir);
    let content = std::fs::read_to_string(&path).ok()?;
    let file: DefaultModelFile = serde_json::from_str(&content).ok()?;
    file.model.filter(|m| !m.trim().is_empty())
}

/// Convenience: persist using the globally configured data directory
/// ([`codelet_common::get_data_dir`]). Surfaces the data-dir / write error so
/// the `SessionManager` caller can log it; the caller treats it as non-fatal.
pub fn save_default_model(model: &str) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    save_default_model_with_dir(&data_dir, model)
}

/// Convenience: load using the globally configured data directory. Returns
/// `None` when the data directory is uninitialized or no default is persisted,
/// so a `SessionManager` constructed before startup wiring degrades gracefully.
#[must_use]
pub fn load_default_model() -> Option<String> {
    let data_dir = codelet_common::get_data_dir().ok()?;
    load_default_model_with_dir(&data_dir)
}
