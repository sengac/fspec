mod journal;
mod record;
mod source;
mod types;
use super::{compute_hash, load_session, SessionManifest};
pub use types::{ArchiveCursor, ArchiveError, ArchiveFragment, ArchivePage, ArchiveRecordKind};
use uuid::Uuid;

fn snapshot(
    manifest: &SessionManifest,
    cursor: Option<ArchiveCursor>,
) -> Result<(ArchiveCursor, Vec<journal::Entry>), ArchiveError> {
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.version != 1 || cursor.session_id != manifest.id)
    {
        return Err(ArchiveError::InvalidCursor("version or session".into()));
    }
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.end_turn > manifest.messages.len())
    {
        return Err(ArchiveError::SourceChanged);
    }
    let journal = journal::snapshot(manifest, cursor.as_ref())?;
    let cursor = match cursor {
        Some(cursor) => cursor,
        None => ArchiveCursor {
            version: 1,
            session_id: manifest.id,
            revision: manifest.revision,
            end_turn: manifest.messages.len(),
            prefix_hash: source::hash(manifest, manifest.messages.len())?,
            journal_end: journal.end,
            journal_hash: journal.hash,
            turn_index: 0,
            byte_offset: 0,
            record_hash: None,
        },
    };
    if cursor.turn_index > journal.entries.len()
        || (cursor.turn_index == journal.entries.len() && cursor.byte_offset != 0)
    {
        return Err(ArchiveError::InvalidCursor(
            "version, session or position".into(),
        ));
    }
    if cursor.revision > manifest.revision
        || cursor.prefix_hash != source::hash(manifest, cursor.end_turn)?
    {
        return Err(ArchiveError::SourceChanged);
    }
    Ok((cursor, journal.entries))
}

pub fn get_session_archival_page(
    session_id: Uuid,
    cursor: Option<ArchiveCursor>,
    max_bytes: usize,
) -> Result<ArchivePage, ArchiveError> {
    if !(1024..=61440).contains(&max_bytes) {
        return Err(ArchiveError::InvalidRequest(
            "page byte budget must be 1024..61440".into(),
        ));
    }
    let manifest =
        load_session(session_id).map_err(|message| ArchiveError::SourceUnavailable { message })?;
    let (cursor, entries) = snapshot(&manifest, cursor)?;
    let page = ArchivePage {
        session_id,
        source_revision: cursor.revision,
        end_turn: cursor.end_turn,
        fragment: None,
        next_cursor: None,
    };
    if cursor.turn_index == entries.len() {
        return Ok(page);
    }
    let (turn_index, message_id, timestamp, kind, encoded) = match &entries[cursor.turn_index] {
        journal::Entry::Original(index) => {
            let message_id = manifest.messages[*index].message_id;
            let (stored, kind, encoded) = record::read(message_id)?;
            (*index, message_id, stored.created_at, kind, encoded)
        }
        journal::Entry::Transcript { boundary, record } => (
            *boundary,
            record.id,
            record.timestamp,
            ArchiveRecordKind::Transcript,
            serde_json::to_string(&serde_json::json!({"_transcript":record}))?,
        ),
    };
    let record_hash = compute_hash(encoded.as_bytes());
    if cursor
        .record_hash
        .as_ref()
        .is_some_and(|identity| identity != &record_hash)
    {
        return Err(ArchiveError::SourceChanged);
    }
    if cursor.byte_offset >= encoded.len() || !encoded.is_char_boundary(cursor.byte_offset) {
        return Err(ArchiveError::InvalidCursor("record byte offset".into()));
    }
    let fragment = ArchiveFragment {
        turn_index,
        message_id,
        timestamp,
        record_kind: kind,
        byte_offset: cursor.byte_offset,
        total_bytes: encoded.len(),
        json_fragment: String::new(),
    };
    let candidate = |end| {
        let mut candidate = page.clone();
        let mut fragment = fragment.clone();
        fragment.json_fragment = encoded[cursor.byte_offset..end].into();
        candidate.fragment = Some(fragment);
        candidate.next_cursor = if end < encoded.len() {
            Some(ArchiveCursor {
                byte_offset: end,
                record_hash: Some(record_hash.clone()),
                ..cursor.clone()
            })
        } else if cursor.turn_index + 1 < entries.len() {
            Some(ArchiveCursor {
                turn_index: cursor.turn_index + 1,
                byte_offset: 0,
                record_hash: None,
                ..cursor.clone()
            })
        } else {
            None
        };
        candidate
    };
    let complete = candidate(encoded.len());
    if serde_json::to_vec(&complete)?.len() <= max_bytes {
        return Ok(complete);
    }
    let mut lower = cursor.byte_offset + 1;
    let mut upper = encoded.len();
    let mut selected = None;
    while lower <= upper {
        let middle = lower + (upper - lower) / 2;
        let mut end = middle;
        while !encoded.is_char_boundary(end) {
            end -= 1;
        }
        if end <= cursor.byte_offset {
            lower = middle + 1;
            continue;
        }
        let fragment_page = candidate(end);
        if serde_json::to_vec(&fragment_page)?.len() <= max_bytes {
            selected = Some(fragment_page);
            lower = middle + 1;
        } else {
            upper = middle - 1;
        }
    }
    selected.ok_or(ArchiveError::OversizedRecord {
        message_id,
        max_bytes,
    })
}
