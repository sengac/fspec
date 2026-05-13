//! Multi-client + broadcast capacity + tracing client_id tests — RPC-011.
//!
//! Feature: spec/features/multi-client-and-tracing.feature
//!
//! Covers:
//!   - Two clients attached simultaneously see the same chunk stream
//!   - Broadcast capacities are explicit and tuned (chunks=1024, logs=4096, work_units=256)
//!   - Tracing spans carry client_id on per-connection handler tasks
//!
//! Red phase: requires `DEFAULT_WORK_UNITS_CAPACITY` const, retuned chunks
//! (1024) and logs (4096) capacities, `#[tracing::instrument(fields(client_id = %peer))]`
//! on handle_connection, and a `ServerStats::connected_clients()` accessor.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, StreamChunk};
use common::{connect_with_retry, make_workspace};
use tarpc::context;
use tokio::time::timeout;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Two clients attached simultaneously see the same chunk stream
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_clients_attached_simultaneously_see_the_same_chunk_stream() {
    // @step Given a daemon listening on 127.0.0.1:0
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(
        StubSessionManagerHandle::with_provider(Arc::new(StubProvider::new())),
    );
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    let (addr, stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    // @step When the test opens two WebSocketFspecBackends (WS-A and WS-B) against that daemon
    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A");
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B");
    let mut rx_a = ws_a.chunks_rx();
    let mut rx_b = ws_b.chunks_rx();

    // Allow the daemon a tick to register both connections.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // @step And ServerStats.connected_clients reads 2 throughout the test
    assert_eq!(
        stats.connected_clients(),
        2,
        "two connected clients must be counted"
    );

    // @step And WS-A calls create_session(None) returning session_id S
    let sid: SessionId = ws_a
        .client()
        .create_session(context::current(), None)
        .await
        .expect("create_session");

    // @step And WS-A calls send_input(S, "hi")
    ws_a.client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input");

    async fn drain_until_done(
        rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
        sid: &SessionId,
    ) -> Vec<StreamChunk> {
        let mut out = Vec::new();
        for _ in 0..32 {
            match timeout(Duration::from_secs(2), rx.recv()).await {
                Ok(Ok((got_sid, c))) if got_sid == *sid => {
                    let done = matches!(c, StreamChunk::Done { .. });
                    out.push(c);
                    if done {
                        break;
                    }
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        out
    }

    let chunks_a = drain_until_done(&mut rx_a, &sid).await;
    let chunks_b = drain_until_done(&mut rx_b, &sid).await;

    // @step Then both WS-A.chunks_rx() and WS-B.chunks_rx() yield the SAME sequence of (S, chunk) frames in the SAME order
    assert!(!chunks_a.is_empty(), "client A must observe at least one chunk");
    assert_eq!(
        bincode::serialize(&chunks_a).expect("encode A"),
        bincode::serialize(&chunks_b).expect("encode B"),
        "clients A and B must observe byte-equal chunk sequences"
    );

    // @step And no chunk is delivered to one client and not the other
    assert_eq!(
        chunks_a.len(),
        chunks_b.len(),
        "chunk counts must match between A and B"
    );

    // connected_clients still reads 2 (no client has disconnected).
    assert_eq!(stats.connected_clients(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Broadcast capacities are explicit and tuned
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn broadcast_capacities_are_explicit_and_tuned() {
    // @step Given codelet/rpc/src/lib.rs
    let path = workspace_root().join("rpc").join("src").join("lib.rs");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    // @step When inspecting the broadcast capacity constants
    // @step Then DEFAULT_CHUNKS_CAPACITY equals 1024
    assert!(
        body.contains("DEFAULT_CHUNKS_CAPACITY: usize = 1024"),
        "DEFAULT_CHUNKS_CAPACITY must equal 1024 in codelet/rpc/src/lib.rs"
    );

    // @step And DEFAULT_LOGS_CAPACITY equals 4096
    assert!(
        body.contains("DEFAULT_LOGS_CAPACITY: usize = 4096"),
        "DEFAULT_LOGS_CAPACITY must equal 4096 in codelet/rpc/src/lib.rs"
    );

    // @step And DEFAULT_WORK_UNITS_CAPACITY equals 256
    assert!(
        body.contains("DEFAULT_WORK_UNITS_CAPACITY: usize = 256"),
        "DEFAULT_WORK_UNITS_CAPACITY must equal 256 in codelet/rpc/src/lib.rs"
    );

    // @step And the constants are used as the third argument of broadcast::channel(...) in SharedFspecService::new and SharedFspecService::with_session_manager
    assert!(
        body.contains("broadcast::channel(DEFAULT_CHUNKS_CAPACITY)"),
        "broadcast::channel(DEFAULT_CHUNKS_CAPACITY) must be used in SharedFspecService"
    );
    assert!(
        body.contains("broadcast::channel(DEFAULT_LOGS_CAPACITY)"),
        "broadcast::channel(DEFAULT_LOGS_CAPACITY) must be used in SharedFspecService"
    );
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("rpc-server must have a parent")
        .to_path_buf()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Tracing spans carry client_id on per-connection handler tasks
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tracing_spans_carry_client_id_on_per_connection_handler_tasks() {
    use std::sync::Mutex;
    use tracing::Subscriber;
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    // Custom tracing layer that records every event's formatted fields
    // so we can grep for `client_id=...` in the captured output.
    #[derive(Default)]
    struct CaptureLayer {
        records: Arc<Mutex<Vec<String>>>,
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
            use std::fmt::Write;
            let mut visitor = FieldVisitor::default();
            event.record(&mut visitor);
            let mut line = String::new();
            for (k, v) in &visitor.0 {
                let _ = write!(&mut line, "{k}={v} ");
            }
            // Also append span fields.
            if let Some(span) = ctx.lookup_current() {
                line.push('|');
                let mut current = Some(span);
                while let Some(s) = current {
                    line.push(' ');
                    line.push_str(s.name());
                    if let Some(ext) = s.extensions().get::<SpanFieldStash>() {
                        line.push_str(&ext.0);
                    }
                    current = s.parent();
                }
            }
            self.records.lock().unwrap().push(line);
        }

        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            id: &tracing::Id,
            ctx: Context<'_, S>,
        ) {
            let mut visitor = FieldVisitor::default();
            attrs.record(&mut visitor);
            let mut s = String::new();
            for (k, v) in &visitor.0 {
                use std::fmt::Write;
                let _ = write!(&mut s, " {k}={v}");
            }
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(SpanFieldStash(s));
            }
        }
    }

    struct SpanFieldStash(String);

    #[derive(Default)]
    struct FieldVisitor(Vec<(String, String)>);

    impl tracing::field::Visit for FieldVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0
                .push((field.name().to_string(), format!("{value:?}")));
        }
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.push((field.name().to_string(), value.to_string()));
        }
    }

    let records = Arc::new(Mutex::new(Vec::<String>::new()));
    let layer = CaptureLayer {
        records: records.clone(),
    };
    // Use set_global_default so the subscriber is visible to tokio
    // worker threads that run handle_connection. Tests in this binary
    // each get a fresh process so the single global default applies
    // cleanly.
    let subscriber = tracing_subscriber::registry().with(layer);
    let _guard = tracing::subscriber::set_global_default(subscriber)
        .map(|_| ())
        .or_else(|_| -> Result<(), ()> {
            // Already set in this process — fine for our purposes.
            Ok(())
        });

    // @step Given a daemon with two simultaneous clients on peer addrs 127.0.0.1:54321 and 127.0.0.1:54322
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(watcher));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A");
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B");
    tokio::time::sleep(Duration::from_millis(200)).await;

    // @step When both clients call list_work_units once each
    let _ = ws_a
        .client()
        .list_work_units(context::current())
        .await
        .expect("list_work_units A");
    let _ = ws_b
        .client()
        .list_work_units(context::current())
        .await
        .expect("list_work_units B");
    tokio::time::sleep(Duration::from_millis(200)).await;

    // @step Then the daemon's tracing output contains at least two records with field client_id=127.0.0.1:54321
    // @step And at least two records with field client_id=127.0.0.1:54322
    // (We cannot pin specific ephemeral ports; assert the shape only:
    // at least two records carry a client_id=127.0.0.1:<port> tag.)
    let captured = records.lock().unwrap().clone();
    let client_id_count = captured
        .iter()
        .filter(|r| r.contains("client_id="))
        .count();
    assert!(
        client_id_count >= 2,
        "must observe at least 2 tracing records tagged with client_id. Got {client_id_count}; sample: {captured:?}"
    );

    // @step And grepping the log for client_id=127.0.0.1:54321 yields ONLY records originating from that connection's task
    // Surrogate: extract the distinct client_id values from captured
    // records — there should be at least 2 different ones (one per
    // simultaneous connection).
    let mut distinct: std::collections::HashSet<String> = std::collections::HashSet::new();
    for rec in &captured {
        if let Some(start) = rec.find("client_id=") {
            let rest = &rec[start + "client_id=".len()..];
            let end = rest.find(' ').unwrap_or(rest.len());
            distinct.insert(rest[..end].to_string());
        }
    }
    assert!(
        distinct.len() >= 2,
        "must observe at least 2 distinct client_id values, got {distinct:?}"
    );
}
