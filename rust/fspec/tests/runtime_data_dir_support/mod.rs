//! BUG-417: the RPC-070/RPC-073 runtime scenarios create sessions in a
//! test-owned data directory, never the developer's.
//!
//! One temp directory per test binary (BUG-397: a per-test tempdir would be
//! deleted while sibling tests still point the process-global data directory
//! at it), seeded with an offline models.dev catalog, and an explicit default
//! model so session creation is deterministic. Callers serialize with
//! `#[serial_test::serial]` because the data directory is process-global.

use std::sync::{Arc, OnceLock};

use codelet_sessions::SessionManager;

/// The default model every runtime scenario creates its session with.
pub const DEFAULT_MODEL: &str = "anthropic/claude-opus-4-5";

/// Trimmed offline models.dev catalog shared with the sessions tests.
const MODELS_FIXTURE: &str = include_str!("../../../sessions/tests/fixtures/prov101_models.json");

static DATA_DIR: OnceLock<Result<tempfile::TempDir, String>> = OnceLock::new();

fn seeded_data_dir() -> Result<tempfile::TempDir, String> {
    let dir = tempfile::tempdir().map_err(|error| format!("create data dir: {error}"))?;
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&cache).map_err(|error| format!("create cache dir: {error}"))?;
    std::fs::write(cache.join("models.json"), MODELS_FIXTURE)
        .map_err(|error| format!("seed models.json: {error}"))?;
    Ok(dir)
}

/// A session manager rooted in the binary's seeded data directory with
/// [`DEFAULT_MODEL`] as its default model.
pub fn seeded_manager() -> Result<Arc<SessionManager>, String> {
    let dir = DATA_DIR
        .get_or_init(seeded_data_dir)
        .as_ref()
        .map_err(Clone::clone)?;
    codelet_common::set_data_directory(dir.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    // Offline credential detection only; no request leaves the process.
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-bug417-offline");
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    manager.set_default_model(DEFAULT_MODEL)?;
    if manager.get_default_model().as_deref() != Some(DEFAULT_MODEL) {
        return Err(format!("the seeded manager did not adopt {DEFAULT_MODEL}"));
    }
    Ok(manager)
}
