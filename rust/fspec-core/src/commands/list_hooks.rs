//! `list-hooks` — Rust port of `src/commands/list-hooks.ts` (RPC-247).
//!
//! Reads `spec/fspec-hooks.json` and returns a `{events, message?}` payload
//! suitable for the LLM dispatcher and the standalone fspec Rust binary's
//! clap subcommand. Both call sites converge on this single
//! `pub async fn run` so the agent-loop and the shell CLI share one
//! source-of-truth (RPC-003 §7/§11 two-front-doors invariant).
//!
//! Behaviour parity with TypeScript (`src/commands/list-hooks.ts:22-45`):
//!
//! * Successful read **AND** successful JSON parse →
//!   `{events: [{event, hooks: [name1, ...]}, ...]}` (NO `message` field).
//!   Empty `hooks: {}` is a valid happy-path: produces `events: []` with
//!   STILL no `message` field. This branch preserves the JavaScript object
//!   key insertion order (we model `hooks` as `IndexMap<String, Vec<_>>`).
//! * A missing file → `{events: [], message: "No hooks are configured"}`.
//! * A broken file (BUG-611: invalid JSON, or a top-level value, `hooks` or
//!   event value of the wrong shape) → an error naming the problem; the
//!   file is never changed.
//! * Hook entries missing the `name` field surface as JSON `null`
//!   (parity with the TS `hooks.map(h => h.name)` semantics — `name`
//!   is `undefined` in the source array, which JSON.stringify emits as
//!   `null`).
//!
//! list-hooks does NOT auto-create `spec/fspec-hooks.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::command_hooks::config_file::load_for_read;
use crate::error::FspecCoreError;

/// CLI arguments accepted by `list-hooks`. The TS Commander.js
/// registration (`src/commands/list-hooks.ts:47-54`) declares NO
/// `.option(...)` calls, so the shell-facing CLI bridge passes the empty
/// object. `format` is exposed for the structured dispatcher path so
/// `{"format":"json"}` produces the canonical 2-space-indented JSON
/// shape used by the agent loop's tool-call protocol, and `"text"`
/// (the default) renders the documented help-example layout.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListHooksArgs {
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// One event entry in the `events` array of the response shape. Field
/// declaration order (`event`, then `hooks`) is preserved on the wire
/// because we use `#[derive(Serialize)]` rather than routing through
/// `json!{}` (which would alphabetise via `serde_json::Map`'s default
/// BTreeMap backing).
#[derive(Debug, Serialize)]
struct EventEntry {
    event: String,
    hooks: Vec<Option<String>>,
}

/// Response shape returned to the dispatcher. Mirrors the TS
/// `ListHooksResult` interface at `src/commands/list-hooks.ts:14-20`.
///
/// `message` is `#[serde(skip_serializing_if = "Option::is_none")]` so
/// the happy path (`{hooks: {...}}` parsed successfully) omits the
/// field entirely, while the swallow-error path includes it. Field
/// declaration order (`events`, then `message`) is preserved on the
/// wire so the JSON pretty-print starts with `{\n  "events": [],\n` —
/// asserted by `scenario_json_format_two_space_indent_for_empty_case`.
#[derive(Debug, Serialize)]
struct ListHooksResult {
    events: Vec<EventEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListHooksArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-hooks",
            reason: format!("failed to parse args: {e}"),
        })?;

    let result = load_hook_config(project_root)?;

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-hooks",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text.
        _ => Ok(render_text(&result)),
    }
}

/// Read `spec/fspec-hooks.json` and convert it into a [`ListHooksResult`]:
/// a missing file is the canonical empty payload, a broken one an error
/// (BUG-611). An entry's `name` that is missing or not a string is `None`.
fn load_hook_config(project_root: &Path) -> Result<ListHooksResult, FspecCoreError> {
    let Some(config) = load_for_read(project_root, "list-hooks")? else {
        return Ok(empty_with_message());
    };
    let events = config
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(event, entries)| EventEntry {
            event: event.clone(),
            hooks: entries
                .as_array()
                .into_iter()
                .flatten()
                .map(|h| h.get("name").and_then(Value::as_str).map(str::to_string))
                .collect(),
        })
        .collect();
    Ok(ListHooksResult {
        events,
        message: None,
    })
}

/// Build the canonical empty-with-sentinel response for a missing file.
fn empty_with_message() -> ListHooksResult {
    ListHooksResult {
        events: Vec::new(),
        message: Some("No hooks are configured".to_string()),
    }
}

/// Render the text format documented in the `list-hooks` help-example.
///
/// Layout:
///
/// ```text
/// Configured Hooks:
///
/// <event-1>:
///   - <hook-1>
///   - <hook-2>
///
/// <event-2>:
///   - <hook-3>
/// ```
///
/// For the empty case (zero events) the function returns the exact
/// sentinel `"No hooks are configured"` (no trailing newline) — used by
/// both the missing-file path and the empty-`hooks: {}` happy path.
/// `scenario_text_format_empty_prints_no_hooks_sentinel` and
/// `scenario_default_format_is_text` assert this byte-for-byte.
///
/// Hook entries with a missing `name` (serialised as JSON `null` on the
/// structured path) render as `  - (unnamed)` in text — neither the
/// feature file nor the TS implementation document a canonical text
/// representation for this edge case, so we choose a stable, visually
/// distinct marker rather than rendering the literal string "null".
fn render_text(result: &ListHooksResult) -> String {
    if result.events.is_empty() {
        return "No hooks are configured".to_string();
    }

    let mut out = String::from("Configured Hooks:\n");
    out.push('\n');

    for entry in &result.events {
        out.push_str(&entry.event);
        out.push_str(":\n");
        for hook in &entry.hooks {
            match hook {
                Some(name) => {
                    out.push_str("  - ");
                    out.push_str(name);
                    out.push('\n');
                }
                None => out.push_str("  - (unnamed)\n"),
            }
        }
        out.push('\n');
    }

    out
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
    fn args_parse_with_defaults() {
        let a: ListHooksArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListHooksArgs = serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel() {
        let r = ListHooksResult {
            events: Vec::new(),
            message: Some("No hooks are configured".to_string()),
        };
        assert_eq!(render_text(&r), "No hooks are configured");
    }

    #[test]
    fn render_text_populated_uses_help_example_layout() {
        let r = ListHooksResult {
            events: vec![
                EventEntry {
                    event: "pre-implementing".to_string(),
                    hooks: vec![Some("lint".to_string())],
                },
                EventEntry {
                    event: "post-implementing".to_string(),
                    hooks: vec![Some("test".to_string()), Some("notify".to_string())],
                },
            ],
            message: None,
        };
        let out = render_text(&r);
        assert!(out.starts_with("Configured Hooks:\n"));
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.contains(&"pre-implementing:"));
        assert!(lines.contains(&"  - lint"));
        assert!(lines.contains(&"post-implementing:"));
        assert!(lines.contains(&"  - test"));
        assert!(lines.contains(&"  - notify"));
        // pre-implementing must precede post-implementing.
        let pre = out.find("pre-implementing:").unwrap();
        let post = out.find("post-implementing:").unwrap();
        assert!(pre < post);
    }

    #[test]
    fn empty_with_message_carries_canonical_string() {
        let r = empty_with_message();
        assert_eq!(r.events.len(), 0);
        assert_eq!(r.message.as_deref(), Some("No hooks are configured"));
    }

    #[test]
    fn missing_name_hook_renders_as_unnamed() {
        let r = ListHooksResult {
            events: vec![EventEntry {
                event: "pre-implementing".to_string(),
                hooks: vec![Some("lint".to_string()), None],
            }],
            message: None,
        };
        let out = render_text(&r);
        assert!(out.lines().any(|l| l == "  - lint"));
        assert!(out.lines().any(|l| l == "  - (unnamed)"));
    }
}
