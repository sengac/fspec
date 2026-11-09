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

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

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
// load (never writes)
// ============================================================================

/// Read the `fspec-config.json` at `config_path` into a `Value`. `Ok(None)`
/// when the file does not exist; BUG-451: an unreadable file or malformed
/// JSON is an error.
fn read_config_value(config_path: &Path) -> Result<Option<serde_json::Value>, String> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!(path = %config_path.display(), "rlcd config read: file not found (not configured)");
            return Ok(None);
        }
        Err(e) => return Err(format!("cannot read {}: {e}", config_path.display())),
    };
    serde_json::from_str::<serde_json::Value>(&content)
        .map(Some)
        .map_err(|e| format!("{} is not valid JSON: {e}", config_path.display()))
}

/// Path-injectable core: load the RLCD config from the given
/// `fspec-config.json` path. A missing file or a missing `rlcd` section
/// means "not configured" (the documented defaults); BUG-451: an unreadable
/// or malformed file, a non-object root and a malformed `rlcd` section are
/// errors. Never writes.
pub fn load_rlcd_config_from(config_path: &Path) -> Result<RlcdConfig, String> {
    let Some(root) = read_config_value(config_path)? else {
        return Ok(RlcdConfig::default());
    };
    let root = root
        .as_object()
        .ok_or_else(|| format!("{} is not a JSON object", config_path.display()))?;
    let Some(section) = root.get("rlcd") else {
        return Ok(RlcdConfig::default());
    };
    serde_json::from_value(section.clone()).map_err(|e| {
        format!(
            "the rlcd section of {} is malformed: {e}",
            config_path.display()
        )
    })
}

/// Convenience: load using the env-resolved user directory
/// (`FSPEC_USER_DIR` / `~/.fspec`).
pub fn load_rlcd_config() -> Result<RlcdConfig, String> {
    load_rlcd_config_from(&codelet_common::user_dir::fspec_user_dir()?.join("fspec-config.json"))
}

// ============================================================================
// save (key-preserving read-merge-write)
// ============================================================================

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
    // BUG-451: a missing file starts empty; an unreadable, malformed or
    // non-object one is an error (it is never overwritten). BUG-640: one
    // locked read-modify-write.
    codelet_common::fspec_config::update_config_file(config_path, |root| {
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
        Ok::<_, String>(())
    })
}

/// Convenience: persist `rlcd.url` using the env-resolved user directory.
pub fn save_rlcd_url(url: &str) -> Result<(), String> {
    let dir = codelet_common::user_dir::fspec_user_dir()?;
    save_rlcd_url_at(&dir.join("fspec-config.json"), url)
}
