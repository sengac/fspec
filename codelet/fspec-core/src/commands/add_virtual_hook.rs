//! `add-virtual-hook` — Rust port of `src/commands/add-virtual-hook.ts` (RPC-195).
//!
//! Attaches a work-unit-scoped virtual hook to `spec/work-units.json`. The
//! `virtualHooks` array lives on `workUnit.extra` (parity with the
//! `list-virtual-hooks` port, RPC-252).
//!
//! ## Semantics (mirrors src/commands/add-virtual-hook.ts:24-93)
//!
//! 1. Resolve work unit by id; missing → `InvalidArgs("Work unit 'X' does not exist")`.
//! 2. Initialise `virtualHooks` to `[]` if absent.
//! 3. Derive hook `name` from `command`: first whitespace-separated token,
//!    then the last `/`-separated segment. Fallback `"hook"` when empty.
//!    (`"npm run lint"` → `"npm"`; `"/usr/bin/node x.js"` → `"node"`.)
//! 4. If `gitContext=true`: write `spec/hooks/.virtual/<id>-<name>.sh` at
//!    mode 0o755, store the relative path as the hook command, set
//!    `gitContext: true` on the stored entry. Otherwise store the command
//!    verbatim and OMIT `gitContext` (TS only sets it when truthy).
//! 5. Append, bump `updatedAt`, single atomic write.
//!
//! ## Result shape (JSON)
//!
//! `{ "success": true, "hookCount": <new length> }` — camelCase matches TS.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

/// CLI arguments accepted by `add-virtual-hook`. Mirrors the TS
/// `AddVirtualHookOptions` shape at `src/commands/add-virtual-hook.ts:10-17`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AddVirtualHookArgs {
    work_unit_id: String,
    event: String,
    command: String,
    #[serde(default)]
    blocking: Option<bool>,
    #[serde(default)]
    git_context: Option<bool>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AddVirtualHookResult {
    success: bool,
    #[serde(rename = "hookCount")]
    hook_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddVirtualHookArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-virtual-hook",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Validate args::work_unit_id non-empty (mirror missing-field semantics
    // — `serde(default)` makes "" parse cleanly, but we want the same
    // "workUnitId" surface in the error).
    if args.work_unit_id.is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-virtual-hook",
            reason: "missing field `workUnitId`".to_string(),
        });
    }

    let mut data = ensure_work_units_file(project_root)?;

    // Source-exists pre-flight (TS add-virtual-hook.ts:33-36).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-virtual-hook",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Derive hook name from command (TS line 46:
    //   options.command.split(' ')[0].split('/').pop() || 'hook').
    let hook_name = derive_hook_name(&args.command);

    // Optionally generate a git-context script file. Done BEFORE the work-
    // units write so the on-disk state can never reference a missing script.
    let command_to_store = if args.git_context.unwrap_or(false) {
        let script_path = generate_virtual_hook_script(
            &args.work_unit_id,
            &hook_name,
            &args.command,
            project_root,
        )?;
        // Strip project root prefix to store a clean relative path
        // (TS: scriptPath.replace(cwd + '/', '')).
        relative_to_project_root(project_root, &script_path)
    } else {
        args.command.clone()
    };

    // Build the typed hook entry and serialise back into `extra`.
    let mut hook_obj = serde_json::Map::new();
    hook_obj.insert("name".to_string(), Value::String(hook_name));
    hook_obj.insert("event".to_string(), Value::String(args.event.clone()));
    hook_obj.insert("command".to_string(), Value::String(command_to_store));
    hook_obj.insert(
        "blocking".to_string(),
        Value::Bool(args.blocking.unwrap_or(false)),
    );
    if args.git_context.unwrap_or(false) {
        hook_obj.insert("gitContext".to_string(), Value::Bool(true));
    }

    // Append into wu.extra["virtualHooks"], initialising to [] when missing.
    // Presence was verified at the top of `run`; we return a structured
    // error rather than panic to satisfy clippy::expect_used.
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-virtual-hook",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            });
        }
    };
    let entry = wu
        .extra
        .entry("virtualHooks".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    // `entry` was just coerced to an array — unreachable otherwise.
    let new_count = match entry.as_array_mut() {
        Some(arr) => {
            arr.push(Value::Object(hook_obj));
            arr.len()
        }
        None => 0,
    };
    wu.updated_at = iso8601_now();

    // Single atomic write at end.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&AddVirtualHookResult {
        success: true,
        hook_count: new_count,
    })
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-virtual-hook",
        reason: format!("failed to serialize result: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Derive a virtual-hook name from the raw command.
///
/// Mirrors the TS one-liner at `add-virtual-hook.ts:46`:
/// ```js
/// const hookName = options.command.split(' ')[0].split('/').pop() || 'hook';
/// ```
///
/// Examples:
///   * `"npm run lint"`             → `"npm"`
///   * `"eslint src/"`              → `"eslint"`
///   * `"/usr/bin/node script.js"`  → `"node"`
///   * `""` / `"   "`               → `"hook"` (TS `|| 'hook'` fallback)
fn derive_hook_name(command: &str) -> String {
    let first_token = command.split(' ').next().unwrap_or("");
    let after_slash = first_token.rsplit('/').next().unwrap_or("");
    if after_slash.is_empty() {
        "hook".to_string()
    } else {
        after_slash.to_string()
    }
}

/// Generate the virtual-hook shell script under
/// `<project_root>/spec/hooks/.virtual/<workUnitId>-<hookName>.sh`. Mirrors
/// `src/hooks/script-generation.ts:45-105` and writes mode 0o755 on Unix.
fn generate_virtual_hook_script(
    work_unit_id: &str,
    hook_name: &str,
    command: &str,
    project_root: &Path,
) -> Result<std::path::PathBuf, FspecCoreError> {
    let virtual_dir = project_root.join("spec").join("hooks").join(".virtual");
    std::fs::create_dir_all(&virtual_dir).map_err(|e| FspecCoreError::Io {
        command: "add-virtual-hook",
        source: e,
    })?;
    let filename = format!("{work_unit_id}-{hook_name}.sh");
    let script_path = virtual_dir.join(&filename);

    // Always git-context-style content here — `generate_virtual_hook_script`
    // is only called when `args.git_context == Some(true)`. Mirrors the TS
    // `if (gitContext)` branch at `script-generation.ts:64-87`.
    let script_content = format!(
        "#!/bin/bash\n\
set -e\n\
\n\
# Read context JSON from stdin\n\
CONTEXT=$(cat)\n\
\n\
# Extract staged and unstaged files from context\n\
STAGED_FILES=$(echo \"$CONTEXT\" | jq -r '.stagedFiles[]? // empty' 2>/dev/null | tr '\\n' ' ')\n\
UNSTAGED_FILES=$(echo \"$CONTEXT\" | jq -r '.unstagedFiles[]? // empty' 2>/dev/null | tr '\\n' ' ')\n\
\n\
# Combine all changed files\n\
ALL_FILES=\"$STAGED_FILES $UNSTAGED_FILES\"\n\
\n\
# Exit if no files to process\n\
if [ -z \"$ALL_FILES\" ]; then\n  \
  echo \"No changed files to process\"\n  \
  exit 0\n\
fi\n\
\n\
# Run command with changed files\n\
{command} $ALL_FILES\n"
    );

    std::fs::write(&script_path, script_content).map_err(|e| FspecCoreError::Io {
        command: "add-virtual-hook",
        source: e,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&script_path, perms).map_err(|e| FspecCoreError::Io {
            command: "add-virtual-hook",
            source: e,
        })?;
    }

    Ok(script_path)
}

/// Strip the project root prefix from `full_path` to yield a forward-slash
/// relative path. Mirrors TS `scriptPath.replace(cwd + '/', '')`.
fn relative_to_project_root(project_root: &Path, full_path: &Path) -> String {
    if let Ok(rel) = full_path.strip_prefix(project_root) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        full_path.to_string_lossy().replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn derive_name_npm_run_lint_becomes_npm() {
        assert_eq!(derive_hook_name("npm run lint"), "npm");
    }

    #[test]
    fn derive_name_eslint_with_path_arg_becomes_eslint() {
        assert_eq!(derive_hook_name("eslint src/"), "eslint");
    }

    #[test]
    fn derive_name_absolute_path_keeps_basename() {
        assert_eq!(derive_hook_name("/usr/bin/node script.js"), "node");
    }

    #[test]
    fn derive_name_empty_command_falls_back_to_hook() {
        assert_eq!(derive_hook_name(""), "hook");
    }

    #[test]
    fn derive_name_trailing_slash_falls_back_to_hook() {
        // "/usr/" → first token "/usr/" → rsplit '/' → ""  → "hook"
        assert_eq!(derive_hook_name("/usr/"), "hook");
    }
}
