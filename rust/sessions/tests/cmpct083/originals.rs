use super::fixture::{self, TestResult, SENTINEL};
use codelet_core::persistence::*;
use codelet_tools::session_search::{
    originals::{OriginalSource, OriginalsRequest},
    types::{SessionSearchAction, SessionSearchResult},
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

pub fn append(manifest: &mut SessionManifest) -> TestResult {
    for (role, text) in [
        ("user", SENTINEL),
        ("assistant", "retained source response"),
    ] {
        let envelope = json!({
            "uuid":uuid::Uuid::new_v4(), "type":role,
            "message":{"role":role,"content":text},
            "provider_extension":{"evidence":["保持🦀",{"exact":true}]}
        });
        append_message_with_metadata(manifest, role, text, serde_json::from_value(envelope)?)?;
    }
    Ok(())
}

pub fn retrieve(root: &Path, manifest: &SessionManifest) -> TestResult<Value> {
    let before = fixture::manifest_bytes(root, manifest.id)?;
    let originals = get_session_messages_full(manifest)?;
    let first = originals.first().ok_or("original reference missing")?;
    let handler = codelet_agent_loop::session_search_handler::create_handler(
        root.to_owned(),
        Arc::new(AtomicBool::new(true)),
    );
    let result = handler(
        SessionSearchAction::Originals {
            request: OriginalsRequest {
                session_id: manifest.id.to_string(),
                start_turn: Some(0),
                end_turn: Some(0),
                next_turn: None,
                limit: Some(1),
                max_bytes: Some(61_440),
            },
        },
        manifest.id,
    );
    let page = match result {
        SessionSearchResult::Originals { page } => page,
        SessionSearchResult::OriginalsError { error } => return Err(error.into()),
        _ => return Err("original retrieval returned a preview instead of originals".into()),
    };
    assert!(matches!(page.source, OriginalSource::Original));
    assert_eq!(page.session_id, manifest.id);
    assert!(!page.truncated);
    assert!(page.next_turn.is_none());
    assert_eq!(page.records.len(), 1);
    let record = &page.records[0];
    assert_eq!(record.turn_index, 0);
    assert_eq!(record.message_id, first.id);
    assert_eq!(record.envelope, serde_json::to_value(&first.metadata)?);
    assert_eq!(record.envelope["message"]["role"], "user");
    assert_eq!(record.envelope["message"]["content"], SENTINEL);
    assert_eq!(
        before,
        fixture::manifest_bytes(root, manifest.id)?,
        "retrieval committed or mutated durable state"
    );
    assert_eq!(
        serde_json::to_value(&originals)?,
        serde_json::to_value(get_session_messages_full(&load_session(manifest.id)?)?)?
    );
    Ok(record.envelope["message"]["content"].clone())
}
