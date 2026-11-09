//! CONFIG-008 — shared `fspec-config.json` module with two-scope deep merge.
//!
//! Feature: spec/features/shared-config-merge.feature
//!
//! Mirrors the TypeScript reference `src/utils/config.ts` (`loadConfig` /
//! `writeConfig`). Settings live in **one shared JSON file** per scope and are
//! deep-merged so the Rust port interoperates with the TS `fspec` CLI:
//!
//!   * User scope:    `<data_dir>/fspec-config.json`
//!   * Project scope: `<project root>/spec/fspec-config.json`, the root
//!     resolved from the working directory (overrides user on merge)
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

/// Project-scope config path: `<project root>/spec/fspec-config.json`, where
/// the project root is resolved from `cwd` (BUG-497: the same root every
/// fspec command uses, so a save from a subdirectory never creates a stray
/// `<cwd>/spec`). WT-015: the config is shared state, so an isolated copy
/// holding a state-root pointer reads and writes the main checkout's.
pub fn project_config_path(cwd: &Path) -> Result<PathBuf, String> {
    crate::spec_paths::ProjectRoots::resolve(cwd)
        .map(|roots| roots.config())
        .map_err(|err| err.to_string())
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

/// BUG-461: read a config file that must hold a JSON object. A missing (or
/// empty) file is an empty object — nothing saved yet; an unreadable file,
/// invalid JSON or a non-object root is an error naming the file, so a
/// writer never replaces a config it could not read.
pub fn load_config_object(path: &Path) -> Result<Value, String> {
    let value = load_config_file(path)?;
    if !value.is_object() {
        return Err(format!("{} must hold a JSON object", path.display()));
    }
    Ok(value)
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

/// Load the USER-scope config file RAW (no merge).
///
/// BUG-193: the counterpart of [`load_config_with_dirs`] for the WRITE half of
/// user-scope persistence. User-scope save cores must read only this file so
/// a write can never carry project-scope keys (e.g. `tools`, `agent`) into
/// `~/.fspec/fspec-config.json`. Semantics mirror [`load_config_file`]:
///
/// * A missing file → `Ok(Value::Object(empty))` (silent fallback).
/// * An empty / whitespace-only file → `Ok(Value::Object(empty))`.
/// * Invalid JSON → `Err("Invalid JSON in <path>: <msg>")`.
/// * A non-object root → an error naming the file (BUG-461).
pub fn load_user_config_file(data_dir: &Path) -> Result<Value, String> {
    load_config_object(&user_config_path(data_dir))
}

/// Load + deep-merge user then project scope.
///
/// Project config overrides user config. `data_dir` selects the user file;
/// `cwd` selects the project file.
pub fn load_config_with_dirs(data_dir: &Path, cwd: &Path) -> Result<Value, String> {
    let user = load_config_file(&user_config_path(data_dir))?;
    let project = load_config_file(&project_config_path(cwd)?)?;
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

/// BUG-640: an `fspec-config.json` read-modify-write failed (lock, read,
/// serialise or write); the message names the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFileError(pub String);

impl std::fmt::Display for ConfigFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigFileError {}

impl From<ConfigFileError> for String {
    fn from(error: ConfigFileError) -> Self {
        error.0
    }
}

impl From<ConfigFileError> for std::io::Error {
    fn from(error: ConfigFileError) -> Self {
        std::io::Error::other(error.0)
    }
}

/// BUG-640: the one writer of an `fspec-config.json`. Holds the spec lock
/// of the file's directory from the read to the write, so concurrent
/// writers (threads or processes) never lose each other's changes; reads
/// the file with [`load_config_object`] (a missing or empty file is `{}`,
/// a broken one fails and is never written over); applies `update`; and,
/// when `update` succeeds and changed the config, replaces the file
/// atomically with pretty JSON and a trailing newline. An `update` error,
/// or an update that changed nothing, leaves the file as it was.
pub fn update_config_file<T, E>(
    path: &Path,
    update: impl FnOnce(&mut Value) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<ConfigFileError>,
{
    let fail = |message: String| E::from(ConfigFileError(message));
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => return Err(fail(format!("{} has no parent directory", path.display()))),
    };
    std::fs::create_dir_all(directory).map_err(|err| {
        fail(format!(
            "Failed to create config directory {}: {err}",
            directory.display()
        ))
    })?;
    let _lock = crate::spec_lock::SpecLock::acquire(directory)
        .map_err(|err| fail(format!("cannot lock {}: {err}", path.display())))?;
    let before = load_config_object(path).map_err(fail)?;
    let mut config = before.clone();
    let result = update(&mut config)?;
    if config == before {
        return Ok(result);
    }
    let mut json = serde_json::to_string_pretty(&config)
        .map_err(|err| fail(format!("Failed to serialize {}: {err}", path.display())))?;
    json.push('\n');
    crate::spec_lock::replace_file_atomically(path, json.as_bytes()).map_err(|err| {
        fail(format!(
            "Failed to write config file {}: {err}",
            path.display()
        ))
    })?;
    Ok(result)
}

/// Config write scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// `<data_dir>/fspec-config.json`.
    User,
    /// `<project root>/spec/fspec-config.json`.
    Project,
}

/// [`update_config_file`] on the chosen scope's file.
pub fn update_config_with_dirs<T, E>(
    scope: ConfigScope,
    data_dir: &Path,
    cwd: &Path,
    update: impl FnOnce(&mut Value) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<ConfigFileError>,
{
    let path = match scope {
        ConfigScope::User => user_config_path(data_dir),
        ConfigScope::Project => {
            project_config_path(cwd).map_err(|err| E::from(ConfigFileError(err)))?
        }
    };
    update_config_file(&path, update)
}
