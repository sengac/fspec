//! TUI-002 / TUI-092 / MODEL-012 — persist the thinking level the user saves
//! with D, per model, across process restarts.
//!
//! Feature: spec/features/thinking-level-remembered-per-model.feature
//!
//! The saved levels live in the **shared CONFIG-008 `fspec-config.json`**
//! under `tui.thinkingLevels`, an object from the model as fspec names it in
//! thinking errors (`provider/model`, or `provider:profile/model` for a
//! profile model) to the level as one string: `off`, `on`,
//! `effort:<name>`, `numeric:<number>` or `budget:<tokens>`. One string per
//! model means the project-over-user deep merge replaces a model's level
//! whole and never mixes two levels (product owner, 2026-11-04, decision 6).
//!
//! Save is a read-modify-write of the RAW user file that preserves sibling
//! keys (BUG-193, BUG-461). Load reads the project-over-user merged config.
//! Nothing is ever loaded as Off: a missing file or entry is "no saved
//! level", and an unreadable config, a `tui.thinkingLevels` that is not an
//! object or an entry that is not a level is an error the caller reports.
//! The old `tui.defaultThinkingLevel` int is neither read nor written, and
//! it is never mapped onto a level (product owner, 2026-11-05).

use std::path::Path;

use serde_json::{Map, Value};

use codelet_common::fspec_config::{load_config_with_dirs, update_config_with_dirs, ConfigScope};
use codelet_rpc_types::ThinkingLevel;

/// Nested config keys holding the saved levels: `tui.thinkingLevels`.
const TUI_KEY: &str = "tui";
const THINKING_LEVELS_KEY: &str = "thinkingLevels";

/// The stored form of `level`; the model default has none (saving it
/// removes the model's entry).
fn encode(level: &ThinkingLevel) -> Option<String> {
    match level {
        ThinkingLevel::ModelDefault => None,
        ThinkingLevel::Off => Some("off".to_string()),
        ThinkingLevel::On => Some("on".to_string()),
        ThinkingLevel::Effort(name) => Some(format!("effort:{name}")),
        ThinkingLevel::Numeric(number) => Some(format!("numeric:{number}")),
        ThinkingLevel::Budget(tokens) => Some(format!("budget:{tokens}")),
    }
}

/// The level a stored entry holds, or why it is not a level.
fn decode(entry: &Value) -> Result<ThinkingLevel, String> {
    let text = entry
        .as_str()
        .ok_or_else(|| format!("{entry} is not a level string"))?;
    let not_a_level = || {
        format!(
            "\"{text}\" is not a level (expected off, on, effort:<name>, numeric:<number> or budget:<tokens>)"
        )
    };
    match text {
        "off" => return Ok(ThinkingLevel::Off),
        "on" => return Ok(ThinkingLevel::On),
        _ => {}
    }
    let (kind, value) = text.split_once(':').ok_or_else(not_a_level)?;
    match kind {
        "effort" if !value.is_empty() => Ok(ThinkingLevel::Effort(value.to_string())),
        "numeric" => serde_json::from_str::<serde_json::Number>(value)
            .map(ThinkingLevel::Numeric)
            .map_err(|_| not_a_level()),
        "budget" => value
            .parse::<u64>()
            .map(ThinkingLevel::Budget)
            .map_err(|_| not_a_level()),
        _ => Err(not_a_level()),
    }
}

/// Save `level` as `model`'s thinking level in the USER-scope shared config,
/// preserving every other key; the model default removes `model`'s entry.
///
/// Path-injectable core used by the global wrapper and tests. `data_dir`
/// selects the user `fspec-config.json`; `cwd` is only passed to the write.
/// BUG-193: this reads the RAW user file (never the project-over-user merged
/// view) so a user-scope write never carries project-scope keys. BUG-461: an
/// unreadable, invalid or non-object user file fails the save and is left
/// unchanged.
pub fn save_default_thinking_level_with_dirs(
    data_dir: &Path,
    cwd: &Path,
    model: &str,
    level: ThinkingLevel,
) -> Result<(), String> {
    // BUG-640: one locked read-modify-write of the user file (BUG-193: never
    // the merged view, so no project key is carried into it).
    update_config_with_dirs(ConfigScope::User, data_dir, cwd, |config| {
        let root = config
            .as_object_mut()
            .ok_or_else(|| "config root is not a JSON object".to_string())?;
        let tui = root
            .entry(TUI_KEY)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| "config tui section is not a JSON object".to_string())?;
        let levels = tui
            .entry(THINKING_LEVELS_KEY)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| "config tui.thinkingLevels is not a JSON object".to_string())?;
        match encode(&level) {
            // BUG-455: saving the model default removes the model's saved level.
            None => {
                levels.remove(model);
            }
            Some(stored) => {
                levels.insert(model.to_string(), Value::String(stored));
            }
        }
        Ok::<_, String>(())
    })
    .map_err(|error| {
        tracing::warn!(%error, %model, level = %level, "save_default_thinking_level: failed to persist");
        error
    })?;
    tracing::info!(
        %model,
        level = %level,
        data_dir = %data_dir.display(),
        "save_default_thinking_level: persisted tui.thinkingLevels"
    );
    Ok(())
}

/// The thinking level saved for `model`: `Ok(None)` when none is saved (no
/// config file, no `tui.thinkingLevels`, or no entry for `model`), and an
/// error naming what could not be read otherwise. Reads the project-over-user
/// merged config. Path-injectable core.
pub fn load_default_thinking_level_with_dirs(
    data_dir: &Path,
    cwd: &Path,
    model: &str,
) -> Result<Option<ThinkingLevel>, String> {
    let config = load_config_with_dirs(data_dir, cwd)?;
    let levels = match config
        .get(TUI_KEY)
        .and_then(|tui| tui.get(THINKING_LEVELS_KEY))
    {
        None => return Ok(None),
        Some(levels) => levels
            .as_object()
            .ok_or_else(|| "tui.thinkingLevels is not a JSON object".to_string())?,
    };
    levels.get(model).map(decode).transpose()
}

/// Save using the globally configured data directory
/// ([`codelet_common::get_data_dir`]) and the current working directory.
pub fn save_default_thinking_level(model: &str, level: ThinkingLevel) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to resolve current directory: {e}"))?;
    save_default_thinking_level_with_dirs(&data_dir, &cwd, model, level)
}

/// Load using the globally configured data directory and the current working
/// directory; an unresolvable directory is an error, never "no level".
pub fn load_default_thinking_level(model: &str) -> Result<Option<ThinkingLevel>, String> {
    let data_dir = codelet_common::get_data_dir()?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to resolve current directory: {e}"))?;
    load_default_thinking_level_with_dirs(&data_dir, &cwd, model)
}
