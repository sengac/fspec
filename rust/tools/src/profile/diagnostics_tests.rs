//! AMGR-018 — AgentManager profile diagnostics rewrite tests
//!
//! Feature: spec/features/agent-manager-profile-diagnostics.feature
//!
//! These tests validate the acceptance criteria for the profile-action rewrite:
//! noise-frame attribution, per-thread sample buckets, hot-stack aggregation,
//! sampling-quality reports, focus filtering, and backward compatibility of
//! `ProfileResult`. All tests are expected to FAIL during the red phase
//! because `attribute_samples` is stubbed with `todo!()`.
//!
//! Test runner: `cargo test -p codelet-tools profile::diagnostics_tests`

use super::attribution::{
    attribute_samples, AttributionOutput, FrameInfo, SampleStack, NOISE_FRAME_PREFIXES,
};
use super::result::{ProfileResult, SamplingReport};

// --------------------------------------------------------------------------
// Shared synthetic-sample helpers
// --------------------------------------------------------------------------

/// Build a single-symbol resolved physical frame with file+line.
fn rust_frame(symbol: &str, file: &str, line: u32) -> Vec<FrameInfo> {
    vec![FrameInfo {
        symbol: symbol.to_string(),
        file: Some(file.to_string()),
        line: Some(line),
    }]
}

/// Build a single-symbol unresolved physical frame (no file / line).
fn noise_frame(symbol: &str) -> Vec<FrameInfo> {
    vec![FrameInfo {
        symbol: symbol.to_string(),
        file: None,
        line: None,
    }]
}

/// Build a physical frame with four inlined symbols (leaf-most → leaf).
fn inlined_frame(outermost: &str, inner_a: &str, inner_b: &str, inner_c: &str) -> Vec<FrameInfo> {
    vec![
        FrameInfo {
            symbol: outermost.to_string(),
            file: Some("rust/tools/src/bridge_relay.rs".to_string()),
            line: Some(42),
        },
        FrameInfo {
            symbol: inner_a.to_string(),
            file: Some("rust/tools/src/bridge_relay.rs".to_string()),
            line: Some(43),
        },
        FrameInfo {
            symbol: inner_b.to_string(),
            file: Some("rust/tools/src/bridge_relay.rs".to_string()),
            line: Some(44),
        },
        FrameInfo {
            symbol: inner_c.to_string(),
            file: Some("rust/tools/src/bridge_relay.rs".to_string()),
            line: Some(45),
        },
    ]
}

/// Helper: construct an AMGR-017-shaped `ProfileResult` stub with default
/// AMGR-018 fields to exercise the serde round-trip in scenario 7.
fn empty_profile_result() -> ProfileResult {
    ProfileResult {
        duration_secs: 1,
        started_at: "2026-04-08T00:00:00Z".to_string(),
        ended_at: "2026-04-08T00:00:01Z".to_string(),
        process: super::result::ProcessReport {
            pid: 1,
            rss_bytes_start: 1,
            rss_bytes_end: 1,
            total_threads_start: 1,
            total_threads_end: 1,
        },
        runtime: super::result::RuntimeReport {
            worker_threads: 1,
            alive_tokio_tasks_start: None,
            alive_tokio_tasks_end: None,
        },
        scopes_by_calls: Vec::new(),
        scopes_by_self_ms: Vec::new(),
        channels: Vec::new(),
        samples_by_thread: Vec::new(),
        hot_stacks: Vec::new(),
        sampling: SamplingReport::default(),
    }
}

// ==========================================================================
// Scenario 1: Attribute runaway subordinate_forwarding recv loop to correct
//             thread and function
// ==========================================================================

// ==========================================================================
// Scenario 2: Detect stripped build and report debug info unavailable
// ==========================================================================

// ==========================================================================
// Scenario 3: Filter samples by focus substring to narrow to a single call chain
// ==========================================================================

// ==========================================================================
// Scenario 4: Walk leaf to root skipping noise frames for attribution
// ==========================================================================

// ==========================================================================
// Scenario 5: Credit only the outermost inlined symbol per physical frame
// ==========================================================================

// ==========================================================================
// Scenario 6: Report cpu_cores_consumed from total sample count
// ==========================================================================

// ==========================================================================
// Scenario 7: Preserve backward compatibility of existing ProfileResult fields
// ==========================================================================

#[path = "diagnostics_tests/attribute_runaway_subordinate_forwarding.rs"]
mod attribute_runaway_subordinate_forwarding;
#[path = "diagnostics_tests/walk_leaf_to_root.rs"]
mod walk_leaf_to_root;
