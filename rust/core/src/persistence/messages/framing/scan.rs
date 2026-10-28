use super::FramingError;
use crate::persistence::messages::index::IndexEntry;
use crate::persistence::messages::{compute_hash, StoredMessage};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use uuid::Uuid;

pub(crate) fn parse_record(bytes: &[u8], offset: u64) -> Result<StoredMessage, FramingError> {
    let message: StoredMessage =
        serde_json::from_slice(bytes).map_err(|error| FramingError::Corrupt {
            offset,
            reason: error.to_string(),
        })?;
    if !message.content_hash.is_empty()
        && message.content_hash != compute_hash(message.content.as_bytes())
    {
        return Err(FramingError::Corrupt {
            offset,
            reason: "content hash mismatch".into(),
        });
    }
    Ok(message)
}

pub(crate) fn scan_range(
    path: &Path,
    start: u64,
) -> Result<(HashMap<Uuid, IndexEntry>, u64), FramingError> {
    let mut reader = BufReader::new(File::open(path)?);
    reader.seek(SeekFrom::Start(start))?;
    let mut entries = HashMap::new();
    let mut offset = start;
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        if codelet_common::read_work::read_until(&mut reader, b'\n', &mut bytes)? == 0 {
            break;
        }
        if !bytes.iter().all(u8::is_ascii_whitespace) {
            let message = match parse_record(&bytes, offset) {
                Ok(message) => message,
                Err(_) if !bytes.ends_with(b"\n") => break,
                Err(error) => return Err(error),
            };
            let byte_length =
                u32::try_from(bytes.len()).map_err(|error| FramingError::Corrupt {
                    offset,
                    reason: error.to_string(),
                })?;
            entries.insert(
                message.id,
                IndexEntry {
                    byte_offset: offset,
                    byte_length,
                },
            );
        }
        offset += bytes.len() as u64;
    }
    Ok((entries, offset))
}
