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
use codelet_common::fspec_config::update_config_file;
use codelet_common::user_dir::fspec_user_dir;

use crate::profile_sections::read_config_value;

/// Path-injectable core: read the persisted model string from the given user
/// directory (the `~/.fspec` equivalent). Reads `fspec-config.json`
/// `tui.lastUsedModel`; on absence falls back to the legacy
/// `default-model.json` `model` key. Returns `None` when neither is present.
pub fn load_persisted_model_string_from(user_dir: &Path) -> Result<Option<String>, String> {
    // Primary source of truth (TS parity): fspec-config.json -> tui.lastUsedModel.
    // BUG-461: an unreadable or malformed config is an error naming it.
    let root = read_config_value(&user_dir.join("fspec-config.json"))?;
    if let Some(model) = root
        .get("tui")
        .and_then(|tui| tui.get("lastUsedModel"))
        .and_then(|m| m.as_str())
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        return Ok(Some(model.to_string()));
    }
    // Legacy back-compat (PROV-119 -> PROV-120): default-model.json -> model
    // (BUG-463: its read failures are errors too).
    load_default_model_with_dir(user_dir)
}

/// Convenience: read using the env-resolved user directory
/// (`FSPEC_USER_DIR` / `HOME`-based `~/.fspec`). An unresolvable directory
/// is an error; `None` when neither store records a model.
pub fn load_persisted_model_string() -> Result<Option<String>, String> {
    load_persisted_model_string_from(&fspec_user_dir()?)
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
    // BUG-461: a config that cannot be read is never replaced. BUG-640:
    // the read and the write happen under the config directory's lock.
    update_config_file(&user_dir.join("fspec-config.json"), |root| {
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
        Ok::<_, String>(())
    })
}

/// Convenience: persist `model` to `tui.lastUsedModel` using the env-resolved
/// user directory (`FSPEC_USER_DIR` / `HOME`-based `~/.fspec`). Returns an
/// `Err` when the directory cannot be resolved or the write fails; callers log
/// it and treat persistence as best-effort.
pub fn save_persisted_model_string(model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Ok(());
    }
    save_persisted_model_string_to(&fspec_user_dir()?, model)
}

/// BUG-613: remove `tui.lastUsedModel` from the user config when it is
/// `model`; any other value, and a config without it, are left alone.
pub fn forget_persisted_model_string(model: &str) -> Result<(), String> {
    update_config_file(&fspec_user_dir()?.join("fspec-config.json"), |root| {
        if let Some(tui) = root
            .get_mut("tui")
            .and_then(serde_json::Value::as_object_mut)
            .filter(|tui| {
                tui.get("lastUsedModel").and_then(serde_json::Value::as_str) == Some(model)
            })
        {
            tui.remove("lastUsedModel");
        }
        Ok::<_, String>(())
    })
}
