#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
//! Shared helpers for PROV-062 custom-provider integration tests.
//!
//! Included via `#[path = "custom_test_helpers.rs"] mod helpers;`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// The 7 required functions for a fully-custom provider script.
pub const VALID_SCRIPT: &str = r#"
fn build_request(ctx) { #{} }
fn build_headers(ctx) { #{} }
fn build_url(ctx) { "" }
fn parse_response(resp) { #{} }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(err) { #{} }
"#;

/// Script missing `parse_response`.
pub const SCRIPT_MISSING_PARSE_RESPONSE: &str = r#"
fn build_request(ctx) { #{} }
fn build_headers(ctx) { #{} }
fn build_url(ctx) { "" }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(err) { #{} }
"#;

/// Script with invalid Rhai syntax.
pub const SCRIPT_SYNTAX_ERROR: &str = "fn build_request( { \n";

/// Script that calls oauth::generate_pkce.
pub const SCRIPT_CALLING_PKCE: &str = r#"
fn build_request(ctx) { #{} }
fn build_headers(ctx) { #{} }
fn build_url(ctx) { "" }
fn parse_response(resp) { #{} }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(err) { #{} }
fn make_pkce() { oauth::generate_pkce() }
"#;

/// RAII env-var guard.
pub struct EnvGuard {
    key: &'static str,
    prior: Option<String>,
}

impl EnvGuard {
    pub fn set(key: &'static str, value: &Path) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prior }
    }

    pub fn remove(key: &'static str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, prior }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prior.take() {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

/// RAII CWD guard.
pub struct CwdGuard {
    prior: PathBuf,
}

impl CwdGuard {
    pub fn set(new_cwd: &Path) -> Self {
        let prior = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(new_cwd).expect("set cwd");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// Write `VALID_SCRIPT` to `dir/filename`.
pub fn write_valid_script(dir: &Path, filename: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, VALID_SCRIPT).expect("write script");
    path
}

/// Build a minimal valid ProviderConfig JSON value with bearer auth.
pub fn minimal_cfg(name: &str, script_rel: &str) -> Value {
    json!({
        "name": name,
        "display_name": "My LLM",
        "base_url": "https://api.example.com",
        "script": script_rel,
        "auth": { "type": "bearer", "env_var": "MY_KEY" },
        "models": { "smart": { "id": "model-smart-v2" } }
    })
}

/// Write a JSON config file to `path`.
pub fn write_cfg(path: &Path, cfg: &Value) {
    fs::write(path, serde_json::to_string_pretty(cfg).unwrap()).unwrap();
}

/// Build cfg + script pair in tmp. Returns (cfg_path, script_filename).
pub fn cfg_with_script(tmp: &Path, name: &str, json_filename: &str) -> PathBuf {
    let script = write_valid_script(tmp, "p.rhai");
    let cfg = minimal_cfg(name, &script.file_name().unwrap().to_string_lossy());
    let cfg_path = tmp.join(json_filename);
    write_cfg(&cfg_path, &cfg);
    cfg_path
}
