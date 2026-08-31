//! MUX-001 — persist the TUI mux grid config in the shared CONFIG-008
//! `fspec-config.json` under `tui.mux`.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Mirrors `default_thinking_level_persistence.rs` (TUI-002/TUI-092):
//! the mux config (orientation, splits, pane list, focused pane,
//! enabled flag) is stored in the **shared `fspec-config.json`** under
//! the nested key `tui.mux` (a JSON object), NOT a dedicated file.
//! This honours the project-scope override via CONFIG-008's deep merge
//! and interoperates with the TS `fspec` CLI sibling keys
//! (`tui.lastUsedModel`, `tui.defaultThinkingLevel`).
//!
//! The core is `serde_json::Value`-based because the typed `MuxConfig`
//! lives in `codelet-fspec-tui` (which depends on `codelet-sessions`,
//! not the reverse); the TUI's `MuxState` does the Value ↔ MuxConfig
//! serde round-trip.
//!
//! Save is a read-modify-write to USER scope that preserves sibling
//! keys; load relies on `load_config_with_dirs` so a project
//! `<cwd>/spec/fspec-config.json` overrides the user value.
//!
//! Invariants:
//!   * A missing / malformed config loads as `None` (the caller falls
//!     back to the default preset — load is infallible).
//!   * Persistence is best-effort: the `_with_dirs` core surfaces an
//!     `Err` that the caller logs and swallows (non-fatal).

use std::path::Path;

use serde_json::{Map, Value};

use codelet_common::fspec_config::{
    load_config_with_dirs, write_config_with_dirs, ConfigScope,
};

/// Nested config keys holding the persisted mux config: `tui.mux`.
const TUI_KEY: &str = "tui";
const MUX_KEY: &str = "mux";

/// Persist `config` (a JSON object) under `tui.mux` in the USER-scope
/// shared config, preserving any existing sibling keys
/// (read-modify-write).
///
/// Path-injectable core used by both the global convenience wrapper and
/// tests. `data_dir` selects the user `fspec-config.json`; `cwd` is
/// required so the read half loads the deep-merged view (its write half
/// always targets USER).
pub fn save_mux_config_with_dirs(
    data_dir: &Path,
    cwd: &Path,
    config: &Value,
) -> Result<(), String> {
    // Read the existing merged config; an unreadable / invalid file
    // degrades to an empty object rather than aborting the save.
    let mut root =
        load_config_with_dirs(data_dir, cwd).unwrap_or_else(|_| Value::Object(Map::new()));

    if !root.is_object() {
        root = Value::Object(Map::new());
    }

    let root_map = root
        .as_object_mut()
        .ok_or_else(|| "config root is not a JSON object".to_string())?;
    let tui = root_map.entry(TUI_KEY).or_insert_with(|| Value::Object(Map::new()));
    if !tui.is_object() {
        *tui = Value::Object(Map::new());
    }
    let tui_map = tui
        .as_object_mut()
        .ok_or_else(|| "config tui section is not a JSON object".to_string())?;
    tui_map.insert(MUX_KEY.to_string(), config.clone());

    match write_config_with_dirs(ConfigScope::User, &root, data_dir, cwd) {
        Ok(()) => {
            tracing::info!(
                data_dir = %data_dir.display(),
                cwd = %cwd.display(),
                "save_mux_config: persisted tui.mux"
            );
            Ok(())
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                data_dir = %data_dir.display(),
                cwd = %cwd.display(),
                "save_mux_config: failed to persist (non-fatal)"
            );
            Err(e)
        }
    }
}

/// Load the persisted mux config from the shared config.
///
/// Reads `tui.mux` from the deep-merged (project-over-user) config; a
/// missing / malformed config, or a non-object value, yields `None`
/// (graceful degradation). Path-injectable core.
#[must_use]
pub fn load_mux_config_with_dirs(data_dir: &Path, cwd: &Path) -> Option<Value> {
    let Ok(config) = load_config_with_dirs(data_dir, cwd) else {
        tracing::debug!(
            data_dir = %data_dir.display(),
            cwd = %cwd.display(),
            "load_mux_config: config unreadable, using default preset"
        );
        return None;
    };
    let value = &config[TUI_KEY][MUX_KEY];
    if value.is_null() {
        return None;
    }
    if !value.is_object() {
        tracing::debug!(
            value = %value,
            "load_mux_config: tui.mux is not an object, using default preset"
        );
        return None;
    }
    tracing::debug!(
        data_dir = %data_dir.display(),
        cwd = %cwd.display(),
        "load_mux_config: loaded tui.mux"
    );
    Some(value.clone())
}

/// Convenience: persist using the globally configured data directory
/// ([`codelet_common::get_data_dir`]) and current working directory.
/// Surfaces the data-dir / write error so the caller can log it;
/// treated as non-fatal.
pub fn save_mux_config(config: &Value) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to resolve current directory: {e}"))?;
    save_mux_config_with_dirs(&data_dir, &cwd, config)
}

/// Convenience: load using the globally configured data directory +
/// current working directory. Returns `None` when either cannot be
/// resolved or nothing is persisted, so a TUI constructed before
/// startup wiring degrades gracefully.
#[must_use]
pub fn load_mux_config() -> Option<Value> {
    let Ok(data_dir) = codelet_common::get_data_dir() else {
        return None;
    };
    let Ok(cwd) = std::env::current_dir() else {
        return None;
    };
    load_mux_config_with_dirs(&data_dir, &cwd)
}
