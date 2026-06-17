//! `delete-diagram` — Rust port of `src/commands/delete-diagram.ts` (RPC-216).
//!
//! Removes a Mermaid diagram entry from `spec/foundation.json`'s top-level
//! `architectureDiagrams` array. Match is by **title only** (Framing A) —
//! the supplied `section` argument is echoed in the response message but
//! ignored for the actual lookup. This mirrors the de-facto behaviour: the
//! TS `add-diagram` does NOT write a `section` field, so `deleteDiagram`
//! filtering on `d.section === section` always failed on diagrams created
//! by `addDiagram`. Matching by title only restores the intuitive contract.
//!
//! ## Framing A divergences
//!
//! * **Match key**: TS filters on `(section, title)`; Rust filters on
//!   `title` only and echoes the supplied section in the message.
//! * **FOUNDATION.md regeneration**: `generate-foundation-md` is unported,
//!   so the Rust core does NOT touch `spec/FOUNDATION.md`. The CLI bridge
//!   still prints `  Regenerated: spec/FOUNDATION.md` for stdout parity.
//! * **Auto-create**: foundation.json is NOT auto-created here (unlike
//!   `add-diagram`). Missing file → explicit error.
//!
//! ## Args (camelCase JSON)
//!
//! `{ "section": String, "title": String }` — both required.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DeleteDiagramArgs {
    section: String,
    title: String,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteDiagramArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-diagram",
            reason: format!("failed to parse args: {e}"),
        })?;

    let foundation_path = project_root.join("spec").join("foundation.json");

    // No auto-create — explicit missing-file error (parity with TS
    // delete-diagram.ts:30-35).
    if !foundation_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-diagram",
            reason: "foundation.json not found: spec/foundation.json".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&foundation_path).map_err(|source| FspecCoreError::Io {
        command: "delete-diagram",
        source,
    })?;
    let mut data: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "foundation.json".to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    let arr_present_and_has_entry = {
        let arr_opt = data.get("architectureDiagrams").and_then(|v| v.as_array());
        arr_opt
            .map(|arr| {
                arr.iter()
                    .any(|d| d.get("title").and_then(|t| t.as_str()) == Some(args.title.as_str()))
            })
            .unwrap_or(false)
    };

    if !arr_present_and_has_entry {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-diagram",
            reason: format!(
                "Diagram '{}' not found in section '{}'",
                args.title, args.section
            ),
        });
    }

    // Safe mutate: we already verified the entry exists.
    if let Some(arr) = data
        .get_mut("architectureDiagrams")
        .and_then(|v| v.as_array_mut())
    {
        if let Some(idx) = arr
            .iter()
            .position(|d| d.get("title").and_then(|t| t.as_str()) == Some(args.title.as_str()))
        {
            arr.remove(idx);
        }
    }

    write_json_atomic(&foundation_path, &data)?;

    let message = format!(
        "Deleted diagram '{}' from section '{}'",
        args.title, args.section
    );

    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "delete-diagram",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}
