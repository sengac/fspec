//! TUI-109 — the Checkpoints LoadingDialog counter row folds per-item
//! checkpoint-enumeration progress.
//!
//! Feature: spec/features/stream-per-item-progress-idx-total-for-checkpoint-enumeration-into-the-shared-loadingdialog-counter-row.feature
//!
//! The TUI half: the App fold renders `(loaded/…)` → `(N/total)`, bootstrap
//! spawns the `checkpoints_progress_rx()` subscriber that forwards frames
//! onto the bus, a late done frame after `CheckpointsLoaded` is stale-dropped,
//! and a transport with no frames degrades to spinner + stage label. The git
//! and rpc halves live in tui109_checkpoint_enumeration_stream.rs (BUG-355
//! split, one test file per feature).

use codelet_fspec_tui::{Action, ViewMode};
use codelet_test_helpers::{OrFail, TestResult};

mod common;

mod tui109_support;
use tui109_support::{
    checkpoint_info, checkpoint_list, fresh_app, progress, render_text, wait_for_action,
};

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The loading dialog counter advances from (0/…) to (N/total)
// before the list folds
// ─────────────────────────────────────────────────────────────────────────

/// TUI fold half — the counter row renders `(loaded/…)` while the total
/// is unknown, climbs, and shows `(N/total)` just before the list folds.
#[tokio::test]
async fn tui_counter_advances_from_pending_total_to_final_before_list_folds() -> TestResult {
    // @step Given a Checkpoints view whose list load is in flight on a transport that streams progress events
    let (mut app, mock) = fresh_app();
    app.bootstrap().await.or_fail("bootstrap")?;
    app.dispatch(Action::OpenCheckpointsView);
    assert!(app.is_view_loading(), "list load in flight");

    // @step When the transport emits CheckpointsProgress events (1/…), (47/…), then (150/150) with done=true before the list result folds
    mock.push_checkpoints_progress(progress(1, 0, false));
    let action = wait_for_action(&mut app, |a| matches!(a, Action::CheckpointsProgress(_)))
        .await
        .or_fail("CheckpointsProgress on the action bus")?;
    assert!(
        matches!(action, Action::CheckpointsProgress(p) if (p.loaded, p.total) == (1, 0)),
        "the pending-total frame reaches the bus"
    );

    // @step Then the loading dialog counter row shows (1/…) while the total is still unknown
    app.dispatch(Action::CheckpointsProgress(progress(1, 0, false)));
    assert!(
        app.is_view_loading(),
        "still loading after a pending-total frame"
    );
    let dialog = app.navigator_checkpoints_loading_dialog();
    assert!(
        dialog.progress.is_some(),
        "progress slot fed while the list stage is in flight"
    );
    assert!(
        render_text(&dialog).contains("(1/…)"),
        "pending total renders as (1/…)"
    );

    // @step And the counter row climbs through the intermediate values as each progress event folds
    app.dispatch(Action::CheckpointsProgress(progress(47, 0, false)));
    assert!(
        render_text(&app.navigator_checkpoints_loading_dialog()).contains("(47/…)"),
        "counter climbed"
    );

    // @step And the counter row shows the final (150/150) just before the list appears
    app.dispatch(Action::CheckpointsProgress(progress(150, 150, true)));
    assert!(
        render_text(&app.navigator_checkpoints_loading_dialog()).contains("(150/150)"),
        "final (N/total) before the list folds"
    );

    // @step When the CheckpointsLoaded fold arrives with 150 checkpoints
    app.dispatch(Action::CheckpointsLoaded(checkpoint_list(150)));
    // Settle the files stage (the cascade continues onto the first
    // selected checkpoint) so the dialog can dismiss.
    app.dispatch(Action::CheckpointFilesLoaded {
        work_unit_id: "AUTH-001".into(),
        name: "cp-000".into(),
        files: vec![],
    });

    // @step Then the list renders and the loading dialog is dismissed
    assert!(!app.is_view_loading(), "list flushed → idle");
    assert_eq!(app.navigator_checkpoints_len(), 150);
    Ok(())
}

/// Subscriber half — bootstrap spawns a subscriber on
/// `checkpoints_progress_rx()` that forwards every frame onto the bus
/// (supporting test; the fold itself is pinned above).
#[tokio::test]
async fn tui_bootstrap_spawns_checkpoints_progress_subscriber_forwarding_to_the_bus() -> TestResult
{
    let (mut app, mock) = fresh_app();
    app.bootstrap().await.or_fail("bootstrap")?;
    // The full bootstrap subscriber set includes TUI-109's
    // checkpoints_progress_rx subscriber (BUG-351: one shared total).
    assert_eq!(
        app.subscriber_task_count(),
        common::BOOTSTRAP_SUBSCRIBER_TASKS,
        "bootstrap must spawn the checkpoints-progress subscriber task"
    );

    for frame in [
        progress(1, 0, false),
        progress(47, 0, false),
        progress(150, 150, true),
    ] {
        mock.push_checkpoints_progress(frame);
        let action = wait_for_action(&mut app, |a| matches!(a, Action::CheckpointsProgress(_)))
            .await
            .or_fail("CheckpointsProgress forwarded by the subscriber")?;
        assert!(
            matches!(&action, Action::CheckpointsProgress(p) if *p == frame),
            "the subscriber forwards {frame:?} unchanged, got {action:?}"
        );
    }
    assert!(
        app.active_view() == ViewMode::Board,
        "view untouched by the subscriber"
    );
    Ok(())
}

/// TUI fold half of the capped enumeration (the rpc half is in
/// tui109_checkpoint_enumeration_stream.rs) — the dialog renders whatever
/// the wire carries, so a capped done frame paints `(200/250)`.
#[test]
fn tui_capped_progress_frame_renders_200_of_250() -> TestResult {
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    app.dispatch(Action::CheckpointsProgress(progress(200, 250, true)));
    assert!(
        render_text(&app.navigator_checkpoints_loading_dialog()).contains("(200/250)"),
        "counter shows loaded=cap, total=full enumeration"
    );
    app.dispatch(Action::CheckpointsLoaded(checkpoint_list(200)));
    assert_eq!(app.navigator_checkpoints_len(), 200);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A late progress event after CheckpointsLoaded is stale-dropped
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tui_late_done_event_after_checkpoints_loaded_is_stale_dropped() -> TestResult {
    // @step Given a Checkpoints view whose list has already flushed via the CheckpointsLoaded fold
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    app.dispatch(Action::CheckpointsLoaded(vec![checkpoint_info(
        "AUTH-001", "baseline",
    )]));
    // The list stage has flushed (the cascade may continue onto the
    // files stage — that is the list presentation state).
    assert!(
        app.navigator_checkpoints_list_loaded(),
        "list stage flushed"
    );

    // @step When a late CheckpointsProgress event with done=true arrives after the fold
    app.dispatch(Action::CheckpointsProgress(progress(1, 1, true)));

    // @step Then the view stays in the list presentation state and is not loading
    assert!(
        app.navigator_checkpoints_list_loaded(),
        "late event must not re-open the list load"
    );
    assert_eq!(app.navigator_checkpoints_len(), 1);

    // @step And the loading dialog is not re-painted - progress events after the list fold are ignored
    let dialog = app.navigator_checkpoints_loading_dialog();
    assert!(
        dialog.progress.is_none(),
        "progress slot must NOT be re-fed after the list fold"
    );
    assert!(
        !render_text(&dialog).contains('/'),
        "no counter row painted for a stale frame"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A transport that does not forward progress degrades to
// spinner plus stage label
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tui_no_progress_transport_degrades_to_spinner_and_stage_label() -> TestResult {
    // @step Given a Checkpoints view whose list load is in flight on a transport that never emits progress events
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenCheckpointsView);
    assert!(app.is_view_loading(), "list load in flight");

    // @step When the loading dialog is painted while the list load is still in flight
    let text = render_text(&app.navigator_checkpoints_loading_dialog());

    // @step Then the dialog shows the spinner and the stage label "Loading checkpoint list…"
    assert!(text.contains('⠋'), "spinner glyph painted");
    assert!(
        text.contains("Loading checkpoint list…"),
        "stage label painted"
    );

    // @step And no counter row is painted - the TUI-107 behavior is preserved with no timeout or extra logic
    assert!(
        !text.contains("(/") && !text.contains("/…"),
        "no counter row without progress events"
    );
    Ok(())
}
