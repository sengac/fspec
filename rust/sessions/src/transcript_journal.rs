use crate::transcript_archive::TranscriptArchive;
use codelet_rpc_types::{
    StreamChunk, TranscriptCursor, TranscriptError, TranscriptFragment, TranscriptPage,
};

pub(crate) struct TranscriptJournal {
    generation: String,
    chunks: Vec<StreamChunk>,
    encoded: Vec<String>,
    seeded: bool,
    failure: Option<String>,
    archive: Option<TranscriptArchive>,
    archive_failure: Option<String>,
}

impl Default for TranscriptJournal {
    fn default() -> Self {
        Self {
            generation: uuid::Uuid::new_v4().to_string(),
            chunks: Vec::new(),
            encoded: Vec::new(),
            seeded: false,
            failure: None,
            archive: None,
            archive_failure: None,
        }
    }
}

impl TranscriptJournal {
    pub(crate) fn generation(&self) -> &str {
        &self.generation
    }

    pub(crate) fn is_seeded(&self) -> bool {
        self.seeded
    }

    pub(crate) fn resident_chunks(&self) -> usize {
        self.chunks.len()
    }

    pub(crate) fn record_archive_failure(&mut self, generation: &str, error: &TranscriptError) {
        if self.generation == generation && !self.seeded {
            self.archive_failure = Some(error.to_string());
        }
    }

    pub(crate) fn install_archive(
        &mut self,
        archive: TranscriptArchive,
        expected_generation: &str,
    ) -> Result<bool, TranscriptError> {
        if self.generation != expected_generation {
            return Err(TranscriptError::GenerationMismatch);
        }
        if self.seeded {
            return Ok(false);
        }
        self.generation = uuid::Uuid::new_v4().to_string();
        self.archive = Some(archive);
        self.archive_failure = None;
        self.seeded = true;
        Ok(true)
    }

    pub(crate) fn buffered(&self, limit: usize) -> Result<Vec<StreamChunk>, TranscriptError> {
        let mut chunks = self
            .archive
            .as_ref()
            .map(|archive| archive.buffered(limit))
            .transpose()?
            .unwrap_or_default();
        chunks.extend(
            self.chunks
                .iter()
                .take(limit.saturating_sub(chunks.len()))
                .cloned(),
        );
        Ok(chunks)
    }

    pub(crate) fn completed_turns(&self) -> usize {
        self.archive
            .as_ref()
            .map_or(0, TranscriptArchive::completed_turns)
            + self
                .chunks
                .iter()
                .filter(|chunk| matches!(chunk, StreamChunk::Done))
                .count()
    }

    fn event_count(&self) -> usize {
        self.archive.as_ref().map_or(0, TranscriptArchive::len) + self.chunks.len()
    }

    fn fragment(
        &self,
        cursor: &TranscriptCursor,
        max_bytes: usize,
    ) -> Result<TranscriptFragment, TranscriptError> {
        let archive_len = self.archive.as_ref().map_or(0, TranscriptArchive::len);
        let sequence = cursor.sequence as usize;
        let (json_fragment, total_bytes, original) = if sequence < archive_len {
            self.archive
                .as_ref()
                .ok_or(TranscriptError::InvalidCursor)?
                .fragment(sequence, cursor.byte_offset, max_bytes)?
        } else {
            let encoded = self
                .encoded
                .get(sequence - archive_len)
                .ok_or(TranscriptError::InvalidCursor)?;
            if cursor.byte_offset >= encoded.len() || !encoded.is_char_boundary(cursor.byte_offset)
            {
                return Err(TranscriptError::InvalidCursor);
            }
            let mut end = encoded
                .len()
                .min(cursor.byte_offset.saturating_add(max_bytes));
            while !encoded.is_char_boundary(end) {
                end -= 1;
            }
            (
                encoded[cursor.byte_offset..end].to_owned(),
                encoded.len(),
                None,
            )
        };
        Ok(TranscriptFragment {
            sequence: cursor.sequence,
            byte_offset: cursor.byte_offset,
            total_bytes,
            json_fragment,
            original,
        })
    }

    pub(crate) fn push(&mut self, chunk: StreamChunk) {
        match serde_json::to_string(&chunk) {
            Ok(encoded) => self.encoded.push(encoded),
            Err(error) => self.failure = Some(error.to_string()),
        }
        self.chunks.push(chunk);
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn page(
        &self,
        cursor: Option<TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<TranscriptPage, TranscriptError> {
        if !(1024..=65_536).contains(&max_bytes) {
            return Err(TranscriptError::InvalidByteLimit);
        }
        if let Some(failure) = self.failure.as_ref().or(self.archive_failure.as_ref()) {
            return Err(TranscriptError::JournalUnavailable(failure.clone()));
        }
        if cursor.is_none() && snapshot_end.is_some() {
            return Err(TranscriptError::InvalidSnapshot);
        }
        let cursor = cursor.unwrap_or_else(|| TranscriptCursor {
            generation: self.generation.clone(),
            sequence: 0,
            byte_offset: 0,
        });
        if cursor.generation != self.generation {
            return Err(TranscriptError::GenerationMismatch);
        }
        let snapshot_end = snapshot_end.unwrap_or(self.event_count() as u64);
        if snapshot_end > self.event_count() as u64 {
            return Err(TranscriptError::InvalidSnapshot);
        }
        if cursor.sequence > snapshot_end {
            return Err(TranscriptError::InvalidCursor);
        }
        let mut page = TranscriptPage {
            generation: self.generation.clone(),
            snapshot_end,
            fragment: None,
            next_cursor: cursor.clone(),
            complete: cursor.sequence == snapshot_end,
        };
        if page.complete {
            if cursor.byte_offset != 0 {
                return Err(TranscriptError::InvalidCursor);
            }
            return Ok(page);
        }
        let source = self.fragment(&cursor, max_bytes as usize)?;
        let remaining = &source.json_fragment;
        let mut lower = 0;
        let mut upper = remaining.len().min(max_bytes as usize) + 1;
        while lower + 1 < upper {
            let middle = lower + (upper - lower) / 2;
            let mut end = middle;
            while !remaining.is_char_boundary(end) {
                end -= 1;
            }
            Self::set_fragment(&mut page, &cursor, &source, &remaining[..end]);
            let size = serde_json::to_vec(&page)
                .map_err(|error| TranscriptError::InvalidPayload(error.to_string()))?
                .len();
            if size <= max_bytes as usize {
                lower = middle;
            } else {
                upper = middle;
            }
        }
        while !remaining.is_char_boundary(lower) {
            lower -= 1;
        }
        if lower == 0 {
            return Err(TranscriptError::InvalidByteLimit);
        }
        Self::set_fragment(&mut page, &cursor, &source, &remaining[..lower]);
        Ok(page)
    }

    fn set_fragment(
        page: &mut TranscriptPage,
        cursor: &TranscriptCursor,
        source: &TranscriptFragment,
        text: &str,
    ) {
        let total_bytes = source.total_bytes;
        let end = cursor.byte_offset + text.len();
        page.next_cursor = TranscriptCursor {
            generation: cursor.generation.clone(),
            sequence: cursor.sequence + u64::from(end == total_bytes),
            byte_offset: if end == total_bytes { 0 } else { end },
        };
        page.complete = page.next_cursor.sequence == page.snapshot_end;
        page.fragment = Some(TranscriptFragment {
            sequence: cursor.sequence,
            byte_offset: cursor.byte_offset,
            total_bytes,
            json_fragment: text.to_owned(),
            original: source.original.clone(),
        });
    }
}
