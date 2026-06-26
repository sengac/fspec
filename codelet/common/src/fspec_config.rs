//! CONFIG-008 — shared `fspec-config.json` module with two-scope deep merge.
//!
//! Feature: spec/features/shared-config-merge.feature
//!
//! Mirrors the TypeScript reference `src/utils/config.ts` (`loadConfig` /
//! `writeConfig`). Settings live in **one shared JSON file** per scope and are
//! deep-merged so the Rust port interoperates with the TS `fspec` CLI:
//!
//!   * User scope:    `<data_dir>/fspec-config.json`
//!   * Project scope: `<cwd>/spec/fspec-config.json` (overrides user on merge)
//!
//! `serde_json::Value` is the in-memory representation, mirroring the untyped JS
//! object. The path-injectable `*_with_dirs` cores accept explicit directories so
//! tests use OS temp dirs (no `$HOME` mutation, no global `set_data_directory`);
//! thin global wrappers resolve [`codelet_common::get_data_dir`] +
//! [`std::env::current_dir`]. No `unwrap()` in fallible paths — all surface
//! `Result<_, String>`.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Shared config file name (under both the data dir and `<cwd>/spec`).
const CONFIG_FILE: &str = "fspec-config.json";

/// User-scope config path: `<data_dir>/fspec-config.json`.
#[must_use]
pub fn user_config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CONFIG_FILE)
}

/// Project-scope config path: `<cwd>/spec/fspec-config.json`.
#[must_use]
pub fn project_config_path(cwd: &Path) -> PathBuf {
    cwd.join("spec").join(CONFIG_FILE)
}

/// Load a single config file.
///
/// * A missing file → `Ok(Value::Object(empty))` (silent fallback).
/// * An empty / whitespace-only file → `Ok(Value::Object(empty))`.
/// * Invalid JSON → `Err("Invalid JSON in <path>: <msg>")`.
fn load_config_file(path: &Path) -> Result<Value, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Value::Object(Map::new()));
        }
        Err(err) => return Err(format!("Failed to read {}: {err}", path.display())),
    };

    if content.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }

    serde_json::from_str(&content)
        .map_err(|err| format!("Invalid JSON in {}: {err}", path.display()))
}

/// Deep-merge `source` over `target` (project over user).
///
/// For each key in `source`: if both `target` and `source` hold a JSON object,
/// merge recursively (preserving sibling keys); otherwise `source` replaces
/// `target` wholesale (arrays and scalars replace, never merge).
#[must_use]
pub fn deep_merge(target: Value, source: Value) -> Value {
    match (target, source) {
        (Value::Object(mut target_map), Value::Object(source_map)) => {
            for (key, source_value) in source_map {
                let merged = match target_map.remove(&key) {
                    Some(target_value) => deep_merge(target_value, source_value),
                    None => source_value,
                };
                target_map.insert(key, merged);
            }
            Value::Object(target_map)
        }
        // Source is not an object (scalar/array/null), or target is not an
        // object: source replaces target.
        (_, source) => source,
    }
}

/// Load + deep-merge user then project scope.
///
/// Project config overrides user config. `data_dir` selects the user file;
/// `cwd` selects the project file.
pub fn load_config_with_dirs(data_dir: &Path, cwd: &Path) -> Result<Value, String> {
    let user = load_config_file(&user_config_path(data_dir))?;
    let project = load_config_file(&project_config_path(cwd))?;
    Ok(deep_merge(user, project))
}

/// Convenience wrapper using [`codelet_common::get_data_dir`] +
/// [`std::env::current_dir`].
pub fn load_config() -> Result<Value, String> {
    let data_dir = crate::get_data_dir()?;
    let cwd = std::env::current_dir()
        .map_err(|err| format!("Failed to resolve current directory: {err}"))?;
    load_config_with_dirs(&data_dir, &cwd)
}

/// Config write scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// `<data_dir>/fspec-config.json`.
    User,
    /// `<cwd>/spec/fspec-config.json`.
    Project,
}

/// Write a whole config `Value` to the chosen scope.
///
/// Creates the parent directory on demand and writes pretty (2-space) JSON,
/// mirroring the TS `writeConfig`.
pub fn write_config_with_dirs(
    scope: ConfigScope,
    config: &Value,
    data_dir: &Path,
    cwd: &Path,
) -> Result<(), String> {
    let path = match scope {
        ConfigScope::User => user_config_path(data_dir),
        ConfigScope::Project => project_config_path(cwd),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create config directory {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|err| format!("Failed to serialize config: {err}"))?;
    std::fs::write(&path, json)
        .map_err(|err| format!("Failed to write config file {}: {err}", path.display()))
}

/// Convenience wrapper using [`codelet_common::get_data_dir`] +
/// [`std::env::current_dir`].
pub fn write_config(scope: ConfigScope, config: &Value) -> Result<(), String> {
    let data_dir = crate::get_data_dir()?;
    let cwd = std::env::current_dir()
        .map_err(|err| format!("Failed to resolve current directory: {err}"))?;
    write_config_with_dirs(scope, config, &data_dir, &cwd)
}
