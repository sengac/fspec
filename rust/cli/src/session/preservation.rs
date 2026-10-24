use super::Session;
use codelet_core::compaction::{DagValidationContext, StructuralAnnotation};
use codelet_core::persistence::compaction_boundary::CompactionSource;
use codelet_core::persistence::{get_message, SessionManifest};
use rig::message::Message;
use std::collections::BTreeMap;

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
    let mut required_files = BTreeMap::new();
    for identity in &source.prefix {
        let message = get_message(*identity)?.ok_or("missing authoritative original")?;
        if let Some(value) = message.metadata.get("annotations") {
            let annotations: Vec<StructuralAnnotation> = serde_json::from_value(value.clone())
                .map_err(|error| format!("invalid authoritative annotations: {error}"))?;
            include_files(&mut required_files, &annotations);
        }
    }
    let mut indices: Vec<_> = session.annotations.keys().collect();
    indices.sort_unstable();
    for index in indices {
        include_files(&mut required_files, &session.annotations[index]);
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

fn include_files(files: &mut BTreeMap<String, String>, annotations: &[StructuralAnnotation]) {
    for annotation in annotations {
        if let StructuralAnnotation::FileModification { path, operation } = annotation {
            files.insert(path.clone(), format!("{operation:?}"));
        }
    }
}
