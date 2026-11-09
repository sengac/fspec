//! CMPCT-085: authoritative file identities for DAG validation.
//!
//! Shared by the compactor builder (captured evidence) and the durable commit
//! so both validate candidates against the same file awareness contract.

use super::{FileOp, StructuralAnnotation};
use crate::persistence::get_message;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Add every file modification in `annotations` to `files`, later entries
/// replacing earlier ones for the same path.
pub fn include_file_identities(
    files: &mut BTreeMap<String, String>,
    annotations: &[StructuralAnnotation],
) {
    for annotation in annotations {
        if let StructuralAnnotation::FileModification { path, operation } = annotation {
            let operation = match operation {
                FileOp::Created => "Created",
                FileOp::Modified => "Modified",
                FileOp::Deleted => "Deleted",
            };
            files.insert(path.clone(), operation.to_string());
        }
    }
}

/// File identities recorded on the persisted originals of a captured prefix.
pub fn persisted_file_identities(prefix: &[Uuid]) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    for identity in prefix {
        let message = get_message(*identity)?.ok_or("missing authoritative original")?;
        if let Some(value) = message.metadata.get("annotations") {
            let annotations: Vec<StructuralAnnotation> = serde_json::from_value(value.clone())
                .map_err(|error| format!("invalid authoritative annotations: {error}"))?;
            include_file_identities(&mut files, &annotations);
        }
    }
    Ok(files)
}
