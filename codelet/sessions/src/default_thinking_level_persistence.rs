//! TUI-002 / TUI-092 — persist the user-selected default thinking level across
//! process restarts and re-apply it to every new/resumed session.
//!
//! Feature: spec/features/default-thinking-level-shared-config-persistence.feature
//!
//! Mirrors the TS reference `src/tui/config/defaultThinkingLevelConfig.ts`: the
//! chosen default thinking level (set via the `/thinking` dialog →
//! `set_thinking_level_default`) is stored in the **shared CONFIG-008
//! `fspec-config.json`** under the nested key `tui.defaultThinkingLevel`
//! (int `0..=3`), NOT a dedicated file. This restores interop with the TS
//! `fspec` CLI (sibling keys such as `tui.lastUsedModel` live in the same file)
//! and honours the project-scope override via CONFIG-008's deep merge.
//!
//! Save is a read-modify-write to USER scope that preserves sibling keys; load
//! relies on `load_config_with_dirs` so a project `<cwd>/spec/fspec-config.json`
//! overrides the user value.
//!
//! Invariants:
//!   * Only `0..=3` is meaningful (`0=Off, 1=Low, 2=Medium, 3=High`).
//!   * A missing / malformed config, or an out-of-range value, loads as
//!     [`ThinkingLevel::Off`] (graceful degradation — load is infallible).
//!   * Persistence is best-effort: the `_with_dirs` core surfaces an `Err` that
//!     the caller logs and swallows (non-fatal).

use std::path::Path;

use serde_json::{Map, Value};

use codelet_common::fspec_config::{load_config_with_dirs, write_config_with_dirs, ConfigScope};
use codelet_rpc_types::ThinkingLevel;

/// Nested config keys holding the persisted default level: `tui.defaultThinkingLevel`.
const TUI_KEY: &str = "tui";
const DEFAULT_THINKING_LEVEL_KEY: &str = "defaultThinkingLevel";

/// Map a validated `0..=3` integer to a [`ThinkingLevel`]; anything else → `Off`.
fn level_from_u8(level: u8) -> ThinkingLevel {
    match level {
        1 => ThinkingLevel::Low,
        2 => ThinkingLevel::Medium,
        3 => ThinkingLevel::High,
        _ => ThinkingLevel::Off,
    }
}

/// Persist `level` under `tui.defaultThinkingLevel` in the USER-scope shared
/// config, preserving any existing sibling keys (read-modify-write).
///
/// Path-injectable core used by both the global convenience wrapper and tests.
/// `data_dir` selects the user `fspec-config.json`; `cwd` is required so the
/// read half loads the deep-merged view (its write half always targets USER).
pub fn save_default_thinking_level_with_dirs(
    data_dir: &Path,
    cwd: &Path,
    level: ThinkingLevel,
) -> Result<(), String> {
    // Read the existing merged config; an unreadable / invalid file degrades to
    // an empty object rather than aborting the save.
    let mut config =
        load_config_with_dirs(data_dir, cwd).unwrap_or_else(|_| Value::Object(Map::new()));

    // Ensure the root is an object (a non-object persisted value is replaced).
    if !config.is_object() {
        config = Value::Object(Map::new());
    }

    // Ensure `config["tui"]` is an object, then set the nested key.
    let root = config
        .as_object_mut()
        .ok_or_else(|| "config root is not a JSON object".to_string())?;
    let tui = root
        .entry(TUI_KEY)
        .or_insert_with(|| Value::Object(Map::new()));
    if !tui.is_object() {
        *tui = Value::Object(Map::new());
    }
    let tui_map = tui
        .as_object_mut()
        .ok_or_else(|| "config tui section is not a JSON object".to_string())?;
    tui_map.insert(
        DEFAULT_THINKING_LEVEL_KEY.to_string(),
        Value::from(level as u8),
    );

    match write_config_with_dirs(ConfigScope::User, &config, data_dir, cwd) {
        Ok(()) => {
            // TUI-002: mirror the model path's `set_default_model` success
            // logging (structured fields) so the default-thinking save path is
            // observable in the combined log alongside `lastUsedModel` writes.
            tracing::info!(
                level = level as u8,
                data_dir = %data_dir.display(),
                cwd = %cwd.display(),
                "save_default_thinking_level: persisted tui.defaultThinkingLevel"
            );
            Ok(())
        }
        Err(e) => {
            // Surface the swallowed write error here (best-effort core) so a
            // failed save is no longer silent even if a caller drops the Err.
            tracing::warn!(
                error = %e,
                level = level as u8,
                data_dir = %data_dir.display(),
                cwd = %cwd.display(),
                "save_default_thinking_level: failed to persist (non-fatal)"
            );
            Err(e)
        }
    }
}

/// Load the persisted default thinking level from the shared config.
///
/// Reads `tui.defaultThinkingLevel` from the deep-merged (project-over-user)
/// config; a missing / malformed config, or an out-of-range value, yields
/// [`ThinkingLevel::Off`] (graceful degradation). Path-injectable core.
#[must_use]
pub fn load_default_thinking_level_with_dirs(data_dir: &Path, cwd: &Path) -> ThinkingLevel {
    let Ok(config) = load_config_with_dirs(data_dir, cwd) else {
        tracing::debug!(
            data_dir = %data_dir.display(),
            cwd = %cwd.display(),
            "load_default_thinking_level: config unreadable, defaulting to Off"
        );
        return ThinkingLevel::Off;
    };
    let level = config[TUI_KEY][DEFAULT_THINKING_LEVEL_KEY]
        .as_u64()
        .filter(|n| *n <= 3)
        .map_or(ThinkingLevel::Off, |n| level_from_u8(n as u8));
    tracing::debug!(
        level = level as u8,
        data_dir = %data_dir.display(),
        cwd = %cwd.display(),
        "load_default_thinking_level: loaded tui.defaultThinkingLevel"
    );
    level
}

/// Convenience: persist using the globally configured data directory
/// ([`codelet_common::get_data_dir`]) and current working directory. Surfaces
/// the data-dir / write error so the caller can log it; treated as non-fatal.
pub fn save_default_thinking_level(level: ThinkingLevel) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to resolve current directory: {e}"))?;
    save_default_thinking_level_with_dirs(&data_dir, &cwd, level)
}

/// Convenience: load using the globally configured data directory + current
/// working directory. Returns [`ThinkingLevel::Off`] when either cannot be
/// resolved or nothing is persisted, so a session constructed before startup
/// wiring degrades gracefully.
#[must_use]
pub fn load_default_thinking_level() -> ThinkingLevel {
    let Ok(data_dir) = codelet_common::get_data_dir() else {
        return ThinkingLevel::Off;
    };
    let Ok(cwd) = std::env::current_dir() else {
        return ThinkingLevel::Off;
    };
    load_default_thinking_level_with_dirs(&data_dir, &cwd)
}

/// Load the persisted default thinking level, preserving the TS-parity
/// "no key" vs "explicit value" distinction (TUI-093).
///
/// Mirrors the TS `loadDefaultThinkingLevel(): JsThinkingLevel | null`:
/// returns `None` when the key is absent / the config is unreadable / the value
/// is out of range, and `Some(level)` (including `Some(Off)` for an explicit
/// `0`) when a valid `0..=3` is present. The guarded apply path depends on this
/// distinction so it never clobbers a session when no default was ever chosen.
/// Path-injectable core.
#[must_use]
pub fn load_default_thinking_level_opt_with_dirs(
    data_dir: &Path,
    cwd: &Path,
) -> Option<ThinkingLevel> {
    let config = load_config_with_dirs(data_dir, cwd).ok()?;
    config[TUI_KEY][DEFAULT_THINKING_LEVEL_KEY]
        .as_u64()
        .filter(|n| *n <= 3)
        .map(|n| level_from_u8(n as u8))
}

/// Convenience: Option-returning load using the globally configured data
/// directory + current working directory. Returns `None` when either cannot be
/// resolved or nothing valid is persisted (TUI-093 TS-parity `null`).
#[must_use]
pub fn load_default_thinking_level_opt() -> Option<ThinkingLevel> {
    let data_dir = codelet_common::get_data_dir().ok()?;
    let cwd = std::env::current_dir().ok()?;
    load_default_thinking_level_opt_with_dirs(&data_dir, &cwd)
}
