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
use crate::profile_sections::{fspec_user_dir, read_config_value};

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
