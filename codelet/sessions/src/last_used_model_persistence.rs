//! PROV-120 — read the persisted startup model string (TS-parity source of
//! truth) with legacy back-compat.
//!
//! Feature: spec/features/startup-model-initialization-first-available.feature
//!
//! Ports `loadPersistedModelString()` from the TypeScript
//! `src/tui/services/modelInitializationService.ts`: the persisted model is
//! `tui.lastUsedModel` in the user `~/.fspec/fspec-config.json`, which is the
//! single source of truth in the TS reference.
//!
//! Reconciliation (PROV-119 -> PROV-120): when `fspec-config.json` has no
//! `tui.lastUsedModel`, fall back ONCE to the legacy
//! `~/.fspec/default-model.json` (`model` key) for continuity; new writes go to
//! `tui.lastUsedModel`.

use std::path::Path;

use crate::default_model_persistence::load_default_model_with_dir;
use crate::profile_sections::{fspec_user_dir, read_config_value, write_config_value};

/// Path-injectable core: read the persisted model string from the given user
/// directory (the `~/.fspec` equivalent). Reads `fspec-config.json`
/// `tui.lastUsedModel`; on absence falls back to the legacy
/// `default-model.json` `model` key. Returns `None` when neither is present.
#[must_use]
pub fn load_persisted_model_string_from(user_dir: &Path) -> Option<String> {
    // Primary source of truth (TS parity): fspec-config.json -> tui.lastUsedModel.
    if let Some(root) = read_config_value(&user_dir.join("fspec-config.json")) {
        if let Some(model) = root
            .get("tui")
            .and_then(|tui| tui.get("lastUsedModel"))
            .and_then(|m| m.as_str())
            .map(str::trim)
            .filter(|m| !m.is_empty())
        {
            return Some(model.to_string());
        }
    }
    // Legacy back-compat (PROV-119 -> PROV-120): default-model.json -> model.
    load_default_model_with_dir(user_dir)
}

/// Convenience: read using the env-resolved user directory
/// (`FSPEC_USER_DIR` / `HOME`-based `~/.fspec`). Returns `None` when the
/// directory cannot be resolved or neither store records a model.
#[must_use]
pub fn load_persisted_model_string() -> Option<String> {
    let user_dir = fspec_user_dir()?;
    load_persisted_model_string_from(&user_dir)
}

/// PROV-122 — path-injectable core of the WRITE side: persist `model` as
/// `tui.lastUsedModel` in `<user_dir>/fspec-config.json` with a key-preserving
/// read-merge-write. Mirrors the TS `modelSelectionService.selectModel`
/// read-merge-write and the `save_custom_model_at` pattern (only the target key
/// changes; `preserve_order` serde keeps all other keys in place).
///
/// Behavior:
/// * An empty / whitespace-only `model` is a no-op success — the PROV-101
///   invariant (never persist an empty selection, never write a hardcoded
///   fallback) is preserved.
/// * A missing / malformed config starts from `{}` so a fresh install still
///   gets a file.
/// * The `tui` object is created when absent; any existing `tui.*` fields are
///   left untouched.
///
/// Returns an `Err` (string) on a serialize / write failure so the caller can
/// log it; callers treat persistence as best-effort and non-fatal.
pub fn save_persisted_model_string_to(user_dir: &Path, model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Ok(());
    }
    let config_path = user_dir.join("fspec-config.json");

    // Read-merge-write: start from the existing config (preserving every key)
    // or an empty object when the file is missing / malformed.
    let mut root = read_config_value(&config_path).unwrap_or_else(|| serde_json::json!({}));

    // Ensure `root` is an object so we can attach `tui` to it; a non-object
    // top-level (corrupt config) is replaced with a fresh object rather than
    // silently dropping the new selection.
    if !root.is_object() {
        root = serde_json::json!({});
    }

    let obj = root
        .as_object_mut()
        .ok_or_else(|| "fspec-config.json root is not a JSON object".to_string())?;
    let tui = obj.entry("tui").or_insert_with(|| serde_json::json!({}));
    let tui_obj = tui
        .as_object_mut()
        .ok_or_else(|| "fspec-config.json `tui` is not a JSON object".to_string())?;
    tui_obj.insert(
        "lastUsedModel".to_string(),
        serde_json::Value::String(model.to_string()),
    );

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create user directory: {e}"))?;
    }
    write_config_value(&config_path, &root)
        .map_err(|e| format!("Failed to write fspec-config.json: {e}"))
}

/// Convenience: persist `model` to `tui.lastUsedModel` using the env-resolved
/// user directory (`FSPEC_USER_DIR` / `HOME`-based `~/.fspec`). Returns an
/// `Err` when the directory cannot be resolved or the write fails; callers log
/// it and treat persistence as best-effort.
pub fn save_persisted_model_string(model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Ok(());
    }
    let user_dir =
        fspec_user_dir().ok_or_else(|| "Could not resolve fspec user directory".to_string())?;
    save_persisted_model_string_to(&user_dir, model)
}
