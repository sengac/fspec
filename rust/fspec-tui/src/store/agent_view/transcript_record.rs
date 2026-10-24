use codelet_rpc_types::{StreamChunk, TranscriptOriginal};

use super::session_context::SessionContext;
use crate::views::agent::ChunkKind;

impl SessionContext {
    pub(crate) fn record_transcript_chunk(
        &mut self,
        chunk: &StreamChunk,
        original: Option<TranscriptOriginal>,
    ) {
        let trimmed = self.scrollback.trimmed_rows_total();
        super::record_chunk::record_chunk(self, chunk);
        if self.scrollback.trimmed_rows_total() != trimmed {
            let resident = self
                .scrollback
                .chunks()
                .iter()
                .map(|chunk| chunk.seq)
                .collect::<std::collections::HashSet<_>>();
            self.transcript_originals
                .retain(|sequence, _| resident.contains(sequence));
        }
        let Some(original) = original else {
            return;
        };
        let sequence = match chunk {
            StreamChunk::ToolResult { tool_result, .. } => {
                self.transcript_tool_sequence(&tool_result.tool_call_id)
            }
            StreamChunk::ToolProgress { tool_progress, .. } => {
                self.transcript_tool_sequence(&tool_progress.tool_call_id)
            }
            StreamChunk::Thinking { .. } => self
                .in_flight_thinking
                .and_then(|index| self.scrollback.chunks().get(index))
                .map(|chunk| chunk.seq),
            StreamChunk::Done
            | StreamChunk::ResponseAttemptAborted
            | StreamChunk::ResponseAttemptStarted => None,
            _ => self.scrollback.chunks().last().map(|chunk| chunk.seq),
        };
        if let Some(sequence) = sequence {
            let originals = self.transcript_originals.entry(sequence).or_default();
            if !originals.iter().any(|stored| {
                stored.turn_index == original.turn_index && stored.message_id == original.message_id
            }) {
                originals.push(original);
            }
        }
    }

    fn transcript_tool_sequence(&self, identity: &str) -> Option<u64> {
        self.scrollback.chunks().iter().find(|chunk| {
            matches!(chunk.source.as_ref().map(|source| &source.kind), Some(ChunkKind::ToolCall { tool_call_id, .. }) if tool_call_id == identity)
        }).map(|chunk| chunk.seq)
    }
}
