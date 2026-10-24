use super::{get_message, load_session};
use codelet_tools::session_search::originals::{
    OriginalPage, OriginalRecord, OriginalSource, OriginalsError, OriginalsRequest,
    MAX_ORIGINAL_BYTES, MAX_ORIGINAL_RECORDS,
};
use serde_json::Value;
use uuid::Uuid;

use super::original_blobs::restore_original_blobs;

pub fn retrieve_originals(request: OriginalsRequest) -> Result<OriginalPage, OriginalsError> {
    let invalid = |message: &str| OriginalsError::InvalidRequest {
        message: message.to_string(),
    };
    let session_id = Uuid::parse_str(&request.session_id)
        .map_err(|_| invalid("session_id must explicitly name a session UUID"))?;
    let limit = request.limit.unwrap_or(20);
    let max_bytes = request.max_bytes.unwrap_or(MAX_ORIGINAL_BYTES);
    let start = request.start_turn.unwrap_or(0);
    if !(1..=MAX_ORIGINAL_RECORDS).contains(&limit)
        || !(1..=MAX_ORIGINAL_BYTES).contains(&max_bytes)
        || request.end_turn.is_some_and(|end| end < start)
    {
        return Err(invalid(
            "invalid range, record limit (1..100), or byte limit (1..61440)",
        ));
    }
    let session = load_session(session_id)
        .map_err(|message| OriginalsError::SourceUnavailable { message })?;
    let upper = request
        .end_turn
        .map(|end| end.saturating_add(1))
        .unwrap_or(session.messages.len())
        .min(session.messages.len());
    let next = request.next_turn.unwrap_or(start);
    if start > session.messages.len() || next < start || next > upper {
        return Err(invalid(
            "continuation must stay within the selected persisted range",
        ));
    }
    let mut page = OriginalPage {
        session_id,
        source: OriginalSource::Original,
        truncated: false,
        records: Vec::new(),
        next_turn: None,
    };
    for turn_index in next..upper {
        if page.records.len() == limit {
            break;
        }
        let message_id = session.messages[turn_index].message_id;
        let stored = get_message(message_id)
            .map_err(|message| OriginalsError::SourceUnavailable { message })?
            .ok_or(OriginalsError::MissingMessage { message_id })?;
        let mut envelope =
            match super::provider_state::legacy::marker(&stored).map_err(|error| {
                OriginalsError::SourceUnavailable {
                    message: error.to_string(),
                }
            })? {
                Some(marker) => marker,
                None => {
                    if !stored.metadata.get("message").is_some_and(Value::is_object)
                        || !stored.metadata.get("uuid").is_some_and(Value::is_string)
                    {
                        return Err(OriginalsError::MissingEnvelope { message_id });
                    }
                    Value::Object(stored.metadata.into_iter().collect())
                }
            };
        restore_original_blobs(&mut envelope)?;
        page.records.push(OriginalRecord {
            turn_index,
            message_id,
            envelope,
        });
        page.next_turn = (turn_index + 1 < upper).then_some(turn_index + 1);
        if page.serialized_len()? > max_bytes {
            page.records.pop();
            if page.records.is_empty() {
                return Err(OriginalsError::OversizedEnvelope {
                    turn_index,
                    max_bytes,
                });
            }
            page.next_turn = Some(turn_index);
            break;
        }
    }
    if page.serialized_len()? > max_bytes {
        return Err(OriginalsError::OversizedEnvelope {
            turn_index: next,
            max_bytes,
        });
    }
    Ok(page)
}
