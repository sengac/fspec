#![cfg(not(feature = "noop"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/compaction-post-inject-loading-state.feature
//!
//! This test file validates: After inject_summary ends compaction,
//! isLoading not set while agent loop continues running.
//!
//! Tests verify:
//! - on_injected callback emits SessionStateChange(Running) before CompactionComplete
//! - Done handler keeps status Running when pending_dag_content has content

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

// ============================================================================
// Scenario 1: on_injected emits SessionStateChange Running before CompactionComplete
// ============================================================================

/// Tests the REAL production emission pipeline. CMPCT-038 moved the
/// `CompactionComplete` emission from the on_injected closure (which only
/// knows the summary size) to `apply_pending_dag_and_emit` (which knows the
/// recalculated post-injection total). The ordering property is unchanged:
/// SessionStateChange(Running) is emitted BEFORE CompactionComplete.
#[test]
fn test_on_injected_callback_emits_running_before_compaction_complete() {
    // @step Given the agent loop is processing a compaction instruction
    let compaction_flag = Arc::new(AtomicBool::new(true));
    let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // Track the emission order by recording chunk type names.
    let emissions: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let emissions_for_emit = emissions.clone();

    // @step And the Rust session status is Running from CompactionContinuing
    // Build a recording emit closure — same signature as handle_output.
    let record_emit = move |chunk: codelet_napi::StreamChunk| {
        let type_name = match &chunk {
            codelet_napi::StreamChunk::SessionStateChange { state } => {
                format!("SessionStateChange({:?})", state)
            }
            codelet_napi::StreamChunk::CompactionComplete { .. } => {
                "CompactionComplete".to_string()
            }
            other => format!("{:?}", other),
        };
        emissions_for_emit.lock().unwrap().push(type_name);
    };

    // CMPCT-038: on_injected is non-emitting in production — it only clears
    // the compaction progress spinner. Track that it fired.
    let on_injected_fired = Arc::new(AtomicBool::new(false));
    let fired = on_injected_fired.clone();
    let on_injected: codelet_napi::inject_summary_handler::OnInjectedCallback =
        Arc::new(move |_injected_tokens: u32| {
            fired.store(true, Ordering::SeqCst);
        });

    let handler = codelet_napi::inject_summary_handler::create_handler(
        pending_dag.clone(),
        200_000,
        compaction_flag.clone(),
        Some(on_injected),
    );

    // @step When the inject_summary handler fires the on_injected callback
    let result = handler(
        uuid::Uuid::new_v4(),
        "# D2: Architecture\n- JWT auth\n# D1: Arc\n- Building login".to_string(),
    );
    assert!(result.is_ok(), "inject_summary should succeed");
    assert!(
        on_injected_fired.load(Ordering::SeqCst),
        "on_injected must fire when the DAG is stored"
    );
    assert!(
        emissions.lock().unwrap().is_empty(),
        "on_injected must NOT emit any chunks (CMPCT-038 — emission happens at the apply site)"
    );

    // Production then applies the DAG after the stream — same call as the
    // agent_loop apply site (apply_pending_dag_and_emit).
    let pre_compaction_tokens = Arc::new(AtomicU32::new(50_000));
    let provider_manager = codelet_providers::ProviderManager::new()
        .or_else(|_| codelet_providers::ProviderManager::with_provider("gemini"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("zai"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("claude"))
        .expect("Need at least one API key for tests");
    let mut session = codelet_cli::session::Session::from_provider_manager(provider_manager);
    let applied = codelet_napi::inject_summary_handler::apply_pending_dag_and_emit(
        &mut session,
        &pending_dag,
        pre_compaction_tokens.load(Ordering::Acquire),
        &record_emit,
    );
    assert!(applied.is_some(), "DAG must be applied");

    // @step Then a SessionStateChange with state Running must be emitted before CompactionComplete
    let emitted = emissions.lock().unwrap();
    assert!(
        emitted.len() >= 2,
        "apply_pending_dag_and_emit must emit at least 2 events, got: {:?}",
        emitted
    );

    let running_idx = emitted
        .iter()
        .position(|e| e == "SessionStateChange(Running)")
        .expect("Must emit SessionStateChange(Running)");
    let complete_idx = emitted
        .iter()
        .position(|e| e == "CompactionComplete")
        .expect("Must emit CompactionComplete");

    assert!(
        running_idx < complete_idx,
        "SessionStateChange(Running) at index {} must precede CompactionComplete at index {}. Emissions: {:?}",
        running_idx, complete_idx, *emitted
    );

    // @step And the Rust session status must remain Running after CompactionComplete is sent
    // Verify: compaction_in_progress is false (inject_summary cleared it) but
    // no Done/Idle has been emitted — only the agent_loop sets Idle after apply_pending_dag.
    assert!(
        !compaction_flag.load(Ordering::Relaxed),
        "compaction_in_progress should be cleared by inject_summary"
    );
    assert!(
        !emitted
            .iter()
            .any(|e| e.contains("Idle") || e.contains("Done")),
        "the emission pipeline must NOT emit Idle or Done. Emissions: {:?}",
        *emitted
    );
}

/// Verify emit_post_injection_events produces correct CompactionComplete
/// metrics including compression ratio.
#[test]
fn test_emit_post_injection_events_metrics() {
    // @step Given pre-compaction tokens are stored
    let chunks: Arc<Mutex<Vec<codelet_napi::StreamChunk>>> = Arc::new(Mutex::new(Vec::new()));
    let chunks_clone = chunks.clone();

    let record = move |chunk: codelet_napi::StreamChunk| {
        chunks_clone.lock().unwrap().push(chunk);
    };

    // @step When emit_post_injection_events is called with known token counts
    codelet_napi::inject_summary_handler::emit_post_injection_events(
        &record, 40_000, // original
        10_000, // compacted
    );

    // @step Then exactly 2 chunks are emitted in order
    let emitted = chunks.lock().unwrap();
    assert_eq!(emitted.len(), 2);

    // First must be SessionStateChange(Running)
    match &emitted[0] {
        codelet_napi::StreamChunk::SessionStateChange { state } => {
            assert_eq!(*state, codelet_napi::SessionState::Running);
        }
        other => panic!(
            "First emission must be SessionStateChange(Running), got: {:?}",
            other
        ),
    }

    // Second must be CompactionComplete with correct token counts and ratio
    match &emitted[1] {
        codelet_napi::StreamChunk::CompactionComplete { compaction_result } => {
            assert_eq!(compaction_result.original_tokens, 40_000);
            assert_eq!(compaction_result.compacted_tokens, 10_000);
            assert!(
                compaction_result.compression_ratio > 0.0,
                "Compression ratio should be positive, got: {}",
                compaction_result.compression_ratio
            );
        }
        other => panic!(
            "Second emission must be CompactionComplete, got: {:?}",
            other
        ),
    }
}

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
    let should_set_idle = codelet_napi::inject_summary_handler::should_idle_on_done(
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
            codelet_napi::inject_summary_handler::should_idle_on_done(&compaction, &dag);

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
    // @step Given compaction_in_progress is true (compaction was started)
    let compaction_flag = Arc::new(AtomicBool::new(true));
    let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // @step And the agent fails without calling inject_summary (no DAG stored)
    assert!(
        pending_dag.lock().unwrap().is_none(),
        "No DAG should be stored when agent fails"
    );

    // @step When the agent_loop cleanup runs after stream completes
    // Replicate the exact cleanup code from agent_loop:
    let was_compacting = compaction_flag.swap(false, Ordering::SeqCst);

    // @step Then compaction_in_progress must be cleared to false
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
    let should_idle =
        codelet_napi::inject_summary_handler::should_idle_on_done(&compaction_flag, &pending_dag);
    assert!(
        should_idle,
        "Agent loop should set Idle when no DAG is pending after error"
    );
}
