use codelet_rpc_types::{StreamChunk, TranscriptError, TranscriptOriginal};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom};
use std::sync::Mutex;

struct Entry {
    offset: u64,
    bytes: usize,
    original: Option<TranscriptOriginal>,
}

pub(crate) struct TranscriptArchive {
    file: Mutex<File>,
    entries: Vec<Entry>,
    completed_turns: usize,
}

fn unavailable(error: impl std::fmt::Display) -> TranscriptError {
    TranscriptError::JournalUnavailable(error.to_string())
}

impl TranscriptArchive {
    pub(crate) fn build<I>(chunks: I) -> Result<Self, TranscriptError>
    where
        I: IntoIterator<Item = Result<(StreamChunk, Option<TranscriptOriginal>), TranscriptError>>,
    {
        let mut file = BufWriter::new(tempfile::tempfile().map_err(unavailable)?);
        let mut entries = Vec::new();
        let mut completed_turns = 0;
        for chunk in chunks {
            let (chunk, original) = chunk?;
            let offset = file.stream_position().map_err(unavailable)?;
            serde_json::to_writer(&mut file, &chunk).map_err(unavailable)?;
            let end = file.stream_position().map_err(unavailable)?;
            entries.push(Entry {
                offset,
                bytes: usize::try_from(end - offset).map_err(unavailable)?,
                original,
            });
            completed_turns += usize::from(matches!(chunk, StreamChunk::Done));
        }
        let file = file.into_inner().map_err(unavailable)?;
        Ok(Self {
            file: Mutex::new(file),
            entries,
            completed_turns,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn completed_turns(&self) -> usize {
        self.completed_turns
    }

    pub(crate) fn fragment(
        &self,
        sequence: usize,
        byte_offset: usize,
        max_bytes: usize,
    ) -> Result<(String, usize, Option<TranscriptOriginal>), TranscriptError> {
        let entry = self
            .entries
            .get(sequence)
            .ok_or(TranscriptError::InvalidCursor)?;
        if byte_offset >= entry.bytes {
            return Err(TranscriptError::InvalidCursor);
        }
        let mut file = self.file.lock().map_err(unavailable)?;
        file.seek(SeekFrom::Start(entry.offset + byte_offset as u64))
            .map_err(unavailable)?;
        let mut bytes = vec![0; (entry.bytes - byte_offset).min(max_bytes)];
        file.read_exact(&mut bytes).map_err(unavailable)?;
        let valid = match std::str::from_utf8(&bytes) {
            Ok(text) => text.len(),
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => return Err(TranscriptError::InvalidCursor),
        };
        bytes.truncate(valid);
        let text = String::from_utf8(bytes)
            .map_err(|error| TranscriptError::InvalidPayload(error.to_string()))?;
        Ok((text, entry.bytes, entry.original.clone()))
    }

    pub(crate) fn buffered(&self, limit: usize) -> Result<Vec<StreamChunk>, TranscriptError> {
        let mut file = self.file.lock().map_err(unavailable)?;
        self.entries
            .iter()
            .take(limit)
            .map(|entry| {
                file.seek(SeekFrom::Start(entry.offset))
                    .map_err(unavailable)?;
                serde_json::from_reader(BufReader::new((&mut *file).take(entry.bytes as u64)))
                    .map_err(unavailable)
            })
            .collect()
    }
}
