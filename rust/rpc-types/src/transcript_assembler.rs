use crate::{StreamChunk, TranscriptCursor, TranscriptError, TranscriptOriginal, TranscriptPage};

#[derive(Debug, Default)]
pub struct TranscriptAssembler {
    cursor: Option<TranscriptCursor>,
    pending: String,
    total_bytes: usize,
    pending_original: Option<TranscriptOriginal>,
    original: Option<TranscriptOriginal>,
}

impl TranscriptAssembler {
    pub fn from_cursor(cursor: TranscriptCursor) -> Result<Self, TranscriptError> {
        if cursor.byte_offset != 0 {
            return Err(TranscriptError::InvalidCursor);
        }
        Ok(Self {
            cursor: Some(cursor),
            ..Self::default()
        })
    }

    pub fn cursor(&self) -> Option<&TranscriptCursor> {
        self.cursor.as_ref()
    }

    pub fn original(&self) -> Option<&TranscriptOriginal> {
        self.original.as_ref()
    }

    pub fn accept(
        &mut self,
        page: &TranscriptPage,
    ) -> Result<Option<StreamChunk>, TranscriptError> {
        if page.generation != page.next_cursor.generation {
            return Err(TranscriptError::GenerationMismatch);
        }
        let initial = TranscriptCursor {
            generation: page.generation.clone(),
            sequence: 0,
            byte_offset: 0,
        };
        let current = self.cursor.as_ref().unwrap_or(&initial);
        if current.generation != page.generation {
            return Err(TranscriptError::GenerationMismatch);
        }
        let next = &page.next_cursor;
        if next.sequence > page.snapshot_end
            || (next.sequence == page.snapshot_end && next.byte_offset != 0)
            || page.complete != (next.sequence == page.snapshot_end)
        {
            return Err(TranscriptError::InvalidSnapshot);
        }
        if (next.sequence, next.byte_offset) < (current.sequence, current.byte_offset) {
            return Ok(None);
        }
        let Some(fragment) = &page.fragment else {
            if !page.complete || next != current || !self.pending.is_empty() {
                return Err(TranscriptError::InvalidCursor);
            }
            self.cursor = Some(next.clone());
            return Ok(None);
        };
        if next == current {
            return Ok(None);
        }
        if fragment.sequence != current.sequence
            || fragment.byte_offset != current.byte_offset
            || fragment.byte_offset != self.pending.len()
            || fragment.json_fragment.is_empty()
            || (fragment.byte_offset != 0 && fragment.total_bytes != self.total_bytes)
            || (fragment.byte_offset != 0 && fragment.original != self.pending_original)
        {
            return Err(TranscriptError::InvalidCursor);
        }
        let end = fragment
            .byte_offset
            .checked_add(fragment.json_fragment.len())
            .filter(|end| *end <= fragment.total_bytes)
            .ok_or(TranscriptError::InvalidCursor)?;
        let finished = end == fragment.total_bytes;
        if (next.sequence, next.byte_offset)
            != if finished {
                (
                    fragment
                        .sequence
                        .checked_add(1)
                        .ok_or(TranscriptError::InvalidCursor)?,
                    0,
                )
            } else {
                (fragment.sequence, end)
            }
        {
            return Err(TranscriptError::InvalidCursor);
        }
        self.pending.push_str(&fragment.json_fragment);
        let result = if finished {
            match serde_json::from_str(&self.pending) {
                Ok(chunk) => Some(chunk),
                Err(error) => {
                    self.pending.truncate(fragment.byte_offset);
                    return Err(TranscriptError::InvalidPayload(error.to_string()));
                }
            }
        } else {
            None
        };
        self.total_bytes = fragment.total_bytes;
        self.pending_original = fragment.original.clone();
        if finished {
            self.pending = String::new();
            self.total_bytes = 0;
            self.original = self.pending_original.take();
        }
        self.cursor = Some(next.clone());
        Ok(result)
    }
}
