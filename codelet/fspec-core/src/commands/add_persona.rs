//! `add-persona` — Rust port of `src/commands/add-persona.ts` (RPC-186).
//!
//! Appends a persona `{ name, description, goals }` to the top-level
//! `personas` array of `spec/foundation.json` (or `spec/foundation.json.draft`
//! when the draft is present — draft takes precedence).
//!
//! ## Semantics (parity with TS `addPersona`)
//!
//! 1. **Draft precedence**: if `spec/foundation.json.draft` exists, the write
//!    targets the draft; otherwise `spec/foundation.json`. The resolved file
//!    name is echoed back to the caller (`fileName`).
//! 2. **Missing file**: if the resolved target does NOT exist, return the
//!    canonical `"foundation.json not found"` error WITHOUT creating any file
//!    (TS catches `ENOENT` and rethrows). Note this deliberately does NOT use
//!    `ensure_foundation_file` — the TS command never auto-creates.
//! 3. **personas init**: a missing `personas` key is initialised to `[]`.
//! 4. **Placeholder clearing**: if the personas array is non-empty AND every
//!    entry is a placeholder (any of name/description/goal contains
//!    `[QUESTION:` or `[DETECTED:`), the entire array is cleared first and the
//!    cleared count is reported (`removedPlaceholders`). A single real persona
//!    suppresses the clear.
//! 5. **Append**: push `{ name, description, goals }` (goals defaults to `[]`).
//! 6. **Write**: serialise with 2-space indent + a single trailing newline via
//!    [`write_json_atomic_trailing_newline`] (TS writes
//!    `JSON.stringify(foundation, null, 2) + '\n'`). Unknown top-level fields
//!    are preserved byte-for-byte (full-document round-trip as `Value`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/add_persona.rs` is JSON marshalling + stdout rendering
//! only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic_trailing_newline;

/// CLI arguments accepted by `add-persona`. Mirrors the TS positional
/// `<name> <description>` + repeatable `--goal` option.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddPersonaArgs {
    name: String,
    description: String,
    #[serde(default)]
    goals: Vec<String>,
}

/// Returns true when `text` contains a placeholder marker.
fn is_placeholder_text(text: &str) -> bool {
    text.contains("[QUESTION:") || text.contains("[DETECTED:")
}

/// Mirror of the TS `isPlaceholderPersona` helper: a persona is a placeholder
/// when its name, description, or ANY goal contains a placeholder marker.
fn is_placeholder_persona(persona: &Value) -> bool {
    let name_hit = persona
        .get("name")
        .and_then(Value::as_str)
        .map(is_placeholder_text)
        .unwrap_or(false);
    let desc_hit = persona
        .get("description")
        .and_then(Value::as_str)
        .map(is_placeholder_text)
        .unwrap_or(false);
    let goals_hit = persona
        .get("goals")
        .and_then(Value::as_array)
        .map(|gs| gs.iter().filter_map(Value::as_str).any(is_placeholder_text))
        .unwrap_or(false);
    name_hit || desc_hit || goals_hit
}

/// Mirror of the TS `hasOnlyPlaceholders`: false for an empty array, true when
/// every entry is a placeholder.
fn has_only_placeholders(personas: &[Value]) -> bool {
    if personas.is_empty() {
        return false;
    }
    personas.iter().all(is_placeholder_persona)
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddPersonaArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-persona",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Draft precedence: prefer foundation.json.draft when present.
    let spec_dir = project_root.join("spec");
    let draft_path = spec_dir.join("foundation.json.draft");
    let final_path = spec_dir.join("foundation.json");

    let (target_path, file_name) = if draft_path.exists() {
        (draft_path, "foundation.json.draft")
    } else {
        (final_path, "foundation.json")
    };

    // Missing target → canonical not-found error, create nothing.
    if !target_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-persona",
            reason: "foundation.json not found".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&target_path).map_err(|source| FspecCoreError::Io {
        command: "add-persona",
        source,
    })?;
    let mut foundation: Value =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: file_name.to_string(),
            reason: e.to_string(),
        })?;

    let root_obj = match foundation.as_object_mut() {
        Some(o) => o,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: file_name.to_string(),
                reason: "top-level value must be a JSON object".to_string(),
            });
        }
    };

    // Ensure personas exists and is an array.
    let entry = root_obj
        .entry("personas".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    let personas = match entry.as_array_mut() {
        Some(a) => a,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: file_name.to_string(),
                reason: "personas must be an array".to_string(),
            });
        }
    };

    // Clear an all-placeholder array before appending the real persona.
    let mut removed_count: u64 = 0;
    if has_only_placeholders(personas) {
        removed_count = personas.len() as u64;
        personas.clear();
    }

    // Append the new persona in TS key order: name, description, goals.
    let mut persona_obj: Map<String, Value> = Map::new();
    persona_obj.insert("name".to_string(), Value::String(args.name.clone()));
    persona_obj.insert(
        "description".to_string(),
        Value::String(args.description.clone()),
    );
    persona_obj.insert(
        "goals".to_string(),
        Value::Array(args.goals.iter().cloned().map(Value::String).collect()),
    );
    personas.push(Value::Object(persona_obj));

    // Atomic write with trailing newline (TS appends `'\n'`).
    write_json_atomic_trailing_newline(&target_path, &foundation)?;

    let result = json!({
        "success": true,
        "fileName": file_name,
        "removedPlaceholders": removed_count,
        "name": args.name,
        "description": args.description,
        "goals": args.goals,
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-persona",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_minimal_no_goals() {
        let a: AddPersonaArgs =
            serde_json::from_str(r#"{"name":"Dev","description":"Builds"}"#).unwrap();
        assert_eq!(a.name, "Dev");
        assert_eq!(a.description, "Builds");
        assert!(a.goals.is_empty());
    }

    #[test]
    fn args_parse_with_goals() {
        let a: AddPersonaArgs = serde_json::from_str(
            r#"{"name":"Founder","description":"Runs co","goals":["Ship fast","Stay safe"]}"#,
        )
        .unwrap();
        assert_eq!(
            a.goals,
            vec!["Ship fast".to_string(), "Stay safe".to_string()]
        );
    }

    #[test]
    fn placeholder_detection_question_and_detected() {
        assert!(is_placeholder_persona(
            &json!({"name":"[QUESTION: Who?]","description":"d","goals":[]})
        ));
        assert!(is_placeholder_persona(
            &json!({"name":"x","description":"[DETECTED: Admin]","goals":[]})
        ));
        assert!(is_placeholder_persona(
            &json!({"name":"x","description":"d","goals":["[QUESTION: g?]"]})
        ));
        assert!(!is_placeholder_persona(
            &json!({"name":"Real","description":"d","goals":["g"]})
        ));
    }

    #[test]
    fn has_only_placeholders_empty_is_false() {
        assert!(!has_only_placeholders(&[]));
    }

    #[test]
    fn has_only_placeholders_mixed_is_false() {
        let arr = vec![
            json!({"name":"Real","description":"d","goals":[]}),
            json!({"name":"[DETECTED: Admin]","description":"d","goals":[]}),
        ];
        assert!(!has_only_placeholders(&arr));
    }
}
