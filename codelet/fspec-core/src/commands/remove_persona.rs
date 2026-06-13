//! `remove-persona` — Rust port of `src/commands/remove-persona.ts`
//! (RPC-277).
//!
//! Removes the FIRST persona whose `name` exactly matches (case-sensitive) the
//! supplied argument from the top-level `personas` array of
//! `spec/foundation.json` (or `spec/foundation.json.draft` when present —
//! draft takes precedence).
//!
//! ## Semantics (parity with TS `removePersona`)
//!
//! 1. **Draft precedence**: prefer `spec/foundation.json.draft`, else
//!    `spec/foundation.json`. The resolved file name is echoed (`fileName`).
//! 2. **Missing file**: resolved target absent → canonical
//!    `"foundation.json not found"` error, create nothing.
//! 3. **Empty / missing personas**: error
//!    `Persona "<name>" not found` + `No personas exist in foundation`.
//! 4. **Name not present**: error `Persona "<name>" not found` +
//!    `Available personas: <comma-joined names>`.
//! 5. **Match**: remove the FIRST matching entry only (TS `findIndex` +
//!    `splice(index, 1)`).
//! 6. **Write**: 2-space indent + single trailing newline via
//!    [`write_json_atomic_trailing_newline`] (TS appends `'\n'`); unknown
//!    top-level fields preserved (full-document round-trip as `Value`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/remove_persona.rs` is JSON marshalling + stdout
//! rendering only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic_trailing_newline;

/// CLI arguments accepted by `remove-persona`. Mirrors the TS positional
/// `<name>`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemovePersonaArgs {
    name: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemovePersonaArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-persona",
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
            command: "remove-persona",
            reason: "foundation.json not found".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&target_path).map_err(|source| FspecCoreError::Io {
        command: "remove-persona",
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

    // Empty / missing personas → "no personas exist".
    let personas = match root_obj.get_mut("personas").and_then(Value::as_array_mut) {
        Some(a) if !a.is_empty() => a,
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-persona",
                reason: format!(
                    "Persona \"{}\" not found\n  No personas exist in foundation",
                    args.name
                ),
            });
        }
    };

    // Find first exact (case-sensitive) name match.
    let index = personas
        .iter()
        .position(|p| p.get("name").and_then(Value::as_str) == Some(args.name.as_str()));

    let index = match index {
        Some(i) => i,
        None => {
            let available: Vec<&str> = personas
                .iter()
                .filter_map(|p| p.get("name").and_then(Value::as_str))
                .collect();
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-persona",
                reason: format!(
                    "Persona \"{}\" not found\n  Available personas: {}",
                    args.name,
                    available.join(", ")
                ),
            });
        }
    };

    // Remove the first matching persona only.
    personas.remove(index);

    // Atomic write with trailing newline (TS appends `'\n'`).
    write_json_atomic_trailing_newline(&target_path, &foundation)?;

    let result = json!({
        "success": true,
        "fileName": file_name,
        "name": args.name,
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-persona",
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
    fn args_parse_name() {
        let a: RemovePersonaArgs = serde_json::from_str(r#"{"name":"Admin"}"#).unwrap();
        assert_eq!(a.name, "Admin");
    }

    #[test]
    fn args_parse_rejects_missing_name() {
        let r: Result<RemovePersonaArgs, _> = serde_json::from_str(r#"{}"#);
        assert!(r.is_err());
    }
}
