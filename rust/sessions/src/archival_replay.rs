use crate::background_session::BackgroundSession;
use crate::replay_content::Replay;
use codelet_core::persistence::{
    get_session_archival_page, load_session, ArchiveCursor, ArchiveFragment, ArchiveRecordKind,
};
use codelet_rpc_types::{NotificationSeverity, StreamChunk, TranscriptError, TranscriptOriginal};
use std::collections::VecDeque;
use uuid::Uuid;

struct ArchivalReplay {
    session_id: Uuid,
    cursor: Option<ArchiveCursor>,
    finished: bool,
    chunks: VecDeque<StreamChunk>,
    original: Option<TranscriptOriginal>,
    compaction: Option<(usize, StreamChunk)>,
    marker: Option<StreamChunk>,
}

impl BackgroundSession {
    pub fn restore_archival_transcript(&self) -> Result<bool, TranscriptError> {
        let manifest = load_session(self.id).map_err(TranscriptError::JournalUnavailable)?;
        let compaction = manifest
            .compaction
            .as_ref()
            .zip(manifest.compaction_commit.as_ref())
            .map(|(state, commit)| {
                (
                    state.compacted_before_index,
                    StreamChunk::user_notification(
                        format!(
                            "[compaction metadata] generation {} at {}; originals 0..{}\n{}",
                            commit.identity.generation,
                            state.compacted_at.to_rfc3339(),
                            state.compacted_before_index,
                            state.summary
                        ),
                        NotificationSeverity::Info,
                    ),
                )
            });
        self.seed_archival_transcript(ArchivalReplay {
            session_id: self.id,
            cursor: None,
            finished: false,
            chunks: VecDeque::new(),
            original: None,
            compaction,
            marker: None,
        })
    }
}

fn same_record(first: &ArchiveFragment, fragment: &ArchiveFragment) -> bool {
    first.turn_index == fragment.turn_index
        && first.message_id == fragment.message_id
        && first.timestamp == fragment.timestamp
        && first.record_kind == fragment.record_kind
        && first.total_bytes == fragment.total_bytes
}

impl ArchivalReplay {
    fn load_record(&mut self) -> Result<(), TranscriptError> {
        let mut first: Option<ArchiveFragment> = None;
        let mut encoded = String::new();
        loop {
            let page = get_session_archival_page(self.session_id, self.cursor.take(), 61_440)
                .map_err(|error| TranscriptError::JournalUnavailable(error.to_string()))?;
            self.cursor = page.next_cursor;
            self.finished = self.cursor.is_none();
            if let Some((_, marker)) = self.compaction.take_if(|(boundary, _)| {
                page.fragment
                    .as_ref()
                    .is_none_or(|fragment| fragment.turn_index >= *boundary)
            }) {
                self.marker = Some(marker);
            }
            let Some(mut fragment) = page.fragment else {
                if first.is_some() {
                    return Err(TranscriptError::InvalidPayload(
                        "incomplete original".into(),
                    ));
                }
                return Ok(());
            };
            if fragment.byte_offset != encoded.len()
                || first
                    .as_ref()
                    .is_some_and(|first| !same_record(first, &fragment))
            {
                return Err(TranscriptError::InvalidPayload(
                    "original identity changed".into(),
                ));
            }
            encoded.push_str(&fragment.json_fragment);
            let total_bytes = fragment.total_bytes;
            if first.is_none() {
                fragment.json_fragment.clear();
                first = Some(fragment);
            }
            if encoded.len() > total_bytes || (self.finished && encoded.len() != total_bytes) {
                return Err(TranscriptError::InvalidPayload(
                    "incomplete original".into(),
                ));
            }
            if encoded.len() == total_bytes {
                let original = first.take().ok_or_else(|| {
                    TranscriptError::InvalidPayload("missing original identity".into())
                })?;
                if original.record_kind != ArchiveRecordKind::ProviderState {
                    self.chunks.extend(
                        Replay::parse(std::slice::from_ref(&encoded))
                            .map_err(|error| TranscriptError::InvalidPayload(error.to_string()))?
                            .chunks,
                    );
                    self.original = Some(TranscriptOriginal {
                        turn_index: original.turn_index,
                        message_id: original.message_id.to_string(),
                        timestamp: original.timestamp.to_rfc3339(),
                        chunk_index: 0,
                    });
                }
                return Ok(());
            }
        }
    }
}

impl Iterator for ArchivalReplay {
    type Item = Result<(StreamChunk, Option<TranscriptOriginal>), TranscriptError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(marker) = self.marker.take() {
                return Some(Ok((marker, None)));
            }
            if let Some(chunk) = self.chunks.pop_front() {
                let original = self.original.clone();
                if let Some(original) = self.original.as_mut() {
                    original.chunk_index += 1;
                }
                return Some(Ok((chunk, original)));
            }
            if self.finished {
                if let Some((_, marker)) = self.compaction.take() {
                    return Some(Ok((marker, None)));
                }
                return None;
            }
            if let Err(error) = self.load_record() {
                self.finished = true;
                return Some(Err(error));
            }
        }
    }
}
