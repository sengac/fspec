//! Shared offline helpers for the PROV-108 `profile_rpc_surface` integration
//! suite. Kept in a sibling `#[path]` module so the main test file stays under
//! the 300-LoC budget.

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::ProfileDefinition;
use codelet_sessions::SessionManager;
use serde_json::Value;

pub fn make_handle() -> Arc<dyn SessionManagerHandle> {
    Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>
}

/// RAII guard for the process-global `FSPEC_USER_DIR` env var. Captures the
/// prior value on construction and restores it (or removes it if previously
/// unset) on `Drop`, so the suite never leaks state into the process even
/// though the tests are `#[serial]`. Keeps everything OFFLINE (temp dir only).
pub struct EnvGuard {
    prior: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(dir: &Path) -> Self {
        let prior = std::env::var_os("FSPEC_USER_DIR");
        std::env::set_var("FSPEC_USER_DIR", dir);
        Self { prior }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prior {
            Some(value) => std::env::set_var("FSPEC_USER_DIR", value),
            None => std::env::remove_var("FSPEC_USER_DIR"),
        }
    }
}

/// Point `FSPEC_USER_DIR` at `dir` without seeding a config file. The returned
/// guard must be held for the lifetime of the test (binds via `let _env`).
#[must_use]
pub fn point_env(dir: &Path) -> EnvGuard {
    EnvGuard::set(dir)
}

/// Write an `fspec-config.json` into `dir` from the given root JSON and point
/// `FSPEC_USER_DIR` at `dir`. The returned guard must be held for the test.
#[must_use]
pub fn seed_config(dir: &Path, root: Value) -> EnvGuard {
    std::fs::write(
        dir.join("fspec-config.json"),
        serde_json::to_string_pretty(&root).unwrap(),
    )
    .unwrap();
    point_env(dir)
}

pub fn config_path(dir: &Path) -> std::path::PathBuf {
    dir.join("fspec-config.json")
}

pub fn read_root(dir: &Path) -> Value {
    let content = std::fs::read_to_string(config_path(dir)).unwrap();
    serde_json::from_str(&content).unwrap()
}

pub fn read_profile(dir: &Path, name: &str) -> Value {
    read_root(dir)["providers"]["openai"]["profiles"][name].clone()
}

pub fn basic_def(base_url: &str, api_key: &str) -> ProfileDefinition {
    ProfileDefinition {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
        streaming: None,
        auto_continue: None,
    }
}
