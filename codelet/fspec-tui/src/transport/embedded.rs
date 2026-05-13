//! Embedded (in-process) [`FspecBackend`] implementation.
//!
//! Feature: spec/features/fspec-tui-embedded-backend.feature
//! Architecture note 6 (RPC-008): wraps `codelet_rpc_embedded::EmbeddedTransport`
//! and preserves the RPC-005 Q9 host-supplied-Handle invariant at the trait
//! boundary — `new` takes a non-defaulted `tokio::runtime::Handle` plus
//! `Arc<SharedFspecService>`.
//!
//! Construction immediately spawns the in-process tarpc server task on the
//! supplied runtime handle (via `EmbeddedTransport::client()`), so RPC method
//! bodies are one-line delegates to the cached client. Subscription methods
//! are zero-cost passthroughs to the underlying `EmbeddedTransport::*_rx`.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelet_rpc::{FspecServiceClient, SharedFspecService};
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::{
    HealthInfo, LogRecord, SessionId, SessionInfo, StreamChunk, WorkUnitInfo,
};
use tarpc::context;
use tokio::sync::broadcast;

use super::FspecBackend;

/// In-process [`FspecBackend`] backed by `codelet_rpc_embedded::EmbeddedTransport`.
///
/// Holds the underlying transport (so subscription receivers stay alive)
/// alongside a cached tarpc client whose worker task was spawned at
/// construction time on the host-supplied runtime handle.
pub struct EmbeddedFspecBackend {
    transport: EmbeddedTransport,
    client: FspecServiceClient,
}

impl EmbeddedFspecBackend {
    /// Build an embedded backend bound to the supplied tokio runtime handle
    /// and shared service.
    ///
    /// The `handle` argument is intentionally NON-DEFAULTED so the
    /// RPC-005 Q9 invariant ("EmbeddedTransport requires a tokio Handle
    /// at construction") propagates to this trait boundary. See
    /// `codelet/rpc-embedded/tests/architecture_invariants.rs::scenario_7_*`
    /// (widened by RPC-008 to scan `codelet/fspec-tui/src/` too).
    pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self {
        let transport = EmbeddedTransport::new(handle, service);
        let client = transport.client();
        Self { transport, client }
    }
}

#[async_trait]
impl FspecBackend for EmbeddedFspecBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(self.client.list_work_units(context::current()).await?)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        Ok(self.client.list_sessions(context::current()).await?)
    }

    async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        Ok(self.client.create_session(context::current(), role).await?)
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.client.send_input(context::current(), id, text).await?;
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        self.client.interrupt(context::current(), id).await?;
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.transport.work_units_rx()
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.transport.chunks_rx()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.transport.logs_rx()
    }

    async fn health(&self) -> Result<HealthInfo> {
        // RPC-011: embedded backend routes through the same tarpc
        // FspecService::health() method — both transports share the
        // single `FspecServiceImpl` implementation per RPC-005 rule.
        Ok(self.client.health(context::current()).await?)
    }
}
