use super::super::transcript::{JournalProjection, JournalRecord, JournalStore};
use super::super::SessionManifest;
use super::{ArchiveCursor, ArchiveError};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub(super) enum Entry {
    Original(usize),
    Transcript {
        boundary: usize,
        record: Box<JournalRecord>,
    },
}

pub(super) struct Snapshot {
    pub end: usize,
    pub hash: String,
    pub entries: Vec<Entry>,
}

pub(super) fn snapshot(
    manifest: &SessionManifest,
    cursor: Option<&ArchiveCursor>,
) -> Result<Snapshot, ArchiveError> {
    let unavailable =
        |error: super::super::transcript::JournalError| ArchiveError::SourceUnavailable {
            message: error.to_string(),
        };
    let mut records = JournalStore::configured(manifest.id)
        .map_err(unavailable)?
        .read()
        .map_err(unavailable)?;
    let end = cursor.map_or(records.len(), |cursor| cursor.journal_end);
    if end > records.len() {
        return Err(ArchiveError::SourceChanged);
    }
    records.truncate(end);
    let mut digest = Sha256::new();
    digest.update(manifest.id.as_bytes());
    for record in &records {
        let encoded = serde_json::to_vec(record)?;
        digest.update((encoded.len() as u64).to_le_bytes());
        digest.update(encoded);
    }
    let hash = hex::encode(digest.finalize());
    if cursor.is_some_and(|cursor| cursor.journal_hash != hash) {
        return Err(ArchiveError::SourceChanged);
    }
    let end_turn = cursor.map_or(manifest.messages.len(), |cursor| cursor.end_turn);
    let originals: HashMap<_, _> = manifest.messages[..end_turn]
        .iter()
        .enumerate()
        .map(|(index, reference)| (reference.message_id, index + 1))
        .collect();
    let mut boundaries = HashMap::new();
    let mut prefixes = HashMap::new();
    for record in &records {
        if let Some(anchor) = &record.request_anchor {
            let preceding = anchor
                .original_count
                .checked_sub(1)
                .and_then(|index| manifest.messages.get(index))
                .map(|reference| reference.message_id);
            if anchor.original_count > end_turn || preceding != anchor.preceding_message_id {
                return Err(ArchiveError::SourceUnavailable {
                    message: "transcript request boundary does not match its original prefix"
                        .into(),
                });
            }
            prefixes.insert(record.identity.request_id, anchor.original_count);
            boundaries.insert(record.identity.request_id, anchor.original_count);
        }
    }
    for record in &records {
        if let Some(boundary) = originals.get(&record.identity.message_id) {
            if prefixes
                .get(&record.identity.request_id)
                .is_some_and(|prefix| boundary <= prefix)
            {
                return Err(ArchiveError::SourceUnavailable {
                    message: "transcript message precedes its request boundary".into(),
                });
            }
            boundaries
                .entry(record.identity.request_id)
                .and_modify(|current: &mut usize| *current = (*current).max(*boundary))
                .or_insert(*boundary);
        }
    }
    if let Some(last) = records.last() {
        boundaries
            .entry(last.identity.request_id)
            .or_insert(end_turn);
    }
    let projected = JournalProjection::from_records(records, &originals.keys().copied().collect())
        .map_err(unavailable)?
        .records;
    if projected
        .iter()
        .any(|record| !boundaries.contains_key(&record.identity.request_id))
    {
        return Err(ArchiveError::SourceUnavailable {
            message: "interior transcript request has no original causal anchor".into(),
        });
    }
    let mut pending = projected.into_iter().peekable();
    let mut entries = Vec::new();
    for boundary in 0..=end_turn {
        while pending
            .peek()
            .is_some_and(|record| boundaries.get(&record.identity.request_id) == Some(&boundary))
        {
            let record = pending.next().ok_or(ArchiveError::SourceChanged)?;
            entries.push(Entry::Transcript {
                boundary,
                record: Box::new(record),
            });
        }
        if boundary < end_turn {
            entries.push(Entry::Original(boundary));
        }
    }
    if pending.next().is_some() {
        return Err(ArchiveError::SourceUnavailable {
            message: "transcript causal order disagrees with original message order".into(),
        });
    }
    Ok(Snapshot { end, hash, entries })
}
