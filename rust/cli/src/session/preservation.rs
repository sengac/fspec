use super::Session;
use codelet_core::compaction::{
    include_file_identities, persisted_file_identities, DagValidationContext,
};
use codelet_core::persistence::compaction_boundary::CompactionSource;
use codelet_core::persistence::{get_message, SessionManifest};
use rig::message::Message;

pub(super) fn retained_reminders(
    session: &Session,
    source: &CompactionSource,
) -> Result<Vec<Message>, String> {
    let mut authoritative = Vec::new();
    for identity in &source.prefix {
        let original = get_message(*identity)?.ok_or("missing authoritative original")?;
        if let Some(message) = super::provider::conversational_message(
            &serde_json::to_value(&original.metadata).map_err(|error| error.to_string())?,
        )? {
            authoritative.push(message);
        }
    }
    authoritative.extend(session.messages.iter().cloned());
    Ok(super::system_reminders::partition_for_compaction(&authoritative).0)
}

pub(super) fn validation_context(
    session: &Session,
    manifest: &SessionManifest,
    source: &CompactionSource,
) -> Result<DagValidationContext, String> {
    // CMPCT-085: the builder validates against the same persisted identities.
    let mut required_files = persisted_file_identities(&source.prefix)?;
    let mut indices: Vec<_> = session.annotations.keys().collect();
    indices.sort_unstable();
    for index in indices {
        include_file_identities(&mut required_files, &session.annotations[index]);
    }
    Ok(DagValidationContext {
        persisted_turn_count: source.prefix.len(),
        required_coverage_end: source.prefix.len().checked_sub(1),
        existing_dag: manifest
            .compaction
            .as_ref()
            .map(|state| state.summary.clone()),
        required_files,
        required_reminders: Vec::new(),
    })
}
