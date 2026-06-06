//! `list-virtual-hooks` — Rust port of `src/commands/list-virtual-hooks.ts` (RPC-252).
//!
//! Loads `spec/work-units.json` via the shared `ensure_work_units_file`
//! helper (auto-creates an empty store on ENOENT, parity with the TS
//! `ensureWorkUnitsFile` helper), looks up the requested work unit by id,
//! reads its `virtualHooks` array (preserving insertion order), groups the
//! hooks by `event` into an `IndexMap<String, Vec<VirtualHook>>` so that
//! both event-introduction order AND within-event hook order survive
//! round-tripping, and finally renders either pretty-printed JSON (2-space
//! indent) or a documented text layout with `[blocking]` / `[non-blocking]`
//! / `[git-context]` badges.
//!
//! Behaviour parity with TypeScript
//! (`src/commands/list-virtual-hooks.ts:18-47`):
//!
//! * `data.workUnits[id]` miss → throw `Error("Work unit '<id>' does not exist")`.
//!   We mirror this by returning `FspecCoreError::InvalidArgs { reason }`
//!   so the dispatcher surfaces `success=false` with the reason in the
//!   `error` field.
//! * Missing `virtualHooks` field OR empty array → `{hooks: [], hooksByEvent: {}}`
//!   with `success=true`.
//! * Non-empty array → grouped, insertion-order-preserving `hooksByEvent`.
//!
//! The typed [`WorkUnit`] struct does NOT expose `virtualHooks` directly —
//! it is read out of `wu.extra["virtualHooks"]` (the `#[serde(flatten)]`
//! catch-all map) as a `serde_json::Value::Array`. This keeps the typed
//! work-unit shape minimal and shared across every ported command without
//! threading work-unit-scoped hook data through unrelated call sites.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

/// CLI arguments accepted by `list-virtual-hooks`. Field names mirror the
/// camelCase argument shape received over the dispatcher JSON tool-call
/// protocol. `work_unit_id` is REQUIRED; serde will surface a missing-field
/// error as `InvalidArgs` via the `failed to parse args` reason string.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListVirtualHooksArgs {
    /// Work unit ID to look up (e.g. `"AUTH-001"`). Required.
    work_unit_id: String,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// A single virtual hook entry. Mirrors the TS `VirtualHook` interface at
/// `src/types/index.ts:36-42`. `git_context` is optional and rendered with
/// the `gitContext` JSON key for parity with the TS source.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VirtualHook {
    name: String,
    event: String,
    command: String,
    blocking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "gitContext")]
    git_context: Option<bool>,
}

/// Response shape returned to the dispatcher. Mirrors the TS
/// `ListVirtualHooksResult` interface at
/// `src/commands/list-virtual-hooks.ts:13-16`. Field declaration order
/// (`hooks`, then `hooksByEvent`) is preserved on the wire so the JSON
/// pretty-print starts with `{\n  "hooks": [...],\n  "hooksByEvent": ...`.
#[derive(Debug, Serialize)]
struct ListVirtualHooksResult {
    hooks: Vec<VirtualHook>,
    #[serde(rename = "hooksByEvent")]
    hooks_by_event: IndexMap<String, Vec<VirtualHook>>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListVirtualHooksArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-virtual-hooks",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create) spec/work-units.json — parity with the TS command
    // which calls `ensureWorkUnitsFile(cwd)` unconditionally.
    let data = ensure_work_units_file(project_root)?;

    // Validate that the requested work unit exists. We mirror the exact TS
    // error string `Work unit '<id>' does not exist` (single-quoted id) so
    // that integration callers can match on the canonical substring.
    let work_unit = data
        .work_units
        .get(&args.work_unit_id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "list-virtual-hooks",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        })?;

    // Extract `virtualHooks` from the work-unit's `extra` flatten map. We
    // intentionally read through `wu.extra` rather than adding a typed
    // field to `WorkUnit` because every other ported command reuses the
    // shared `WorkUnit` shape; threading hook data through the canonical
    // type would force every consumer to know about a per-work-unit hook
    // concept that only this command cares about.
    let hooks: Vec<VirtualHook> = match work_unit.extra.get("virtualHooks") {
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| serde_json::from_value::<VirtualHook>(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-virtual-hooks",
                reason: format!("failed to deserialize virtualHooks entry: {e}"),
            })?,
        // Missing or any non-array value (the TS code uses `|| []` which
        // collapses `undefined`, `null`, and the empty array all to `[]`)
        // is treated as "no hooks configured".
        _ => Vec::new(),
    };

    // Group by event, preserving both event-introduction order and the
    // relative order of hooks within each event (parity with the TS
    // `for (const hook of hooks)` insertion loop).
    let mut hooks_by_event: IndexMap<String, Vec<VirtualHook>> = IndexMap::new();
    for hook in &hooks {
        hooks_by_event
            .entry(hook.event.clone())
            .or_default()
            .push(hook.clone());
    }

    let result = ListVirtualHooksResult {
        hooks,
        hooks_by_event,
    };

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "list-virtual-hooks",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text.
        _ => Ok(render_text(&args.work_unit_id, &result)),
    }
}

/// Render the text format documented in the `list-virtual-hooks` help
/// example. The empty case prints the exact sentinel
/// `"No virtual hooks configured for <id>"` (no trailing newline) — used
/// by both the missing-field path and the empty-array path. The populated
/// case prints a `Virtual Hooks for <id>:` header followed by event
/// sections; each hook line carries `[blocking]` or `[non-blocking]` and
/// an optional `[git-context]` badge.
///
/// Emits a leading `\n` before `Virtual Hooks for <id>:` to match the
/// TS implementation (`src/commands/list-virtual-hooks.ts:65`). Don't
/// strip the leading newline — downstream rendering expects it.
fn render_text(work_unit_id: &str, result: &ListVirtualHooksResult) -> String {
    if result.hooks.is_empty() {
        return format!("No virtual hooks configured for {work_unit_id}");
    }

    let mut out = format!("\nVirtual Hooks for {work_unit_id}:\n\n");

    for (event, hooks) in &result.hooks_by_event {
        out.push_str("  ");
        out.push_str(event);
        out.push_str(":\n");
        for hook in hooks {
            let blocking_badge = if hook.blocking {
                "[blocking]"
            } else {
                "[non-blocking]"
            };
            let git_context_badge = if hook.git_context.unwrap_or(false) {
                " [git-context]"
            } else {
                ""
            };
            out.push_str(&format!(
                "    • {name} {blocking_badge}{git_context_badge}\n",
                name = hook.name,
            ));
            out.push_str(&format!("      {}\n", hook.command));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_requires_work_unit_id() {
        let err = serde_json::from_str::<ListVirtualHooksArgs>("{}").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.to_lowercase().contains("workunitid") || msg.contains("workUnitId"),
            "expected missing-field error to mention workUnitId, got: {msg}"
        );
    }

    #[test]
    fn args_parse_camel_case() {
        let a: ListVirtualHooksArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","format":"json"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel_with_id() {
        let r = ListVirtualHooksResult {
            hooks: Vec::new(),
            hooks_by_event: IndexMap::new(),
        };
        assert_eq!(
            render_text("AUTH-001", &r),
            "No virtual hooks configured for AUTH-001"
        );
    }

    #[test]
    fn render_text_populated_uses_documented_layout() {
        let lint = VirtualHook {
            name: "lint".into(),
            event: "post-implementing".into(),
            command: "npm run lint".into(),
            blocking: true,
            git_context: None,
        };
        let test = VirtualHook {
            name: "test".into(),
            event: "post-implementing".into(),
            command: "npm test".into(),
            blocking: false,
            git_context: None,
        };
        let eslint = VirtualHook {
            name: "eslint".into(),
            event: "pre-validating".into(),
            command: "eslint .".into(),
            blocking: true,
            git_context: Some(true),
        };
        let mut by_event = IndexMap::new();
        by_event.insert(
            "post-implementing".to_string(),
            vec![lint.clone(), test.clone()],
        );
        by_event.insert("pre-validating".to_string(), vec![eslint.clone()]);
        let r = ListVirtualHooksResult {
            hooks: vec![lint, test, eslint],
            hooks_by_event: by_event,
        };
        let out = render_text("AUTH-001", &r);
        assert!(out.contains("Virtual Hooks for AUTH-001:"));
        assert!(out.contains("[blocking]"));
        assert!(out.contains("[non-blocking]"));
        assert!(out.contains("[git-context]"));
        // post-implementing precedes pre-validating
        let post = out.find("post-implementing:").unwrap();
        let pre = out.find("pre-validating:").unwrap();
        assert!(post < pre);
    }
}
