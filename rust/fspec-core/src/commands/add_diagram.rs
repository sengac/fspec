//! `add-diagram` — Rust port of `src/commands/add-diagram.ts` (RPC-178).
//!
//! Adds OR updates a Mermaid diagram entry in `spec/foundation.json`'s
//! top-level `architectureDiagrams` array. Each entry uses the generic
//! schema v2.0.0 shape: `{ title, mermaidCode, [description] }`.
//!
//! ## Divergences from TypeScript
//!
//! * **Mermaid validation**: TS uses `mermaid.parse()` + jsdom. The Rust port
//!   now performs *real* validation via the `merman` Rust-native Mermaid
//!   parser (see [`crate::utils::mermaid_validation::validate_mermaid_syntax`]):
//!   pure-string pre-checks (quoted subgraph title, invalid subgraph
//!   identifier) run first for canonical error-message parity, then
//!   `merman_core::Engine::parse_diagram_sync` gates genuine syntax errors.
//! * **FOUNDATION.md regeneration**: `generate-foundation-md` (RPC-233) IS
//!   now ported, but this command does NOT itself regenerate FOUNDATION.md.
//!   The CLI bridge still prints `  Regenerated: spec/FOUNDATION.md` for
//!   stdout parity.
//! * **JSON schema validation**: TS calls `validateFoundationJson` (Ajv);
//!   Rust skips this — no Ajv-equivalent has been ported.
//!
//! ## Args (camelCase JSON)
//!
//! `{ "section": String, "title": String, "code": String, "description"?: String }`
//!
//! `section` is accepted for CLI argv shape parity (the TS Commander.js
//! command also takes it as `<section>`) but is NOT persisted — the
//! generic schema v2.0.0 entry has no `section` field.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_foundation_file;
use crate::io::locked_file::write_json_atomic;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AddDiagramArgs {
    section: String,
    title: String,
    code: String,
    #[serde(default)]
    description: Option<String>,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddDiagramArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-diagram",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Argument validation — mirrors add-diagram.ts:31-49.
    if args.section.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-diagram",
            reason: "Section name cannot be empty".to_string(),
        });
    }
    if args.title.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-diagram",
            reason: "Diagram title cannot be empty".to_string(),
        });
    }
    if args.code.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-diagram",
            reason: "Diagram code cannot be empty".to_string(),
        });
    }

    // Real Mermaid validation via merman (parser-only). Pre-checks for the
    // two canonical TS patterns (quoted subgraph title, invalid subgraph id)
    // run first inside the shared validator, then merman gates real syntax.
    if let Err(reason) = crate::utils::mermaid_validation::validate_mermaid_syntax(&args.code) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-diagram",
            reason,
        });
    }

    // Load-or-init foundation.json (auto-creates the canonical generic
    // schema v2.0.0 default when missing).
    let mut data: Value = ensure_foundation_file(project_root)?;

    let root_obj = match data.as_object_mut() {
        Some(o) => o,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: "foundation.json".to_string(),
                reason: "top-level value must be a JSON object".to_string(),
            });
        }
    };

    // Ensure architectureDiagrams exists and is an array. The two lines
    // above coerce a non-array shape into an empty array, so `as_array_mut`
    // is logically infallible — but we still return a structured error
    // rather than panic to satisfy `clippy::expect_used`.
    let entry = root_obj
        .entry("architectureDiagrams".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    let arr = match entry.as_array_mut() {
        Some(a) => a,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: "foundation.json".to_string(),
                reason: "architectureDiagrams must be an array".to_string(),
            });
        }
    };

    // Build the new entry (generic schema — no `section` field).
    let mut new_entry: Map<String, Value> = Map::new();
    new_entry.insert("title".to_string(), Value::String(args.title.clone()));
    new_entry.insert("mermaidCode".to_string(), Value::String(args.code.clone()));
    if let Some(desc) = &args.description {
        new_entry.insert("description".to_string(), Value::String(desc.clone()));
    }
    let new_entry = Value::Object(new_entry);

    // Replace existing entry with same title, or append.
    let existing_idx = arr
        .iter()
        .position(|d| d.get("title").and_then(|t| t.as_str()) == Some(args.title.as_str()));

    let message = match existing_idx {
        Some(i) => {
            arr[i] = new_entry;
            format!("Updated diagram \"{}\"", args.title)
        }
        None => {
            arr.push(new_entry);
            format!("Added diagram \"{}\"", args.title)
        }
    };

    // Atomic write — preserves all unknown top-level fields verbatim.
    let path = project_root.join("spec").join("foundation.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "add-diagram",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}
