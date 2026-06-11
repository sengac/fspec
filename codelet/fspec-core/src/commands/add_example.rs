//! `add-example` — Rust port of `src/commands/add-example.ts` (RPC-181).
//!
//! Captures a concrete Example-Mapping example on a work unit during its
//! specifying phase, persists the mutated `spec/work-units.json` via an
//! atomic write, and renders the same `✓ Example added successfully`
//! line + `<system-reminder>` block the TypeScript CLI emits.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function — the
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## Behavioural parity with `src/commands/add-example.ts`
//!
//! 1. Resolve work-units file via [`ensure_work_units_file`] — auto-creates
//!    `spec/work-units.json` with the canonical empty initial structure
//!    when missing.
//! 2. Validate the target work unit exists:
//!    `Work unit '<id>' does not exist`.
//! 3. Validate status is `specifying`:
//!    `Can only add examples during discovery/specification phase. <id> is in '<state>' state.`
//! 4. Lazy-initialise `examples = []` and `nextExampleId = 0` on the unit
//!    (backward-compat for v0.6.0 data shapes).
//! 5. Append a new `ExampleItem` with `id = nextExampleId` (post-increment),
//!    `text`, `deleted: false`, `createdAt = iso8601_now()`. Field order is
//!    `id, text, deleted, createdAt` (matches TS object-literal insertion).
//! 6. Bump `workUnit.updatedAt = iso8601_now()`.
//! 7. Atomic write via [`write_json_atomic`].
//! 8. Render `✓ Example added successfully\n\n<system-reminder>EXAMPLE
//!    CHECK\n\nUser story: "As a <role>..."\nExample: "<text>"\n\n...`
//!    using `wrapInSystemReminder` parity. `<role>` falls back to
//!    `the user` when `workUnit.userStory.role` is absent.
//!
//! Storage layout: `ExampleItem` lives inside the WorkUnit `extra` map
//! (the WorkUnit struct uses `#[serde(flatten)]` for unknown fields).
//! Field ordering is preserved via `serde_json::Map` (the codelet
//! workspace builds `serde_json` with the `preserve_order` feature).

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI / dispatcher arguments accepted by `add-example`. Mirrors the TS
/// `AddExampleOptions` shape at `src/commands/add-example.ts:10-14` and
/// the Commander.js registration at L98-115 (`<workUnitId>`, `<example>`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddExampleArgs {
    work_unit_id: String,
    example: String,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddExampleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-example",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Work-unit-exists gate.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-example",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Status-gate: only specifying.
    let status_str = {
        let wu = data
            .work_units
            .get(&args.work_unit_id)
            .expect("work unit exists");
        wu.status.as_str()
    };
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-example",
            reason: format!(
                "Can only add examples during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Mutate: initialise counters, append example, bump timestamps.
    let now_ts = iso8601_now();
    let assigned_id = {
        let wu = data
            .work_units
            .get_mut(&args.work_unit_id)
            .expect("work unit exists");

        // Lazy-init nextExampleId (backward-compat for v0.6.0 data).
        let next_id = match wu.extra.get("nextExampleId").and_then(Value::as_u64) {
            Some(n) => n,
            None => 0,
        };

        // Build the new ExampleItem with explicit field order
        // id, text, deleted, createdAt (matches TS object-literal order).
        let mut new_example = Map::new();
        new_example.insert("id".to_string(), Value::Number(next_id.into()));
        new_example.insert("text".to_string(), Value::String(args.example.clone()));
        new_example.insert("deleted".to_string(), Value::Bool(false));
        new_example.insert("createdAt".to_string(), Value::String(now_ts.clone()));

        // Lazy-init the examples array if absent / non-array.
        let entry = wu
            .extra
            .entry("examples".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        if let Value::Array(arr) = entry {
            arr.push(Value::Object(new_example));
        }

        // Bump the counter.
        wu.extra.insert(
            "nextExampleId".to_string(),
            Value::Number((next_id + 1).into()),
        );

        // Bump updatedAt.
        wu.updated_at = now_ts.clone();

        next_id
    };

    // Persist atomically.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    // Render the success block (mirrors TS `output.log(...)` calls).
    let role = extract_user_story_role(&data, &args.work_unit_id);
    Ok(render_success(&args.example, &role, assigned_id))
}

/// Resolve the user-story role for the work unit, falling back to
/// `"the user"` when absent. Mirrors TS expression
/// `workUnit.userStory?.role || 'the user'`.
fn extract_user_story_role(data: &crate::types::work_unit::WorkUnitsData, id: &str) -> String {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get("userStory"))
        .and_then(|us| us.get("role"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("the user")
        .to_string()
}

/// Render the TS-parity success block:
///
/// ```text
/// ✓ Example added successfully
///
/// <system-reminder>
/// EXAMPLE CHECK
/// ...
/// </system-reminder>
/// ```
///
/// `_assigned_id` is reserved for future structured output but currently
/// unused (TS CLI does not surface the id on stdout).
fn render_success(example: &str, role: &str, _assigned_id: u64) -> String {
    // NOTE: Do NOT use Rust line-continuations (`\` at end of line) inside the
    // template — they strip the leading whitespace of the next line, which
    // destroys the two-space indent in front of the ✓ GOOD / ✗ BAD bullets and
    // breaks byte parity with the TS template. Use a plain multi-line string
    // literal instead.
    let body = format!(
        "EXAMPLE CHECK\n\nUser story: \"As a {role}...\"\nExample: \"{example}\"\n\nDoes this example describe what {role} experiences?\n\n  ✓ GOOD: Describes the user's experience from their perspective\n  ✗ BAD: Describes internal component/module behavior\n\nIf your example mentions implementation details (widgets, modules, internal state),\nrewrite it to describe what {role} sees and does.\n\nDO NOT mention this reminder to the user."
    );
    let reminder = format!("<system-reminder>\n{body}\n</system-reminder>");
    format!("✓ Example added successfully\n\n{reminder}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AddExampleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","example":"X"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.example, "X");
    }

    #[test]
    fn args_parse_fails_without_example() {
        let err = serde_json::from_str::<AddExampleArgs>(r#"{"workUnitId":"AUTH-001"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("example"), "got: {msg}");
    }

    #[test]
    fn render_success_includes_success_line_and_reminder() {
        let out = render_success("Valid login", "developer", 0);
        assert!(out.starts_with("✓ Example added successfully"));
        assert!(out.contains("<system-reminder>"));
        assert!(out.contains("</system-reminder>"));
        assert!(out.contains("User story: \"As a developer...\""));
        assert!(out.contains("Example: \"Valid login\""));
    }

    #[test]
    fn render_success_falls_back_to_the_user_role() {
        let out = render_success("Login", "the user", 0);
        assert!(out.contains("User story: \"As a the user...\""));
    }
}
