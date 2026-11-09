#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;

use manager::sanitize_headers;

use serial_test::serial;

use uuid::Uuid;

fn setup_test_data_dir() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    crate::set_data_directory(temp_dir.path().to_path_buf()).expect("Failed to set data directory");
    temp_dir
}

// ======================================================================
// BUG-134: Per-session debug capture tests
// Feature: spec/features/debug-capture-per-session.feature
// ======================================================================

// ======================================================================
// BUG-134: Tests for DebugStateChange stream event and NAPI integration
// These tests verify the NAPI layer via source code inspection since
// the NAPI runtime is not available in unit tests.
// ======================================================================

// ======================================================================
// BUG-fix: session.start emission must record real provider/model values
//
// Regression test for the user-visible bug where a `/debug` toggle followed
// by an immediate exit produced a `session.start` event containing
// `provider: "unknown"` and `model: "unknown"` even though the TUI had
// already resolved the real provider/model for the session.
//
// Root cause: `start_capture()` emitted `session.start` synchronously before
// the NAPI layer had a chance to call `set_session_metadata`. The fix defers
// `session.start` emission until metadata arrives (with a stop-time fallback).
// ======================================================================

fn read_events(path: &std::path::Path) -> Vec<serde_json::Value> {
    let content = std::fs::read_to_string(path).expect("read session file");
    content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("parse jsonl"))
        .collect()
}

mod part_1;
mod part_2;
mod part_3;
