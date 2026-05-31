//! LOG-005: integration regression test for the default tracing filter.
//!
//! Background: `~/.fspec/logs/fspec-combined.log` was 98% `tarpc::client` /
//! `tarpc::server` INFO span events. Every RPC round-trip emits 6–8
//! INFO events (`SendRequest`, `ReceiveRequest`, `BeginRequest`,
//! `CompleteRequest`, `BufferResponse`, `SendResponse`,
//! `ReceiveResponse`). With `EnvFilter::new("info")` as the fallback,
//! every one of those landed in the daily log, drowning out real
//! diagnostics. (LOG-004 fixed the *recursion* amplification; this card
//! kills the *per-RPC* firehose at the source.)
//!
//! The fix in `codelet/fspec/src/common.rs::env_filter` pins
//! `tarpc::client` and `tarpc::server` to `warn` in the fallback
//! directive set via the `TARPC_QUIET_DIRECTIVES` constant. This test
//! enforces TWO invariants:
//!
//!   1. SOURCE-TEXT INVARIANT: `common.rs` defines
//!      `TARPC_QUIET_DIRECTIVES` and the table contains literal
//!      `tarpc::client=warn` and `tarpc::server=warn` entries.
//!   2. RUNTIME INVARIANT: a tracing subscriber built with the same
//!      directives drops `tarpc::client` / `tarpc::server` INFO events
//!      while still passing `tarpc::*` WARN events and other targets'
//!      INFO events through.
//!
//! Together they guarantee the filter (a) keeps the constant intact and
//! (b) behaves correctly at runtime, without any cross-binary call into
//! private items of the `codelet-fspec` binary crate.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Registry};

/// Capturing layer used by the runtime assertion: records every
/// emitted event's `(target, level)` for later inspection.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<(String, tracing::Level)>>>);

impl<S> Layer<S> for Capture
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let m = event.metadata();
        self.0
            .lock()
            .expect("capture lock")
            .push((m.target().to_string(), *m.level()));
    }
}

/// Scenario: the default fallback tracing filter suppresses tarpc INFO
/// span ceremony while preserving WARN and unrelated targets' INFO.
#[test]
fn default_env_filter_suppresses_tarpc_info_keeps_warn_and_other_info() {
    // @step Given the default fallback directive set (no RUST_LOG)
    // The literal table mirrors the `TARPC_QUIET_DIRECTIVES` constant
    // in codelet/fspec/src/common.rs. The companion source-text
    // assertion below guarantees the constant in common.rs contains
    // these exact strings, so this test cannot drift from production.
    let mut filter = EnvFilter::new("info");
    for d in ["tarpc::client=warn", "tarpc::server=warn"] {
        filter = filter.add_directive(d.parse().expect("hardcoded directive must parse"));
    }

    let capture = Capture::default();
    let subscriber = Registry::default().with(capture.clone()).with(filter);

    // @step When INFO and WARN events fire on tarpc::client, tarpc::server,
    // and an unrelated target (codelet_agent_loop)
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "tarpc::client", "SendRequest");
        tracing::info!(target: "tarpc::server", "ReceiveRequest");
        tracing::info!(target: "codelet_agent_loop::agent_loop", "thinking detected");
        tracing::warn!(target: "tarpc::client", "warn-level still passes");
    });

    let captured = capture.0.lock().expect("capture lock").clone();
    let targets: Vec<&str> = captured.iter().map(|(t, _)| t.as_str()).collect();

    // @step Then tarpc::client INFO is dropped
    assert!(
        !captured
            .iter()
            .any(|(t, lvl)| t == "tarpc::client" && *lvl == tracing::Level::INFO),
        "tarpc::client INFO events MUST be suppressed — regressed LOG-005. Captured targets: {targets:?}"
    );

    // @step And tarpc::server INFO is dropped
    assert!(
        !captured
            .iter()
            .any(|(t, lvl)| t == "tarpc::server" && *lvl == tracing::Level::INFO),
        "tarpc::server INFO events MUST be suppressed — regressed LOG-005. Captured targets: {targets:?}"
    );

    // @step And codelet_agent_loop INFO still passes through
    assert!(
        captured
            .iter()
            .any(|(t, lvl)| t == "codelet_agent_loop::agent_loop"
                && *lvl == tracing::Level::INFO),
        "codelet_agent_loop INFO events MUST still pass through — filter is too aggressive. Captured: {targets:?}"
    );

    // @step And tarpc::client WARN still passes (directive floor is `warn`, not `off`)
    assert!(
        captured
            .iter()
            .any(|(t, lvl)| t == "tarpc::client" && *lvl == tracing::Level::WARN),
        "tarpc::client WARN events MUST still pass — directive should be `warn` not `off`. Captured: {targets:?}"
    );
}

/// Scenario: common.rs source contains the `TARPC_QUIET_DIRECTIVES`
/// constant pinning both tarpc::client and tarpc::server to `warn`.
///
/// This guards against silent regression of the runtime fix: if someone
/// deletes the constant or relaxes the directives back to `info`, the
/// runtime invariant above would still be satisfied for THIS test's
/// hand-built filter, but production would re-flood the log. The
/// source-text check closes that hole.
#[test]
fn common_rs_defines_tarpc_quiet_directives_constant() {
    // @step Given the codelet-fspec source tree
    let common_rs = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/common.rs"
    ))
    .expect("read codelet/fspec/src/common.rs");

    // @step Then the file declares a TARPC_QUIET_DIRECTIVES constant
    assert!(
        common_rs.contains("TARPC_QUIET_DIRECTIVES"),
        "codelet/fspec/src/common.rs MUST define a TARPC_QUIET_DIRECTIVES constant — regressed LOG-005"
    );

    // @step And the constant contains tarpc::client=warn
    assert!(
        common_rs.contains("\"tarpc::client=warn\""),
        "codelet/fspec/src/common.rs MUST list \"tarpc::client=warn\" in TARPC_QUIET_DIRECTIVES — regressed LOG-005"
    );

    // @step And the constant contains tarpc::server=warn
    assert!(
        common_rs.contains("\"tarpc::server=warn\""),
        "codelet/fspec/src/common.rs MUST list \"tarpc::server=warn\" in TARPC_QUIET_DIRECTIVES — regressed LOG-005"
    );

    // @step And env_filter applies the directives to the fallback EnvFilter
    assert!(
        common_rs.contains("for directive in TARPC_QUIET_DIRECTIVES"),
        "codelet/fspec/src/common.rs::env_filter MUST iterate TARPC_QUIET_DIRECTIVES and add each directive — regressed LOG-005"
    );
}
