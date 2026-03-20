//! Agent Lifecycle Hooks — JSON Response Parsing
//!
//! Claude Code compatible JSON response types and interpretation logic.
//! Handles permissionDecision, continue, decision, additionalContext fields.

use serde::Deserialize;

use super::outcome::{HookMessage, HookMessageLevel, PreToolHookDecision};
use super::executor::CommandResult;

/// Parsed hook stdout JSON response (Claude Code compatible).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HookJsonResponse {
    #[serde(rename = "continue")]
    pub continue_field: Option<bool>,
    pub decision: Option<String>,
    pub reason: Option<String>,
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Nested hookSpecificOutput from JSON response.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HookSpecificOutput {
    pub permission_decision: Option<String>,
    pub additional_context: Option<String>,
}

/// Try to parse hook stdout as a Claude Code compatible JSON response.
pub(crate) fn try_parse_json_response(stdout: &str) -> Option<HookJsonResponse> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// Interpret a pre_tool_use command result into a decision.
///
/// Priority order:
/// 1. hookSpecificOutput.permissionDecision → Allow/Deny/Ask
/// 2. continue: false → Deny
/// 3. decision: "deny"/"block" → Deny
/// 4. Exit code 2 + non-empty stderr → Deny
/// 5. Everything else → Continue (no opinion)
pub(crate) fn interpret_pre_tool_result(
    result: &CommandResult,
    messages: &mut Vec<HookMessage>,
) -> PreToolHookDecision {
    // Try JSON interpretation first
    if let Some(json_resp) = try_parse_json_response(&result.stdout) {
        // Priority 1: hookSpecificOutput.permissionDecision
        if let Some(ref hso) = json_resp.hook_specific_output {
            if let Some(ref pd) = hso.permission_decision {
                return match pd.to_lowercase().as_str() {
                    "allow" => PreToolHookDecision::Allow,
                    "deny" => PreToolHookDecision::Deny,
                    "ask" => PreToolHookDecision::Ask,
                    _ => PreToolHookDecision::Continue,
                };
            }
        }

        // Priority 2: continue: false
        if json_resp.continue_field == Some(false) {
            return PreToolHookDecision::Deny;
        }

        // Priority 3: decision: "deny"/"block"
        if let Some(ref d) = json_resp.decision {
            match d.to_lowercase().as_str() {
                "deny" | "block" => return PreToolHookDecision::Deny,
                "allow" => return PreToolHookDecision::Allow,
                _ => {}
            }
        }
    }

    // Priority 4: Exit code 2 + non-empty stderr → Deny
    if result.exit_code == Some(2) {
        let stderr = result.stderr.trim();
        if !stderr.is_empty() {
            return PreToolHookDecision::Deny;
        }
        // Exit code 2 + empty stderr → Warning, continue
        messages.push(HookMessage {
            level: HookMessageLevel::Warning,
            content: "Hook exited with code 2 but no stderr — treating as warning".to_string(),
        });
        return PreToolHookDecision::Continue;
    }

    // Non-zero exit (not 2) → Warning, continue
    if let Some(code) = result.exit_code {
        if code != 0 {
            messages.push(HookMessage {
                level: HookMessageLevel::Warning,
                content: format!(
                    "Hook exited with code {code} — treating as non-blocking warning"
                ),
            });
            return PreToolHookDecision::Continue;
        }
    }

    // Exit code 0 → Continue (no opinion)
    PreToolHookDecision::Continue
}

/// Extract a reason string from the command result (JSON reason > stderr).
pub(crate) fn extract_reason(result: &CommandResult) -> Option<String> {
    // Try JSON reason first
    if let Some(json_resp) = try_parse_json_response(&result.stdout) {
        if let Some(reason) = json_resp.reason {
            return Some(reason);
        }
    }

    // Fall back to stderr
    let stderr = result.stderr.trim();
    if !stderr.is_empty() {
        return Some(stderr.to_string());
    }

    None
}
