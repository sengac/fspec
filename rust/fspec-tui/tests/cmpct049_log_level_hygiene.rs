//! Feature: spec/features/compaction-diagnostic-log-level-hygiene.feature
//!
//! CMPCT-049: the compaction-status diagnostic markers were added to debug
//! the thinking-vs-compacting indicator. This source-shape guard pins the
//! log-level contract: routine per-event propagation lines (channel hops,
//! progress ticks, per-turn bookkeeping) are `tracing::debug!`, while the
//! LIFECYCLE decisions (status transitions, the TUI display-mode flip, the
//! rare compaction-active Done branch) stay `tracing::info!` — a normal
//! session must not flood the log with ~4+ lines per status change at the
//! default level, and the thinking-vs-compacting trace must remain visible.
//!
//! The markers live in four crates; this file reads them via relative
//! paths from the fspec-tui manifest dir (the established cross-crate
//! source-shape pattern).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// Locate a source file (crate-relative, e.g. "agent-loop/src/agent_loop.rs")
/// under the repo's `rust/` directory.
fn repo_source(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_root = manifest.parent().expect("repo rust root");
    std::fs::read_to_string(rust_root.join(rel))
        .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"))
}

/// Find `marker` in `src` and return a window of `before`..`after` chars
/// around it (the window is large enough to contain the tracing macro
/// call that precedes the marker string).
fn window_around<'a>(src: &'a str, marker: &str, before: usize, after: usize) -> &'a str {
    let at = src
        .find(marker)
        .unwrap_or_else(|| panic!("CMPCT-049: marker '{marker}' not found in source"));
    let start = at.saturating_sub(before);
    &src[start..at + after]
}

/// The tracing macro call that emits `marker` must be `want`
/// (e.g. "tracing::debug!("). The marker string is the LAST argument of
/// the macro call, so the macro name sits up to `window` chars BEFORE the
/// marker (after the `macro!( fields…`) — the window spans `window`
/// chars behind the marker's start.
fn assert_log_level(src: &str, file: &str, marker: &str, want: &str, window: usize) {
    let at = src
        .find(marker)
        .unwrap_or_else(|| panic!("CMPCT-049: marker '{marker}' not found in source"));
    let start = at.saturating_sub(window);
    let w = &src[start..at];
    assert!(
        w.contains(want),
        "CMPCT-049: {file} must log '{marker}' at {want}, got window before the marker: {w}"
    );
}

// ============================================================================
// Scenario: Routine propagation lines are logged at DEBUG
// ============================================================================

#[test]
fn routine_propagation_lines_are_logged_at_debug() {
    // @step Given the compaction diagnostic markers exist in the agent loop, background output, session, and TUI layers
    let agent_loop = repo_source("agent-loop/src/agent_loop.rs");
    let background_output = repo_source("agent-loop/src/background_output.rs");
    let background_session = repo_source("sessions/src/background_session.rs");
    let tui_bootstrap = repo_source("fspec-tui/src/app/bootstrap.rs");
    let tui_dispatch = repo_source("fspec-tui/src/app/dispatch_stream_chunks.rs");

    // @step When the routine per-event lines are emitted
    // @step Then the agent-loop turn START and turn END check lines are tracing::debug!
    assert_log_level(
        &agent_loop,
        "agent_loop.rs",
        "[compaction-status] turn START — status → Running (thinking mode)",
        "tracing::debug!(",
        400,
    );
    assert_log_level(
        &agent_loop,
        "agent_loop.rs",
        "[compaction-status] turn END — checking for pending DAG to pin",
        "tracing::debug!(",
        400,
    );
    assert_log_level(
        &agent_loop,
        "agent_loop.rs",
        "[compaction-status] turn END — compaction flag was set → clearing + forcing Idle",
        "tracing::debug!(",
        400,
    );
    assert_log_level(
        &agent_loop,
        "agent_loop.rs",
        "[compaction-status] turn END — watchdog retry pending; keeping compaction flag",
        "tracing::debug!(",
        400,
    );

    // @step And the background_output routine Done arm ("no pending compaction") is tracing::debug!
    assert_log_level(
        &background_output,
        "background_output.rs",
        "[compaction-status] Done: no pending compaction → status set to Idle",
        "tracing::debug!(",
        400,
    );

    // @step And set_compaction_progress and update_compaction_progress are tracing::debug!
    assert_log_level(
        &background_session,
        "background_session.rs",
        "[compaction-status] set_compaction_progress",
        "tracing::debug!(",
        400,
    );
    assert_log_level(
        &background_session,
        "background_session.rs",
        "[compaction-status] update_compaction_progress",
        "tracing::debug!(",
        400,
    );

    // @step And the TUI bootstrap status-recv and both TUI store-updated lines are tracing::debug!
    assert_log_level(
        &tui_bootstrap,
        "bootstrap.rs",
        "[compaction-status] TUI received status change",
        "tracing::debug!(",
        400,
    );
    assert_log_level(
        &tui_dispatch,
        "dispatch_stream_chunks.rs",
        "[compaction-status] TUI store updated (SessionStateChange chunk)",
        "tracing::debug!(",
        400,
    );
    assert_log_level(
        &tui_dispatch,
        "dispatch_stream_chunks.rs",
        "[compaction-status] TUI store updated (push channel)",
        "tracing::debug!(",
        400,
    );
}

// ============================================================================
// Scenario: Lifecycle decision lines stay at INFO
// ============================================================================

#[test]
fn lifecycle_decision_lines_stay_at_info() {
    // @step Given the compaction diagnostic markers exist in the agent loop, session, and TUI layers
    let background_session = repo_source("sessions/src/background_session.rs");
    let background_output = repo_source("agent-loop/src/background_output.rs");
    let animation = repo_source("fspec-tui/src/views/agent/animation.rs");

    // @step When the lifecycle decisions are emitted
    // @step Then the set_status transition line is tracing::info!
    assert_log_level(
        &background_session,
        "background_session.rs",
        "[compaction-status] set_status transition",
        "tracing::info!(",
        400,
    );
    // …and not at DEBUG.
    let w = window_around(
        &background_session,
        "[compaction-status] set_status transition",
        400,
        0,
    );
    assert!(
        !w.contains("tracing::debug!("),
        "CMPCT-049: the set_status transition must NOT be DEBUG, got window: {w}"
    );

    // @step And the TUI display-mode flip line is tracing::info!
    assert_log_level(
        &animation,
        "animation.rs",
        "[compaction-status] TUI display decision: spinner shows",
        "tracing::info!(",
        400,
    );

    // @step And the rare background_output Done arm branch ("compaction/pending-DAG active") is tracing::info!
    assert_log_level(
        &background_output,
        "background_output.rs",
        "[compaction-status] Done: compaction/pending-DAG active → NOT setting Idle",
        "tracing::info!(",
        400,
    );

    // Rule [6]: the [generate-compaction] handler lifecycle lines stay INFO —
    // compaction is rare and these are the audit trail for the sub-agent run.
    let handler = repo_source("agent-loop/src/generate_compaction_handler.rs");
    assert_log_level(
        &handler,
        "generate_compaction_handler.rs",
        "[generate-compaction] ENTER",
        "tracing::info!(",
        300,
    );
    assert_log_level(
        &handler,
        "generate_compaction_handler.rs",
        "[generate-compaction] sub-agent run COMPLETE",
        "tracing::info!(",
        400,
    );

    // The flip line only fires on change (no per-frame spam): the flip
    // guard field exists on AgentView.
    let agent_view = repo_source("fspec-tui/src/views/agent.rs");
    assert!(
        agent_view.contains("last_compaction_diag_display"),
        "CMPCT-049: AgentView must carry the flip-detection field last_compaction_diag_display"
    );
}
