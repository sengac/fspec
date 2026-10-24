use std::time::Instant;

use codelet_rpc_types::{SessionId, TranscriptAssembler, TranscriptPage};
use tokio::task::AbortHandle;

#[derive(Debug, Clone)]
pub enum TranscriptAction {
    Poll,
    Wake(SessionId),
    Live(SessionId, codelet_rpc_types::StreamChunk),
    WakeError(SessionId, String),
    Loaded {
        session: SessionId,
        nonce: u64,
        request_id: u64,
        result: Result<TranscriptPage, String>,
    },
    Resumed {
        session: SessionId,
        nonce: u64,
        result: Result<(), String>,
    },
    Inspected {
        session: SessionId,
        nonce: u64,
        inspection_id: u64,
        result: Result<String, String>,
    },
}

pub(crate) struct TranscriptSync {
    pub nonce: u64,
    pub assembler: TranscriptAssembler,
    pub snapshot_end: Option<u64>,
    pub in_flight: Option<AbortHandle>,
    pub request_id: u64,
    pub dirty: bool,
    pub suspended: bool,
    pub poll_after: Instant,
    pub notice: Option<u64>,
    pub inspector: Option<AbortHandle>,
    pub inspection_id: u64,
}

impl TranscriptSync {
    pub fn new(nonce: u64) -> Self {
        Self {
            nonce,
            assembler: TranscriptAssembler::default(),
            snapshot_end: None,
            in_flight: None,
            request_id: 0,
            dirty: false,
            suspended: false,
            poll_after: Instant::now(),
            notice: None,
            inspector: None,
            inspection_id: 0,
        }
    }
}

impl Drop for TranscriptSync {
    fn drop(&mut self) {
        for task in [&self.in_flight, &self.inspector].into_iter().flatten() {
            task.abort();
        }
    }
}
