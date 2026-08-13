//! Provider-scoped `cred::` Rhai module (PROV-086).
//!
//! The `cred::` namespace gives Rhai OAuth scripts limited, safe access
//! to `~/.fspec/credentials/<provider>.json`. All four functions
//! (`read`, `write`, `delete`, `path`) enforce a strict rule: the
//! `name` argument must equal the provider name captured when the
//! module was built. Any other value — including path-traversal
//! strings like `../../etc/passwd` — is rejected with an access-denied
//! runtime error before any `PathBuf` is constructed.
//!
//! File I/O uses the synchronous `std::fs` API because Rhai scripts
//! themselves are synchronous. When a scripted flow is driven from
//! async Rust, the outer caller already wraps script execution in
//! `tokio::task::spawn_blocking`, so the blocking I/O here does not
//! stall the runtime.
//!
//! On Unix, files written via `cred::write` are chmod'd to `0o600`.

use std::path::{Path, PathBuf};

use rhai::{Dynamic, Map, Module};

use super::engine::RhaiModule;
use super::json_convert::{dynamic_to_json_value, json_value_to_dynamic};

/// Resolve the fspec credentials directory.
///
/// Mirrors `claude_auth::get_fspec_home`:
///   1. If `FSPEC_HOME` is set, use it verbatim.
///   2. Otherwise, `$HOME/.fspec/credentials` (falling back to
///      `/tmp/.fspec/credentials` when `$HOME` is unavailable).
pub fn fspec_home() -> PathBuf {
    if let Ok(fspec_home) = std::env::var("FSPEC_HOME") {
        PathBuf::from(fspec_home)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        PathBuf::from(home).join(".fspec").join("credentials")
    }
}

/// Build the provider-scoped `cred::` module.
///
/// Functions exposed:
/// * `cred::read(name)` → `Map` if the file exists and parses as JSON;
///   `()` (unit) when the file is absent.
/// * `cred::write(name, map)` → persists the map as pretty-printed JSON
///   to `<fspec_home()>/<name>.json` with Unix mode `0o600`.
/// * `cred::delete(name)` → removes the credential file (idempotent).
/// * `cred::path(name)` → absolute path string; does not touch disk.
pub fn build_cred_module(provider_name: String) -> RhaiModule {
    let mut module = Module::new();

    // cred::path(name) -> absolute path string
    {
        let bound = provider_name.clone();
        module.set_native_fn(
            "path",
            move |name: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
                ensure_name_matches(&bound, &name)?;
                let p = credential_path(&name);
                Ok(Dynamic::from(p.to_string_lossy().into_owned()))
            },
        );
    }

    // cred::read(name) -> Map | ()
    {
        let bound = provider_name.clone();
        module.set_native_fn(
            "read",
            move |name: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
                ensure_name_matches(&bound, &name)?;
                let p = credential_path(&name);
                if !p.exists() {
                    return Ok(Dynamic::UNIT);
                }
                let content = std::fs::read_to_string(&p)
                    .map_err(|e| rt_err(format!("cred::read failed: {e}")))?;
                let value: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| rt_err(format!("cred::read JSON parse failed: {e}")))?;
                Ok(json_value_to_dynamic(&value))
            },
        );
    }

    // cred::write(name, map) -> ()
    {
        let bound = provider_name.clone();
        module.set_native_fn(
            "write",
            move |name: String, value: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
                ensure_name_matches(&bound, &name)?;
                let p = credential_path(&name);
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| rt_err(format!("cred::write create_dir_all failed: {e}")))?;
                }
                let json = dynamic_to_json_value(&Dynamic::from_map(value));
                let serialized = serde_json::to_string_pretty(&json)
                    .map_err(|e| rt_err(format!("cred::write serialization failed: {e}")))?;
                std::fs::write(&p, serialized)
                    .map_err(|e| rt_err(format!("cred::write failed: {e}")))?;
                set_mode_0600(&p).map_err(|e| rt_err(format!("cred::write chmod failed: {e}")))?;
                Ok(Dynamic::UNIT)
            },
        );
    }

    // cred::delete(name) -> ()
    {
        let bound = provider_name;
        module.set_native_fn(
            "delete",
            move |name: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
                ensure_name_matches(&bound, &name)?;
                let p = credential_path(&name);
                if p.exists() {
                    std::fs::remove_file(&p)
                        .map_err(|e| rt_err(format!("cred::delete failed: {e}")))?;
                }
                Ok(Dynamic::UNIT)
            },
        );
    }

    RhaiModule {
        name: "cred".to_string(),
        module,
    }
}

/// Enforce that the requested credential name matches the active
/// provider — used by every `cred::*` function before touching the
/// filesystem.
fn ensure_name_matches(
    provider_name: &str,
    requested: &str,
) -> Result<(), Box<rhai::EvalAltResult>> {
    if requested == provider_name {
        Ok(())
    } else {
        Err(rt_err(format!(
            "cred:: access denied: '{requested}' does not match active provider '{provider_name}'"
        )))
    }
}

/// Compute `<fspec_home()>/<name>.json`.
fn credential_path(name: &str) -> PathBuf {
    fspec_home().join(format!("{name}.json"))
}

/// Construct a boxed Rhai runtime error from a message.
fn rt_err(msg: String) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorRuntime(
        msg.into(),
        rhai::Position::NONE,
    ))
}

/// Apply `0o600` permissions on Unix; no-op elsewhere.
#[cfg(unix)]
fn set_mode_0600(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_mode_0600(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
