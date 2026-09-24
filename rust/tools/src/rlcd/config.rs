//! RLCD-001 — user-config loading/writing for the RLCD service supervisor.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! The RLCD config lives ONLY in the user config
//! `~/.fspec/fspec-config.json` under a top-level `"rlcd"` key (user-config
//! only — no project-level merge, HITL decision 2026-09-24). All fields are
//! optional with defaults; a missing/malformed file resolves to defaults
//! (never fatal, never writes). Writes are key-preserving read-merge-write
//! (only the `rlcd` key changes; every other key is preserved verbatim —
//! the same contract as `tui.lastUsedModel`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::rlcd::gate_config::RlcdGateConfig;
use crate::rlcd::security_config::RlcdSecurityConfig;

// ============================================================================
// config schema
// ============================================================================

/// Spawn (auto-start) section of the RLCD config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RlcdSpawnConfig {
    /// Local address the spawned server binds.
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    /// First port the scan starts from.
    #[serde(default = "default_spawn_port")]
    pub port: u16,
    /// Number of consecutive ports the scan checks before failing open.
    #[serde(default = "default_port_scan_limit")]
    pub port_scan_limit: usize,
    /// Executable resolved on PATH (model-agnostic name; current backend:
    /// laya-rs, whose CLI binary is `rlcd`).
    #[serde(default = "default_binary")]
    pub binary: String,
}

impl Default for RlcdSpawnConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_spawn_port(),
            port_scan_limit: default_port_scan_limit(),
            binary: default_binary(),
        }
    }
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_spawn_port() -> u16 {
    8000
}

fn default_port_scan_limit() -> usize {
    10
}

fn default_binary() -> String {
    "rlcd".to_string()
}

/// Top-level `rlcd` config section.
///
/// `backend` is an informational protocol-family tag (renamed from the
/// former backend name — RLCD is model-agnostic; the current backend is
/// laya-rs speaking the Jev/Simple-Jev v1 protocol).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RlcdConfig {
    /// Master switch: when false, consumers treat RLCD as disabled
    /// (fail-open degradation without spawn attempts).
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Endpoint to connect to. Local (127.0.0.1/localhost) URLs are
    /// spawn-eligible; remote URLs are connect-only.
    #[serde(default = "default_url")]
    pub url: String,
    /// Informational protocol-family tag.
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Local auto-spawn section.
    #[serde(default)]
    pub spawn: RlcdSpawnConfig,
    /// Semantic gate section (RLCD-003: workflow command review).
    #[serde(default)]
    pub gate: RlcdGateConfig,
    /// Semantic security layer section (RLCD-004: blocklist second stage).
    #[serde(default)]
    pub security: RlcdSecurityConfig,
}

impl Default for RlcdConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            url: default_url(),
            backend: default_backend(),
            spawn: RlcdSpawnConfig::default(),
            gate: RlcdGateConfig::default(),
            security: RlcdSecurityConfig::default(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_url() -> String {
    "http://127.0.0.1:8000".to_string()
}

fn default_backend() -> String {
    "rlcd".to_string()
}


// ============================================================================
// user dir resolution (mirror of codelet-sessions profile_sections::fspec_user_dir)
// ============================================================================

/// Resolve the fspec user directory: `FSPEC_USER_DIR` env override (tests) →
/// `dirs::home_dir()/.fspec` → `$HOME/.fspec` last resort.
///
/// Mirrors `codelet_sessions::profile_sections::fspec_user_dir` (that one is
/// `pub(crate)` in codelet-sessions, so codelet-tools keeps its own copy —
/// the crate arrow tools → sessions is forbidden).
#[must_use]
pub fn user_dir() -> Option<PathBuf> {
    if let Some(override_dir) = std::env::var_os("FSPEC_USER_DIR") {
        return Some(PathBuf::from(override_dir));
    }
    if let Some(home) = dirs::home_dir() {
        return Some(home.join(".fspec"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".fspec"))
}

// ============================================================================
// load (never fatal, never writes)
// ============================================================================

/// Read the `fspec-config.json` at `config_path` into a `Value`.
/// `None` on a missing or malformed file (mirrors
/// `profile_sections::read_config_value`).
fn read_config_value(config_path: &Path) -> Option<serde_json::Value> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!(path = %config_path.display(), "rlcd config read: file not found (defaults apply)");
            return None;
        }
        Err(e) => {
            warn!(path = %config_path.display(), error = %e, "rlcd config read: failed to read file");
            return None;
        }
    };
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(value) => Some(value),
        Err(e) => {
            warn!(path = %config_path.display(), error = %e, "rlcd config read: malformed JSON (defaults apply)");
            None
        }
    }
}

/// Path-injectable core: load the RLCD config from the given
/// `fspec-config.json` path. Missing/malformed file or section → full
/// defaults. Never errors, never writes.
#[must_use]
pub fn load_rlcd_config_from(config_path: &Path) -> RlcdConfig {
    let root = match read_config_value(config_path) {
        Some(v) if v.is_object() => v,
        _ => return RlcdConfig::default(),
    };
    let Some(section) = root.get("rlcd").and_then(|v| v.as_object()) else {
        return RlcdConfig::default();
    };
    serde_json::from_value(serde_json::Value::Object(section.clone()))
        .unwrap_or_else(|e| {
            warn!(error = %e, "rlcd config: section malformed (defaults apply)");
            RlcdConfig::default()
        })
}

/// Convenience: load using the env-resolved user directory
/// (`FSPEC_USER_DIR` / `~/.fspec`).
#[must_use]
pub fn load_rlcd_config() -> RlcdConfig {
    match user_dir() {
        Some(dir) => load_rlcd_config_from(&dir.join("fspec-config.json")),
        None => RlcdConfig::default(),
    }
}

// ============================================================================
// save (key-preserving read-merge-write)
// ============================================================================

/// Write the JSON `Value` back to `config_path` (pretty + trailing newline).
/// `preserve_order` serde keeps existing key order (mirrors
/// `profile_sections::write_config_value`).
fn write_config_value(config_path: &Path, root: &serde_json::Value) -> std::io::Result<()> {
    let mut serialized = serde_json::to_string_pretty(root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    serialized.push('\n');
    std::fs::write(config_path, &serialized)
}

/// Path-injectable core: persist `rlcd.url = url` into
/// `<config_path>`, preserving every unrelated key verbatim (the
/// `save_persisted_model_string_to` contract, top-level `rlcd` key).
///
/// Missing/malformed files start from `{}`; a non-object root is replaced by
/// `{}`; a non-object `rlcd` section is replaced by a fresh object. An empty
/// `url` is a no-op.
pub fn save_rlcd_url_at(config_path: &Path, url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Ok(());
    }
    let mut root = read_config_value(config_path).unwrap_or_else(|| serde_json::json!({}));
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "fspec-config.json root is not a JSON object".to_string())?;
    let section = obj.entry("rlcd").or_insert_with(|| serde_json::json!({}));
    let section_obj = section
        .as_object_mut()
        .ok_or_else(|| "fspec-config.json `rlcd` is not a JSON object".to_string())?;
    section_obj.insert(
        "url".to_string(),
        serde_json::Value::String(url.to_string()),
    );
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create user directory: {e}"))?;
    }
    write_config_value(config_path, &root)
        .map_err(|e| format!("Failed to write fspec-config.json: {e}"))
}

/// Convenience: persist `rlcd.url` using the env-resolved user directory.
pub fn save_rlcd_url(url: &str) -> Result<(), String> {
    let dir = user_dir().ok_or_else(|| "Could not resolve fspec user directory".to_string())?;
    save_rlcd_url_at(&dir.join("fspec-config.json"), url)
}
