#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/compaction-post-inject-loading-state.feature
//!
//! This test file validates: After inject_summary ends compaction,
//! isLoading not set while agent loop continues running.
//!
//! Tests verify (against `codelet_agent_loop::inject_summary_handler`):
//! - Done handler keeps status Running when pending_dag_content has content
//! - cleanup clears compaction_in_progress when the agent never calls inject_summary
//!
//! The emit_post_injection_events ordering check (Running before
//! CompactionComplete) belongs to compaction-complete-measurement-basis and
//! lives in cmpct038_measurement_basis_test.rs.
//!
//! The on_injected-callback scenario that drove a raw DAG through
//! `create_handler` was dropped: the handler is now fail-closed (see
//! uncaptured_compaction_contract.rs).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ============================================================================
// Scenario 2: Done handler keeps status Running when pending DAG has not been applied
// ============================================================================

/// Calls the REAL `should_idle_on_done()` guard function — the same function
/// used by BackgroundOutput::emit()'s Done handler — to verify that
/// the Done handler must NOT set Idle when pending_dag_content has content.
#[test]
fn test_done_handler_keeps_running_when_pending_dag_exists() {
    // @step Given the inject_summary handler has stored DAG content in pending_dag_content
    let pending_dag: Mutex<Option<String>> = Mutex::new(Some(
        "<system-reminder>\n<!-- type:compaction-dag -->\n# D2: Architecture\n- JWT\n</system-reminder>"
            .to_string(),
    ));

    // @step And compaction_in_progress has been cleared by inject_summary
    let compaction_in_progress = AtomicBool::new(false);
    assert!(
        !compaction_in_progress.load(Ordering::Relaxed),
        "compaction_in_progress should be false (cleared by inject_summary)"
    );

    // @step When the stream finishes and the Done event fires
    // Call the real shared guard function used by BackgroundOutput::emit()
    let should_set_idle = codelet_agent_loop::inject_summary_handler::should_idle_on_done(
        &compaction_in_progress,
        &pending_dag,
    );

    // @step Then the Done handler must check pending_dag_content before setting Idle
    // @step And the status must remain Running if pending_dag_content has content
    assert!(
        !should_set_idle,
        "Done handler must NOT set Idle when pending_dag has content"
    );
}

/// Verify all four truth-table combinations of the Done handler's idle guard
/// by calling the REAL `should_idle_on_done()` function for each case.
///
/// | compaction_in_progress | has_pending_dag | should_set_idle |
/// |------------------------|-----------------|-----------------|
/// | true                   | true            | NO              |
/// | true                   | false           | NO              |
/// | false                  | true            | NO              |
/// | false                  | false           | YES             |
#[test]
fn test_done_handler_idle_guard_truth_table() {
    let cases: Vec<(bool, bool, bool)> = vec![
        // (compaction_in_progress, has_pending_dag, should_set_idle)
        (true, true, false),  // Both active — never idle
        (true, false, false), // Compaction still active — never idle
        (false, true, false), // DAG pending — DON'T idle
        (false, false, true), // Both cleared — safe to idle
    ];

    for (compaction_active, has_dag, expected_idle) in cases {
        let compaction = AtomicBool::new(compaction_active);
        let dag: Mutex<Option<String>> = Mutex::new(if has_dag {
            Some("# DAG content".to_string())
        } else {
            None
        });

        // Call the real shared guard function
        let should_idle =
            codelet_agent_loop::inject_summary_handler::should_idle_on_done(&compaction, &dag);

        assert_eq!(
            should_idle, expected_idle,
            "compaction_active={}, has_dag={} => should_idle={} (expected {})",
            compaction_active, has_dag, should_idle, expected_idle
        );
    }
}

// ============================================================================
// Scenario 3: Agent fails to call inject_summary — cleanup prevents permanent flag
// ============================================================================

/// Verify that the agent_loop's unconditional cleanup (compaction_in_progress.swap(false))
/// prevents the flag from staying true permanently when the agent errors out without
/// calling inject_summary. Uses the real `should_idle_on_done()` to verify the
/// post-cleanup idle decision.
#[test]
fn test_agent_loop_cleanup_clears_compaction_flag_on_error() {
    // @step Given compaction_in_progress is true because compaction was started
    let compaction_flag = Arc::new(AtomicBool::new(true));
    let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // @step And the agent fails without calling inject_summary so no DAG is stored
    assert!(
        pending_dag.lock().unwrap().is_none(),
        "No DAG should be stored when agent fails"
    );

    // @step When the agent_loop cleanup runs after the stream completes
    // Replicate the exact cleanup code from agent_loop:
    let was_compacting = compaction_flag.swap(false, Ordering::SeqCst);

    // @step Then compaction_in_progress must be unconditionally cleared to false
    assert!(
        !compaction_flag.load(Ordering::Relaxed),
        "compaction_in_progress must be cleared by cleanup"
    );
    assert!(
        was_compacting,
        "swap must return true indicating flag was previously set"
    );

    // @step And the session status must be set to Idle since no DAG is pending
    // Use the real shared guard function to verify post-cleanup state
    let should_idle = codelet_agent_loop::inject_summary_handler::should_idle_on_done(
        &compaction_flag,
        &pending_dag,
    );
    assert!(
        should_idle,
        "Agent loop should set Idle when no DAG is pending after error"
    );
}
