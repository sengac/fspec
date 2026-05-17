//! RPC-007: tracing::Layer that captures structured tracing events into
//! a `tokio::sync::broadcast::Sender<LogRecord>`.
//!
//! ## Multi-service multiplex (test-process global)
//!
//! [`register_log_layer`] is called once per `SharedFspecService` (e.g.
//! once per integration-test setup). Because `tracing_subscriber`
//! installs a single global subscriber per process, naive registration
//! would cause only the first call to actually wire its layer up — every
//! subsequent service's `logs_tx` would be silently disconnected from
//! the global subscriber. To keep both rpc-server and rpc-embedded
//! tests honest (and to support hosts that wire up multiple services
//! in the same process), this module installs ONE global layer that
//! fans out every captured event to every currently-registered
//! `broadcast::Sender<LogRecord>`. Each call to [`register_log_layer`]
//! pushes the service's logs_tx onto that global list.
//!
//! ## NAPI parity
//!
//! Mirrors the per-process pattern of NAPI's TypeScriptLayer at
//! codelet/napi/src/lib.rs:152-205 — the difference is that this layer
//! pushes into a broadcast channel instead of a ThreadsafeFunction so
//! multiple subscribers (NAPI, embedded callers, WS fan-out) can
//! co-listen on the same source of truth.

use crate::SharedFspecService;
use codelet_rpc_types::LogRecord;
use std::fmt::Write;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// Process-global registry of `LogRecord` senders. Every captured
/// tracing event is fanned out to each sender. Populated by
/// [`register_log_layer`].
fn senders() -> &'static Mutex<Vec<broadcast::Sender<LogRecord>>> {
    static SENDERS: OnceLock<Mutex<Vec<broadcast::Sender<LogRecord>>>> = OnceLock::new();
    SENDERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// `tracing::Layer` that publishes [`LogRecord`] values onto every
/// sender currently registered in [`senders`].
///
/// Constructed once and installed on the global subscriber by
/// [`register_log_layer`]. New services calling [`register_log_layer`]
/// after install push onto the same global list — the layer itself is
/// never re-installed.
pub struct BroadcastLogLayer;

impl BroadcastLogLayer {
    /// Construct a fresh layer wired to a single broadcast sender.
    /// Retained for backward compatibility with existing rpc-embedded
    /// tests; new code should call [`register_log_layer`] instead.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(sender: broadcast::Sender<LogRecord>) -> SingleBroadcastLogLayer {
        SingleBroadcastLogLayer { sender }
    }
}

/// Single-sender variant retained for backward compatibility.
pub struct SingleBroadcastLogLayer {
    sender: broadcast::Sender<LogRecord>,
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
            // tracing's Display impl wraps strings in quotes when using the
            // Debug visitor; strip a single pair of leading/trailing quotes
            // so the captured message matches the user-supplied literal.
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

fn build_record(event: &Event<'_>) -> LogRecord {
    let mut visitor = MessageVisitor::default();
    event.record(&mut visitor);
    let metadata = event.metadata();
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    LogRecord {
        level: metadata.level().as_str().to_string(),
        target: metadata.target().to_string(),
        message: visitor.message,
        timestamp_ms,
    }
}

impl<S> Layer<S> for BroadcastLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let record = build_record(event);
        if let Ok(guard) = senders().lock() {
            for sender in guard.iter() {
                let _ = sender.send(record.clone());
            }
        }
    }
}

impl<S> Layer<S> for SingleBroadcastLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let record = build_record(event);
        let _ = self.sender.send(record);
    }
}

/// Install a [`BroadcastLogLayer`] on the global tracing subscriber so
/// tracing emissions on the host are observable on
/// [`SharedFspecService::logs_rx`] (and therefore on
/// `EmbeddedTransport::logs_rx` and `FspecWsClient::logs_rx`).
///
/// The first call in a process installs the global multiplex layer.
/// Every call (first or subsequent) appends the service's logs_tx
/// onto the global sender list, so every tracing event is fanned out
/// to every registered service.
pub fn register_log_layer(
    service: Arc<SharedFspecService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(mut guard) = senders().lock() {
        guard.push(service.logs_tx());
    }
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        // try_init returns Err if a global subscriber is already
        // installed (e.g. by an rpc-server binary's tracing_subscriber::fmt
        // call). In that case the multiplex layer can't be added to the
        // already-installed subscriber, but the per-test scenarios that
        // rely on register_log_layer install no other subscriber, so the
        // first call here wins and multiplexes for every subsequent
        // caller.
        let _ = tracing_subscriber::registry()
            .with(BroadcastLogLayer)
            .try_init();
    });
    Ok(())
}
