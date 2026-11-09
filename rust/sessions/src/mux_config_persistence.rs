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
//! (`tui.lastUsedModel`, `tui.thinkingLevels`).
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
//!   * A missing `tui.mux` key loads as `Ok(None)` (the caller uses the
//!     default preset). BUG-708: a config that cannot be read or parsed is
//!     an `Err` naming the file, which the TUI shows before using the
//!     default for that session.
//!   * Persistence is best-effort: the `_with_dirs` core surfaces an
//!     `Err` that the caller logs and swallows (non-fatal).

use std::path::Path;

use serde_json::{Map, Value};

use codelet_common::fspec_config::{load_config_with_dirs, update_config_with_dirs, ConfigScope};

/// Nested config keys holding the persisted mux config: `tui.mux`.
const TUI_KEY: &str = "tui";
const MUX_KEY: &str = "mux";

/// Persist `config` (a JSON object) under `tui.mux` in the USER-scope
/// shared config, preserving any existing sibling keys
/// (read-modify-write).
///
/// Path-injectable core used by both the global convenience wrapper and
/// tests. `data_dir` selects the user `fspec-config.json`; `_cwd` is
/// retained for call-site stability but the save no longer reads the
/// deep-merged view — BUG-193 requires the write half to read the RAW
/// user file so project-scope keys are never mirrored into it.
pub fn save_mux_config_with_dirs(
    data_dir: &Path,
    _cwd: &Path,
    config: &Value,
) -> Result<(), String> {
    // BUG-193: read the RAW user file (never the deep-merged
    // project-over-user view) so the write-back can only ever carry
    // user-scope keys — project-scope keys such as `tools` or `agent`
    // must not be mirrored into `~/.fspec/fspec-config.json`. BUG-461: an
    // unreadable, invalid or non-object user file fails the save and is
    // left unchanged. (`cwd` stays in the signature for load-side call-site
    // stability; the save itself no longer needs it.)
    let saved = update_config_with_dirs(ConfigScope::User, data_dir, _cwd, |root| {
        let root_map = root
            .as_object_mut()
            .ok_or_else(|| "config root is not a JSON object".to_string())?;
        let tui = root_map
            .entry(TUI_KEY)
            .or_insert_with(|| Value::Object(Map::new()));
        if !tui.is_object() {
            *tui = Value::Object(Map::new());
        }
        let tui_map = tui
            .as_object_mut()
            .ok_or_else(|| "config tui section is not a JSON object".to_string())?;
        tui_map.insert(MUX_KEY.to_string(), config.clone());
        Ok::<_, String>(())
    });
    match saved {
        Ok(()) => {
            tracing::info!(
                data_dir = %data_dir.display(),
                _cwd = %_cwd.display(),
                "save_mux_config: persisted tui.mux"
            );
            Ok(())
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                data_dir = %data_dir.display(),
                _cwd = %_cwd.display(),
                "save_mux_config: failed to persist (non-fatal)"
            );
            Err(e)
        }
    }
}

/// Load the persisted mux config from the shared config.
///
/// Reads `tui.mux` from the deep-merged (project-over-user) config; a
/// missing key yields `Ok(None)`. MUX-011: any other value, an object or
/// not, is returned as it is so the TUI reports what is wrong with it
/// instead of silently using the default. BUG-708: a config file that
/// cannot be read or parsed is the loader's error, which names the file.
/// Path-injectable core.
pub fn load_mux_config_with_dirs(data_dir: &Path, cwd: &Path) -> Result<Option<Value>, String> {
    let config = load_config_with_dirs(data_dir, cwd)?;
    let value = &config[TUI_KEY][MUX_KEY];
    if value.is_null() {
        return Ok(None);
    }
    tracing::debug!(
        data_dir = %data_dir.display(),
        cwd = %cwd.display(),
        "load_mux_config: loaded tui.mux"
    );
    Ok(Some(value.clone()))
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
/// current working directory. `Ok(None)` when nothing is persisted;
/// BUG-708: a data directory or current directory that cannot be
/// resolved, or a config that cannot be read, is an `Err`.
pub fn load_mux_config() -> Result<Option<Value>, String> {
    let data_dir = codelet_common::get_data_dir()?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to resolve current directory: {e}"))?;
    load_mux_config_with_dirs(&data_dir, &cwd)
}
