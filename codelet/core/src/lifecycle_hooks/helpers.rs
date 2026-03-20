//! Agent Lifecycle Hooks — Shared Helpers
//!
//! Common utility functions used by both the session engine
//! and the tool engine.

use super::compiled::CompiledHookDefinition;
use super::executor::CommandResult;
use super::outcome::{HookMessage, HookMessageLevel};
use super::response::try_parse_json_response;

/// Collect warning messages from non-zero exit codes.
///
/// When `hook_name` is provided, the message includes the hook name.
/// When `None`, a generic "Hook command" label is used (for unnamed tool-hook commands).
pub(crate) fn collect_exit_code_messages(
    result: &CommandResult,
    hook_name: Option<&str>,
    messages: &mut Vec<HookMessage>,
) {
    if let Some(code) = result.exit_code {
        if code != 0 {
            let label = match hook_name {
                Some(name) => format!("Hook '{name}'"),
                None => "Hook command".to_string(),
            };
            messages.push(HookMessage {
                level: HookMessageLevel::Warning,
                content: format!("{label} exited with code {code}"),
            });
        }
    }
}

/// Collect additional context from non-tool hook stdout (JSON or plain text).
pub(crate) fn collect_non_tool_context(
    result: &CommandResult,
    hook_def: &CompiledHookDefinition,
    messages: &mut Vec<HookMessage>,
    additional_context: &mut Vec<String>,
) {
    if let Some(json_resp) = try_parse_json_response(&result.stdout) {
        if let Some(ref hso) = json_resp.hook_specific_output {
            if let Some(ref ctx_str) = hso.additional_context {
                additional_context.push(ctx_str.clone());
            }
        }
    } else {
        let stdout_trimmed = result.stdout.trim();
        if !stdout_trimmed.is_empty() {
            additional_context.push(stdout_trimmed.to_string());
        }
    }

    collect_exit_code_messages(result, Some(&hook_def.name), messages);
}
