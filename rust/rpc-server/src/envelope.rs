//! WebSocket framing envelope for the fspec RPC wire protocol.
//!
//! Each WebSocket *binary* frame carries a single bincode-encoded
//! [`Envelope`]. RPC-005 implemented only [`Envelope::Rpc`]; RPC-006
//! lit up [`Envelope::WorkUnitsUpdate`] as the first push variant;
//! RPC-007 promotes [`Envelope::Event`] and [`Envelope::LogEvent`] from
//! reserved unit-variants to payload-bearing struct/tuple variants for
//! `(SessionId, StreamChunk)` and `LogRecord` server-pushed traffic.
//!
//! The remaining variants ([`Envelope::CmdReq`], [`Envelope::CmdRes`])
//! are reserved and rejected with a tracing warning.
//!
//! The bytes inside [`Envelope::Rpc`] are the bincode-encoded tarpc protocol
//! message (`tarpc::ClientMessage<FspecServiceRequest>` or
//! `tarpc::Response<FspecServiceResponse>`). [`Envelope::WorkUnitsUpdate`]
//! carries a full `Vec<WorkUnitInfo>` snapshot replacement (per RPC-006
//! plan §Step 3). [`Envelope::Event`] carries `(session_id, chunk)`
//! pairs identical to the embedded `chunks_rx` payload, and
//! [`Envelope::LogEvent`] carries the `LogRecord` published by the
//! host's tracing::Layer (RPC-007 architecture notes 6 + 7).

use codelet_rpc_types::{LogRecord, SessionId, SessionStatus, StreamChunk, WorkUnitInfo};
use serde::{Deserialize, Serialize};

/// WebSocket framing envelope.
///
/// RPC-005 implemented only `Rpc`; RPC-006 promoted `WorkUnitsUpdate`
/// from a unit-variant placeholder to a payload-bearing variant
/// `WorkUnitsUpdate(Vec<WorkUnitInfo>)`; RPC-007 promotes both `Event`
/// and `LogEvent` to payload-bearing variants. The remaining variants
/// (`CmdReq`, `CmdRes`) are reserved-but-rejected. They exist on the
/// type so the wire format is forward-compatible without breaking
/// changes when we add reverse callbacks (later card).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Envelope {
    /// Bincode-encoded tarpc protocol message. Carries the entire RPC
    /// request/response in either direction.
    Rpc(Vec<u8>),
    /// Server-pushed streaming chunk frame (RPC-007).
    /// Carries `(session_id, chunk)` — identical payload as the
    /// embedded `chunks_rx` broadcast so cross-transport parity holds.
    Event {
        session_id: SessionId,
        chunk: StreamChunk,
    },
    /// Server-pushed structured log record (RPC-007).
    /// Carries the `LogRecord` published by the host's tracing::Layer.
    LogEvent(LogRecord),
    /// Server-pushed work-units snapshot replacement (RPC-006).
    /// Carries a full `Vec<WorkUnitInfo>` — the entire current state of
    /// `spec/work-units.json` after a debounced fs-watch event. Sent
    /// once on connect (initial snapshot) and once per debounced
    /// mutation thereafter.
    WorkUnitsUpdate(Vec<WorkUnitInfo>),
    /// Server-pushed session status change (RPC-037).
    /// Carries `(session_id, status)` — fed by the per-connection
    /// `status_changes_fanout` task draining `SharedFspecService::status_changes_rx`
    /// (which delegates to the attached `SessionManagerHandle`). Mirrors
    /// the embedded `EmbeddedTransport::status_changes_rx` payload so
    /// cross-transport parity holds for push-driven status updates.
    StatusUpdate {
        session_id: SessionId,
        status: SessionStatus,
    },
    /// Reserved: server-to-client command request (reverse channel).
    /// Rejected in RPC-007.
    CmdReq,
    /// Reserved: client-to-server command response. Rejected in RPC-007.
    CmdRes,
}

impl Envelope {
    /// Human-readable variant name used in the rejection tracing warning so
    /// the rejection test can grep for the variant the server saw.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Envelope::Rpc(_) => "Rpc",
            Envelope::Event { .. } => "Event",
            Envelope::LogEvent(_) => "LogEvent",
            Envelope::WorkUnitsUpdate(_) => "WorkUnitsUpdate",
            Envelope::StatusUpdate { .. } => "StatusUpdate",
            Envelope::CmdReq => "CmdReq",
            Envelope::CmdRes => "CmdRes",
        }
    }
}
