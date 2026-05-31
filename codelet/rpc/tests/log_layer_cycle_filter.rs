//! Feature: spec/features/tracing-event-recursion-via-broadcastloglayer-bootstrap-logs-task-feedback-loop.feature
//!
//! LOG-004 regression test: BroadcastLogLayer must NOT broadcast events
//! emitted by `codelet_fspec_tui::app::bootstrap`. The TUI logs_task
//! subscriber re-emits every received LogRecord via `debug!()` for
//! visibility; without this filter, that re-emit re-enters the
//! BroadcastLogLayer and gets broadcast back to the same subscriber,
//! causing infinite recursive amplification (one wrap of escaped quotes
//! per round-trip → 8 GB log files within a day).
//!
//! Both the per-instance [`SingleBroadcastLogLayer`] and the
//! process-global [`BroadcastLogLayer`] share the same skip predicate,
//! so we exercise the per-instance variant here to avoid touching the
//! global senders list (which other tests in this crate / downstream
//! crates may also use concurrently).
//!
//! Workspace lints forbid `expect()` on `Result`; the assertion is
//! expressed via `match` so a failure surfaces a `panic!` with a clear
//! message instead.

#![allow(clippy::panic)]

use codelet_rpc::BroadcastLogLayer;
use codelet_rpc_types::LogRecord;
use tokio::sync::broadcast;
use tracing::dispatcher::{self, Dispatch};
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn build_subscriber(sender: broadcast::Sender<LogRecord>) -> Dispatch {
    Dispatch::new(Registry::default().with(BroadcastLogLayer::new(sender)))
}

#[test]
fn events_targeted_at_tui_bootstrap_are_not_broadcast() {
    // @step Given a BroadcastLogLayer is installed with a registered broadcast sender
    let (tx, mut rx) = broadcast::channel::<LogRecord>(16);
    let dispatch = build_subscriber(tx);

    // @step When a tracing event with target "codelet_fspec_tui::app::bootstrap" is emitted
    dispatcher::with_default(&dispatch, || {
        tracing::event!(
            target: "codelet_fspec_tui::app::bootstrap",
            tracing::Level::DEBUG,
            "rpc log: LogRecord {{ ... }}",
        );
    });

    // @step Then the registered broadcast sender receives zero LogRecord values for that event
    match rx.try_recv() {
        Err(broadcast::error::TryRecvError::Empty) => {}
        other => panic!(
            "BroadcastLogLayer must NOT broadcast events from the TUI bootstrap subscriber, got: {other:?}"
        ),
    }
}

#[test]
fn events_from_non_tui_bootstrap_targets_are_still_broadcast() {
    // @step Given a BroadcastLogLayer is installed with a registered broadcast sender
    let (tx, mut rx) = broadcast::channel::<LogRecord>(16);
    let dispatch = build_subscriber(tx);

    // @step When a tracing event with target "codelet_agent_loop::hooks" is emitted
    dispatcher::with_default(&dispatch, || {
        tracing::event!(
            target: "codelet_agent_loop::hooks",
            tracing::Level::DEBUG,
            "starting agent_loop for session abc",
        );
    });

    // @step Then the registered broadcast sender receives exactly one LogRecord value for that event
    let record = match rx.try_recv() {
        Ok(r) => r,
        Err(e) => panic!("expected one broadcast LogRecord, got: {e:?}"),
    };
    assert_eq!(record.target, "codelet_agent_loop::hooks");
    assert_eq!(record.message, "starting agent_loop for session abc");
    match rx.try_recv() {
        Err(broadcast::error::TryRecvError::Empty) => {}
        other => panic!("expected exactly one record, found another: {other:?}"),
    }
}
