//! Feature: spec/features/compaction-diagnostic-log-level-hygiene.feature
//!
//! CMPCT-049: the compaction-status diagnostic markers were added to debug
//! the thinking-vs-compacting indicator. This source-shape guard pins the
//! log-level contract: routine per-event propagation lines (channel hops,
//! progress ticks, per-turn bookkeeping) are `tracing::debug!`, while the
//! LIFECYCLE decisions (status transitions, the TUI display-mode flip, the
//! rare compaction-active Done branch, the compaction sub-agent audit trail)
//! stay `tracing::info!` — a normal session must not flood the log at the
//! default level, and the thinking-vs-compacting trace must remain visible.
//!
//! The markers live in four crates; this file reads them via relative
//! paths from the fspec-tui manifest dir (the established cross-crate
//! source-shape pattern). BUG-373: a module split into a submodule
//! directory is read as its file plus every `.rs` file in that directory.

use codelet_test_helpers::TestResult;
use std::error::Error;
use std::path::{Path, PathBuf};

/// The repo's `rust/` directory.
fn rust_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{} has no parent", manifest.display()))?;
    Ok(root.to_path_buf())
}

fn read(path: &Path) -> Result<String, Box<dyn Error>> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()).into())
}

/// A module's source (crate-relative, e.g. "agent-loop/src/agent_loop.rs"):
/// the file itself plus every `.rs` file of its submodule directory
/// (`agent-loop/src/agent_loop/`), when it has one.
fn repo_module(rel: &str) -> Result<String, Box<dyn Error>> {
    let file = rust_root()?.join(rel);
    let mut source = read(&file)?;
    let directory = file.with_extension("");
    if directory.is_dir() {
        let mut children = std::fs::read_dir(&directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.retain(|path| path.extension().is_some_and(|extension| extension == "rs"));
        children.sort();
        for child in children {
            source.push('\n');
            source.push_str(&read(&child)?);
        }
    }
    Ok(source)
}

/// The `window` bytes before `marker` (moved forward to a char boundary).
fn window_before<'a>(src: &'a str, marker: &str, window: usize) -> Result<&'a str, Box<dyn Error>> {
    let at = src
        .find(marker)
        .ok_or_else(|| format!("CMPCT-049: marker '{marker}' not found in source"))?;
    let mut start = at.saturating_sub(window);
    while !src.is_char_boundary(start) {
        start += 1;
    }
    src.get(start..at)
        .ok_or_else(|| format!("CMPCT-049: no window before '{marker}'").into())
}

/// The tracing macro call that emits `marker` must be `want` (e.g.
/// "tracing::debug!("). The marker string is the LAST argument of the
/// macro call, so the macro name sits up to `window` bytes before it.
fn assert_log_level(src: &str, file: &str, marker: &str, want: &str, window: usize) -> TestResult {
    let before = window_before(src, marker, window)?;
    if !before.contains(want) {
        return Err(format!(
            "CMPCT-049: {file} must log '{marker}' at {want}, got window before the marker: {before}"
        )
        .into());
    }
    Ok(())
}

// ============================================================================
// Scenario: Routine propagation lines are logged at DEBUG
// ============================================================================

#[test]
fn routine_propagation_lines_are_logged_at_debug() -> TestResult {
    // @step Given the compaction diagnostic markers exist in the agent loop, background output, session, and TUI layers
    let agent_loop = repo_module("agent-loop/src/agent_loop.rs")?;
    let background_output = repo_module("agent-loop/src/background_output.rs")?;
    let background_session = repo_module("sessions/src/background_session.rs")?;
    let tui_bootstrap = repo_module("fspec-tui/src/app/bootstrap.rs")?;
    let tui_dispatch = repo_module("fspec-tui/src/app/dispatch_stream_chunks.rs")?;
    let debug = "tracing::debug!(";

    // @step When the routine per-event lines are emitted
    // @step Then the agent-loop turn START and turn END check lines are tracing::debug!
    for marker in [
        "[compaction-status] turn START — status → Running (thinking mode)",
        "[compaction-status] turn END — compaction flag was set → clearing + forcing Idle",
        "[compaction-status] turn END — watchdog retry pending; keeping compaction flag",
    ] {
        assert_log_level(&agent_loop, "agent_loop", marker, debug, 400)?;
    }

    // @step And the background_output routine Done arm ("no pending compaction") is tracing::debug!
    assert_log_level(
        &background_output,
        "background_output.rs",
        "[compaction-status] Done: no pending compaction → status set to Idle",
        debug,
        400,
    )?;

    // @step And set_compaction_progress is tracing::debug! and update_compaction_progress logs through it
    assert_log_level(
        &background_session,
        "background_session",
        "[compaction-status] set_compaction_progress",
        debug,
        400,
    )?;
    let update = window_before(
        &background_session,
        "self.set_compaction_progress(Some(CompactionProgress {",
        200,
    )?;
    if !update.contains("pub fn update_compaction_progress(") {
        return Err(format!(
            "CMPCT-049: update_compaction_progress must log through set_compaction_progress: {update}"
        )
        .into());
    }

    // @step And the TUI bootstrap status-recv and both TUI store-updated lines are tracing::debug!
    assert_log_level(
        &tui_bootstrap,
        "bootstrap.rs",
        "[compaction-status] TUI received status change",
        debug,
        400,
    )?;
    for marker in [
        "[compaction-status] TUI store updated (SessionStateChange chunk)",
        "[compaction-status] TUI store updated (push channel)",
    ] {
        assert_log_level(
            &tui_dispatch,
            "dispatch_stream_chunks.rs",
            marker,
            debug,
            400,
        )?;
    }
    Ok(())
}

// ============================================================================
// Scenario: Lifecycle decision lines stay at INFO
// ============================================================================

#[test]
fn lifecycle_decision_lines_stay_at_info() -> TestResult {
    // @step Given the compaction diagnostic markers exist in the agent loop, session, and TUI layers
    let background_session = repo_module("sessions/src/background_session.rs")?;
    let background_output = repo_module("agent-loop/src/background_output.rs")?;
    let animation = repo_module("fspec-tui/src/views/agent/animation.rs")?;
    let handler = repo_module("agent-loop/src/generate_compaction_handler.rs")?;
    let info = "tracing::info!(";

    // @step When the lifecycle decisions are emitted
    // @step Then the set_status transition line is tracing::info!
    let transition = "[compaction-status] set_status transition";
    assert_log_level(
        &background_session,
        "background_session",
        transition,
        info,
        400,
    )?;
    let before = window_before(&background_session, transition, 400)?;
    if before.contains("tracing::debug!(") {
        return Err(
            format!("CMPCT-049: the set_status transition must NOT be DEBUG: {before}").into(),
        );
    }

    // @step And the TUI display-mode flip line is tracing::info!
    assert_log_level(
        &animation,
        "animation.rs",
        "[compaction-status] TUI display decision: spinner shows",
        info,
        400,
    )?;
    // The flip line only fires on change (no per-frame spam): the flip
    // guard field exists on AgentView.
    let agent_view = repo_module("fspec-tui/src/views/agent.rs")?;
    if !agent_view.contains("last_compaction_diag_display") {
        return Err(
            "CMPCT-049: AgentView must carry the flip field last_compaction_diag_display".into(),
        );
    }

    // @step And the rare background_output Done arm branch ("compaction/pending-DAG active") is tracing::info!
    assert_log_level(
        &background_output,
        "background_output.rs",
        "[compaction-status] Done: compaction/pending-DAG active → NOT setting Idle",
        info,
        400,
    )?;

    // @step And the generate-compaction handler ENTER and sub-agent run COMPLETE lines are tracing::info!
    for marker in [
        "[generate-compaction] ENTER",
        "[generate-compaction] sub-agent run COMPLETE",
    ] {
        assert_log_level(
            &handler,
            "generate_compaction_handler.rs",
            marker,
            info,
            300,
        )?;
    }
    Ok(())
}
