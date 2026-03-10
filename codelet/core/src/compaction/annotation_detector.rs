//! Per-turn structural annotation detection.
//!
//! Zero-cost inline detection of structural milestones from tool call metadata.
//! Called after each completed turn in the stream loop to attach annotations
//! to persisted turn metadata for SessionSearch navigation during DAG construction.
//!
//! Detection is purely synchronous pattern matching — no LLM calls, no external
//! processes, no network requests.
//!
//! Research: ACON (Kang et al., KAIST/Microsoft, arXiv:2510.00615) demonstrates
//! that preserving structural signals (WHY decisions were made) is critical for
//! continuation quality after compression.

use serde_json::Value;

use super::model::{FileOp, StructuralAnnotation};

/// Information about a single tool call within a turn.
///
/// Extracted from the stream loop's tool call tracking — the detector
/// operates on this lightweight struct rather than full rig messages.
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    /// Tool name (e.g., "Fspec", "Bash", "Write", "Edit", "Read", "Grep")
    pub tool_name: String,
    /// Parsed JSON input arguments
    pub input: Value,
    /// Tool output text (if available)
    pub output: Option<String>,
    /// Whether the tool call succeeded
    pub success: bool,
}

/// Context for annotation detection — current turn + optional previous turn.
///
/// ErrorResolution detection requires the previous turn to identify
/// failure → success transitions.
#[derive(Debug)]
pub struct TurnContext<'a> {
    /// Tool calls from the current (just-completed) turn
    pub current_tool_calls: &'a [ToolCallInfo],
    /// Tool calls from the previous turn (for error→success detection)
    pub previous_tool_calls: Option<&'a [ToolCallInfo]>,
}

/// Detect structural annotations from a completed turn's tool calls.
///
/// Returns a Vec of annotations to attach to the turn's persisted metadata.
/// Detection is pure pattern matching — zero LLM cost.
///
/// Detects:
/// - `FspecMilestone`: Fspec tool calls with meaningful commands
/// - `ErrorResolution`: Previous failure + current file edit + success
/// - `FileModification`: Write/Edit tool calls with file paths
pub fn detect_annotations(ctx: &TurnContext<'_>) -> Vec<StructuralAnnotation> {
    let mut annotations = Vec::new();

    detect_fspec_milestones(ctx.current_tool_calls, &mut annotations);
    detect_file_modifications(ctx.current_tool_calls, &mut annotations);
    detect_error_resolutions(ctx, &mut annotations);

    annotations
}

/// Detect FspecMilestone annotations from Fspec tool calls.
///
/// Extracts the `command` and `args._` fields from the Fspec tool input JSON.
fn detect_fspec_milestones(
    tool_calls: &[ToolCallInfo],
    annotations: &mut Vec<StructuralAnnotation>,
) {
    for tc in tool_calls {
        if tc.tool_name != "Fspec" || !tc.success {
            continue;
        }

        let command = match tc.input.get("command").and_then(|v| v.as_str()) {
            Some(cmd) => cmd.to_string(),
            None => continue,
        };

        // Extract args._ array if present, parsing the args JSON string
        let args = extract_fspec_args(&tc.input);

        annotations.push(StructuralAnnotation::FspecMilestone { command, args });
    }
}

/// Extract positional args from Fspec tool input.
///
/// The Fspec tool accepts `args` as a JSON string containing `{"_": [...]}`.
fn extract_fspec_args(input: &Value) -> Vec<String> {
    input
        .get("args")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .and_then(|parsed| parsed.get("_").and_then(|v| v.as_array()).cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Detect FileModification annotations from Write/Edit tool calls.
///
/// - Write tool: `Created` operation (new file)
/// - Edit tool: `Modified` operation (existing file changed)
fn detect_file_modifications(
    tool_calls: &[ToolCallInfo],
    annotations: &mut Vec<StructuralAnnotation>,
) {
    for tc in tool_calls {
        if !tc.success {
            continue;
        }

        let operation = match tc.tool_name.as_str() {
            "Write" => FileOp::Created,
            "Edit" => FileOp::Modified,
            _ => continue,
        };

        if let Some(path) = tc.input.get("file_path").and_then(|v| v.as_str()) {
            annotations.push(StructuralAnnotation::FileModification {
                path: path.to_string(),
                operation,
            });
        }
    }
}

/// Detect ErrorResolution annotations from failure→success transitions.
///
/// Conditions:
/// 1. Previous turn had at least one failed tool call
/// 2. Current turn has at least one Edit/Write (the fix)
/// 3. Current turn has no failed tool calls (all succeeded)
fn detect_error_resolutions(ctx: &TurnContext<'_>, annotations: &mut Vec<StructuralAnnotation>) {
    let previous = match ctx.previous_tool_calls {
        Some(prev) => prev,
        None => return,
    };

    // Find failed tools from previous turn
    let failed_tools: Vec<&ToolCallInfo> = previous.iter().filter(|tc| !tc.success).collect();
    if failed_tools.is_empty() {
        return;
    }

    // Check all current tools succeeded
    let all_current_succeeded = ctx.current_tool_calls.iter().all(|tc| tc.success);
    if !all_current_succeeded {
        return;
    }

    // Find file modifications in current turn (the fix)
    let modified_files: Vec<String> = ctx
        .current_tool_calls
        .iter()
        .filter(|tc| tc.success && (tc.tool_name == "Edit" || tc.tool_name == "Write"))
        .filter_map(|tc| {
            tc.input
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
        .collect();

    if modified_files.is_empty() {
        return;
    }

    // Create ErrorResolution for each failed tool + first modified file
    // Use the first modified file as the resolved_file (most common pattern:
    // one file is edited to fix the error)
    let resolved_file = match modified_files.first() {
        Some(f) => f.clone(),
        None => return, // unreachable due to is_empty() guard, but safe
    };

    for failed in &failed_tools {
        annotations.push(StructuralAnnotation::ErrorResolution {
            failed_tool: failed.tool_name.clone(),
            resolved_file: resolved_file.clone(),
        });
    }
}
