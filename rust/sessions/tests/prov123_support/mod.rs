//! Shared hermetic test setup for the PROV-123 active-session-selection
//! integration tests (`prov123_active_selection_updates_default.rs`).
//!
//! Extracted into a sibling module so the single feature ↔ single test-file
//! mapping is preserved while the test file itself stays under the 300-line
//! limit (PROV-120/121 precedent). A sub-directory module under `tests/` is NOT
//! compiled as its own test binary, so this introduces no extra target.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Mutex;

use serde_json::Value;

/// Serializes every test that swaps the process-global data directory and the
/// `FSPEC_USER_DIR` env var, so a parallel test cannot observe another test's
/// redirect or persisted files (PROV-118/119 precedent).
pub static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

#[path = "../profile_session_support/mod.rs"]
mod profile_session_support;
pub use profile_session_support::{manager_with_seeded_cache, restore_user_dir, seed_qwen_profile};

/// Read `default-model.json` from `dir` as a JSON value, or `None` when absent.
pub fn read_default_model_json(dir: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(dir.join("default-model.json")).ok()?;
    serde_json::from_str(&content).ok()
}

/// Read `fspec-config.json` from `dir` as a JSON value, or `None` when absent.
pub fn read_config(dir: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(dir.join("fspec-config.json")).ok()?;
    serde_json::from_str(&content).ok()
}
