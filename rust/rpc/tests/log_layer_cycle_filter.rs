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

/// BUG-572: route `log` crate records into tracing the way the registry's
/// `try_init` does in a host (tungstenite and tokio_tungstenite log through
/// the `log` crate). Bridged records reach the thread's default dispatcher.
fn bridge_log_records() {
    static BRIDGE: std::sync::Once = std::sync::Once::new();
    BRIDGE.call_once(|| {
        if let Err(error) = tracing_log::LogTracer::init() {
            panic!("the log bridge installs once per test process: {error}");
        }
    });
}

/// Drain every record the sender received, in order.
fn drain(rx: &mut broadcast::Receiver<LogRecord>) -> Vec<LogRecord> {
    let mut records = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(record) => records.push(record),
            Err(broadcast::error::TryRecvError::Empty) => return records,
            Err(other) => panic!("the broadcast must neither lag nor close here: {other:?}"),
        }
    }
}

#[test]
fn below_warning_events_from_the_websocket_transport_crates_are_not_broadcast() {
    // BUG-572: tungstenite logs every frame it writes; broadcasting those
    // records made each forwarded LogEvent produce more records to forward.
    // @step Given a BroadcastLogLayer is installed with a registered broadcast sender
    let (tx, mut rx) = broadcast::channel::<LogRecord>(64);
    let dispatch = build_subscriber(tx);

    // @step When tungstenite, tokio_tungstenite and tarpc emit TRACE, DEBUG and INFO events through tracing and through the log crate
    bridge_log_records();
    dispatcher::with_default(&dispatch, || {
        log::trace!(target: "tungstenite::protocol", "Sending frame");
        log::debug!(target: "tokio_tungstenite::compat", "WouldBlock");
        log::info!(target: "tungstenite", "handshake");
        tracing::event!(target: "tungstenite", tracing::Level::TRACE, "frame");
        tracing::event!(target: "tungstenite::protocol", tracing::Level::TRACE, "Sending frame");
        tracing::event!(target: "tungstenite::protocol", tracing::Level::DEBUG, "Received close frame");
        tracing::event!(target: "tungstenite::handshake::server", tracing::Level::INFO, "handshake");
        tracing::event!(target: "tokio_tungstenite", tracing::Level::TRACE, "Sink.poll_ready");
        tracing::event!(target: "tokio_tungstenite::compat", tracing::Level::DEBUG, "WouldBlock");
        tracing::event!(target: "tokio_tungstenite", tracing::Level::INFO, "connected");
        tracing::event!(target: "tarpc::server", tracing::Level::INFO, "ReceiveRequest");
        tracing::event!(target: "tarpc::client", tracing::Level::DEBUG, "SendRequest");
        tracing::event!(target: "tarpc", tracing::Level::TRACE, "poll");
    });

    // @step Then the registered broadcast sender receives zero LogRecord values
    let received = drain(&mut rx);
    assert!(
        received.is_empty(),
        "below-warning transport events must not be broadcast, got: {received:?}"
    );
}

#[test]
fn transport_warnings_and_errors_and_look_alike_targets_are_still_broadcast() {
    // @step Given a BroadcastLogLayer is installed with a registered broadcast sender
    let (tx, mut rx) = broadcast::channel::<LogRecord>(64);
    let dispatch = build_subscriber(tx);

    // @step When tungstenite emits a WARN event through the log crate, tarpc emits an ERROR event and a target named "tungstenite_ext" emits a TRACE event
    bridge_log_records();
    dispatcher::with_default(&dispatch, || {
        log::warn!(target: "tungstenite::protocol", "connection reset");
        tracing::event!(target: "tarpc::server", tracing::Level::ERROR, "response dropped");
        tracing::event!(target: "tungstenite_ext", tracing::Level::TRACE, "not the transport");
    });

    // @step Then the registered broadcast sender receives exactly those three LogRecord values in order, each with its own target and level
    // A bridged record's message also carries the log.* fields, so only its
    // leading text is compared.
    let received: Vec<(String, String, String)> = drain(&mut rx)
        .into_iter()
        .map(|r| {
            let text = r
                .message
                .split(" log.")
                .next()
                .unwrap_or_default()
                .to_string();
            (r.level, r.target, text)
        })
        .collect();
    let expected: Vec<(String, String, String)> = [
        ("WARN", "tungstenite::protocol", "connection reset"),
        ("ERROR", "tarpc::server", "response dropped"),
        ("TRACE", "tungstenite_ext", "not the transport"),
    ]
    .into_iter()
    .map(|(l, t, m)| (l.to_string(), t.to_string(), m.to_string()))
    .collect();
    assert_eq!(received, expected);
}
