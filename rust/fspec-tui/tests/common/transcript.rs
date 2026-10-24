use codelet_fspec_tui::transport::BackendError;
use codelet_rpc_types::{
    SessionId, TranscriptCursor, TranscriptError, TranscriptFragment, TranscriptPage,
};

use super::MockBackend;

impl MockBackend {
    pub fn transcript_requests(&self) -> Vec<SessionId> {
        self.transcript_requests
            .lock()
            .expect("transcript requests fixture lock")
            .clone()
    }

    pub(super) fn scripted_transcript_page(
        &self,
        session: SessionId,
        cursor: Option<TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<TranscriptPage, BackendError> {
        self.transcript_requests
            .lock()
            .expect("transcript requests fixture lock")
            .push(session.clone());
        let chunks = self
            .buffered_output
            .lock()
            .expect("transcript chunks fixture lock");
        let generation = session.value;
        let cursor = cursor.unwrap_or(TranscriptCursor {
            generation: generation.clone(),
            sequence: 0,
            byte_offset: 0,
        });
        if cursor.generation != generation {
            return Err(TranscriptError::GenerationMismatch.into());
        }
        if cursor.byte_offset != 0 {
            return Err(TranscriptError::InvalidCursor.into());
        }
        let end = snapshot_end.unwrap_or(chunks.len() as u64);
        if end > chunks.len() as u64 || cursor.sequence > end {
            return Err(TranscriptError::InvalidSnapshot.into());
        }
        let mut page = TranscriptPage {
            generation,
            snapshot_end: end,
            fragment: None,
            next_cursor: cursor.clone(),
            complete: cursor.sequence == end,
        };
        if !page.complete {
            let sequence =
                usize::try_from(cursor.sequence).map_err(|_| TranscriptError::InvalidCursor)?;
            let encoded = serde_json::to_string(&chunks[sequence])
                .map_err(|error| TranscriptError::InvalidPayload(error.to_string()))?;
            page.fragment = Some(TranscriptFragment {
                sequence: cursor.sequence,
                byte_offset: 0,
                total_bytes: encoded.len(),
                json_fragment: encoded,
                original: None,
            });
            page.next_cursor.sequence += 1;
            page.complete = page.next_cursor.sequence == end;
        }
        let bytes = serde_json::to_vec(&page)
            .map_err(|error| TranscriptError::InvalidPayload(error.to_string()))?;
        if bytes.len() > max_bytes as usize {
            return Err(TranscriptError::InvalidPayload(
                "scripted event exceeds the page budget".into(),
            )
            .into());
        }
        Ok(page)
    }
}
