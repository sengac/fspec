//! codelet-rpc-embedded: in-process tarpc transport that shares one
//! [`FspecService`] implementation with `codelet-rpc-server`.
//!
//! ## Runtime sharing invariant (RPC-002 Q9)
//!
//! [`EmbeddedTransport::new`] takes a `tokio::runtime::Handle` from the host
//! and spawns the server task on that handle. We deliberately do NOT call
//! `tokio::runtime::Builder` or `Runtime::new` anywhere in this crate.
//!
//! ## Shared service impl
//!
//! [`SharedFspecService`] and [`FspecServiceImpl`] live in `codelet-rpc`,
//! not here, so the WebSocket server reaches the same single instance
//! (RPC-005 architecture rule "service impl written ONCE in a shared module").
//!
//! ## RPC-006 + RPC-007 push channels
//!
//! [`EmbeddedTransport::work_units_rx`], [`EmbeddedTransport::chunks_rx`],
//! and [`EmbeddedTransport::logs_rx`] each return the watcher's
//! broadcast subscription DIRECTLY — no envelope encoding, no fan-out task
//! on the embedded read path (zero-cost path per RPC-002 §5.1). These are
//! the siblings of `FspecWsClient::work_units_rx`,
//! `FspecWsClient::chunks_rx`, and `FspecWsClient::logs_rx` exposed by
//! `codelet-rpc-server` so the future ratatui frontend (RPC-008/RPC-009)
//! can be transport-agnostic.

pub use codelet_rpc::{
    register_log_layer, BroadcastLogLayer, FspecServiceClient, FspecServiceImpl,
    SharedFspecService,
};

use codelet_rpc::FspecService;
use codelet_rpc_types::{LogRecord, SessionId, StreamChunk, WorkUnitInfo};
use futures::StreamExt;
use std::sync::Arc;
use tarpc::{
    client,
    server::{self, Channel},
};
use tokio::sync::broadcast;

/// In-process tarpc transport.
pub struct EmbeddedTransport {
    handle: tokio::runtime::Handle,
    service: Arc<SharedFspecService>,
}

impl EmbeddedTransport {
    /// Build an embedded transport bound to the supplied tokio runtime handle.
    pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self {
        Self { handle, service }
    }

    /// Build an embedded transport AND register a `BroadcastLogLayer` against
    /// the global tracing subscriber so tracing emissions on the host are
    /// observable on `logs_rx()`. Idempotent — if a global subscriber is
    /// already installed (e.g. by the rpc-server binary or by a test
    /// harness), this call only adds the layer to it via `try_init()`.
    pub fn with_log_layer(
        handle: tokio::runtime::Handle,
        service: Arc<SharedFspecService>,
    ) -> Self {
        let _ = register_log_layer(Arc::clone(&service));
        Self { handle, service }
    }

    /// Obtain a tarpc client connected to the in-process server task.
    pub fn client(&self) -> FspecServiceClient {
        let (client_transport, server_transport) = tarpc::transport::channel::unbounded();

        let server = server::BaseChannel::with_defaults(server_transport);
        let service_impl = FspecServiceImpl::new(Arc::clone(&self.service));

        self.handle.spawn(
            server
                .execute(service_impl.serve())
                .for_each(|response| async move {
                    tokio::spawn(response);
                }),
        );

        FspecServiceClient::new(client::Config::default(), client_transport).spawn()
    }

    /// Obtain a fresh broadcast receiver subscribed to the work-units
    /// watcher backing the shared service (RPC-006).
    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.service.watcher_rx()
    }

    /// Obtain a fresh broadcast receiver subscribed to the chunks
    /// channel housed in `SharedFspecService` (RPC-007). The embedded
    /// path returns the broadcast subscription DIRECTLY — no envelope
    /// encoding, no bincode round-trip.
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.service.chunks_rx()
    }

    /// Obtain a fresh broadcast receiver subscribed to the LogRecord
    /// channel housed in `SharedFspecService` (RPC-007). Mirrors
    /// `chunks_rx` for tracing emissions captured by the host's
    /// custom tracing::Layer.
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.service.logs_rx()
    }
}
