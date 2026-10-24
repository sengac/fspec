use super::{get_blob, make_blob_reference};
use codelet_tools::session_search::originals::OriginalsError;
use serde_json::Value;
use std::collections::HashSet;

fn invalid(message: impl Into<String>) -> OriginalsError {
    OriginalsError::InvalidBlob {
        message: message.into(),
    }
}

fn blob_text(value: &Value, hash: &str) -> Result<String, OriginalsError> {
    if value.as_str() != Some(make_blob_reference(hash).as_str()) {
        return Err(invalid(
            "recorded blob hash does not match the stored marker",
        ));
    }
    let bytes = get_blob(hash).map_err(invalid)?;
    String::from_utf8(bytes).map_err(|error| invalid(error.to_string()))
}

pub(super) fn restore_original_blobs(envelope: &mut Value) -> Result<(), OriginalsError> {
    let Some(references) = envelope.get("_blobRefs") else {
        return Ok(());
    };
    let references = references
        .as_array()
        .ok_or_else(|| invalid("recorded blob references must be an array"))?
        .clone();
    if references.is_empty() {
        return Ok(());
    }
    let blocks = envelope
        .pointer_mut("/message/content")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("recorded blob references require content blocks"))?;
    let mut restored_indices = HashSet::new();
    for reference in references {
        let key = reference
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("recorded blob reference requires a string key"))?;
        let hash = reference
            .get("hash")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("recorded blob reference requires a string hash"))?;
        let mut parts = key.splitn(3, ':');
        let kind = parts.next().ok_or_else(|| invalid("missing blob kind"))?;
        let index = parts
            .next()
            .and_then(|index| index.parse::<usize>().ok())
            .ok_or_else(|| invalid("recorded blob reference requires a content index"))?;
        if !restored_indices.insert(index) {
            return Err(invalid(
                "duplicate recorded blob reference for content block",
            ));
        }
        let block = blocks
            .get_mut(index)
            .filter(|block| block.get("type").and_then(Value::as_str) == Some(kind))
            .ok_or_else(|| invalid("recorded blob reference does not match a content block"))?;
        let identifier = parts.next();
        let path = match kind {
            "tool_result" | "tool_use" => {
                let id_field = if kind == "tool_result" {
                    "tool_use_id"
                } else {
                    "id"
                };
                let actual_id = block.get(id_field).and_then(Value::as_str);
                if identifier.is_none() || identifier != actual_id {
                    return Err(invalid(
                        "recorded blob reference does not match the tool ID",
                    ));
                }
                if kind == "tool_result" {
                    "/content"
                } else {
                    "/input"
                }
            }
            "thinking" if identifier.is_none() => "/thinking",
            "image" | "document"
                if identifier.is_none()
                    && block.pointer("/source/type").and_then(Value::as_str) == Some("base64") =>
            {
                "/source/data"
            }
            _ => return Err(invalid("unsupported recorded blob reference")),
        };
        let target = block
            .pointer_mut(path)
            .ok_or_else(|| invalid("recorded blob content is missing"))?;
        if kind == "tool_use" {
            let marker = target
                .as_object()
                .filter(|input| input.len() == 1)
                .and_then(|input| input.get("_blob_ref"))
                .ok_or_else(|| invalid("recorded tool input must contain only a blob marker"))?;
            let text = blob_text(marker, hash)?;
            *target = serde_json::from_str(&text).map_err(|error| invalid(error.to_string()))?;
        } else {
            *target = Value::String(blob_text(target, hash)?);
        }
    }
    Ok(())
}
